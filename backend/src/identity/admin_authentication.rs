//! Password + TOTP authentication service for the isolated Admin session store.

use std::{
    net::IpAddr,
    time::{SystemTime, UNIX_EPOCH},
};

use sqlx::PgPool;
use zeroize::Zeroizing;

use super::{
    authentication::{
        hash_session_token, ip_rate_limit_subject, normalize_identifier, random_session_token,
        session_token_hash, AuthenticationRateLimitKey,
    },
    crypto::{
        decrypt_admin_totp_secret, parse_key_version, EncryptedPayload, KeyProvider,
    },
    password::{hash_password, verify_password},
    totp::matching_step,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminAuthenticationError {
    Database,
    InvalidCredentials,
    InvalidChallenge,
    InvalidSession,
    RateLimited,
    InvalidInput,
    Internal,
}

impl std::fmt::Display for AdminAuthenticationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Database => "Admin authentication database operation failed",
            Self::InvalidCredentials => "Admin identifier or password is invalid",
            Self::InvalidChallenge => "Admin authentication challenge is invalid or expired",
            Self::InvalidSession => "Admin session is invalid or expired",
            Self::RateLimited => "too many Admin authentication attempts",
            Self::InvalidInput => "Admin authentication input is invalid",
            Self::Internal => "Admin authentication operation failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for AdminAuthenticationError {}

pub struct AdminLoginChallenge {
    pub token: Zeroizing<String>,
    pub expires_at_unix: i64,
}

pub struct AdminLoginSession {
    pub user_id: String,
    pub token: Zeroizing<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminAuthenticatedSession {
    pub session_id: String,
    pub user_id: String,
}

pub struct AdminAuthenticationService<'a> {
    pool: &'a PgPool,
    rate_limit_key: &'a AuthenticationRateLimitKey,
    key_provider: &'a dyn KeyProvider,
}

impl<'a> AdminAuthenticationService<'a> {
    pub fn new(
        pool: &'a PgPool,
        rate_limit_key: &'a AuthenticationRateLimitKey,
        key_provider: &'a dyn KeyProvider,
    ) -> Self {
        Self {
            pool,
            rate_limit_key,
            key_provider,
        }
    }

    /// Verify the Admin password and issue a short-lived opaque TOTP challenge.
    pub async fn begin_login(
        &self,
        identifier: &str,
        password: &str,
        peer_ip: IpAddr,
    ) -> Result<AdminLoginChallenge, AdminAuthenticationError> {
        let normalized =
            normalize_identifier(identifier).map_err(|_| AdminAuthenticationError::InvalidInput)?;
        if password.is_empty() || password.len() > 1024 {
            return Err(AdminAuthenticationError::InvalidInput);
        }

        let identifier_hash = self.rate_limit_key.fingerprint(
            b"usahapintar:admin-login-identifier:v1",
            normalized.as_bytes(),
        );
        let ip_subject = ip_rate_limit_subject(peer_ip);
        let ip_hash = self
            .rate_limit_key
            .fingerprint(b"usahapintar:admin-login-ip:v1", &ip_subject);

        let allowed: bool =
            sqlx::query_scalar("SELECT public.reserve_admin_login_attempt($1, $2)")
                .bind(identifier_hash.to_vec())
                .bind(ip_hash.to_vec())
                .fetch_one(self.pool)
                .await
                .map_err(|_| AdminAuthenticationError::Database)?;
        if !allowed {
            return Err(AdminAuthenticationError::RateLimited);
        }

        let credential = sqlx::query_as::<_, (String, String)>(
            "SELECT user_id::text, password_hash \
               FROM public.find_active_admin_password_login($1)",
        )
        .bind(&normalized)
        .fetch_optional(self.pool)
        .await
        .map_err(|_| AdminAuthenticationError::Database)?;

        let password = Zeroizing::new(password.to_owned());
        let verified = tokio::task::spawn_blocking(move || match credential {
            Some((user_id, encoded_hash)) => {
                match verify_password(password.as_bytes(), &encoded_hash) {
                    Ok(true) => Ok(Some(user_id)),
                    Ok(false) => Ok(None),
                    Err(_) => hash_password(password.as_bytes()).map(|_| None),
                }
            }
            None => hash_password(password.as_bytes()).map(|_| None),
        })
        .await
        .map_err(|_| AdminAuthenticationError::Internal)?
        .map_err(|_| AdminAuthenticationError::Internal)?;

        let Some(user_id) = verified else {
            return Err(AdminAuthenticationError::InvalidCredentials);
        };

        let challenge_token = Zeroizing::new(
            random_session_token().map_err(|_| AdminAuthenticationError::Internal)?,
        );
        let challenge_hash = hash_session_token(challenge_token.as_str());
        let (_, expires_at_unix): (String, i64) = sqlx::query_as(
            "SELECT challenge_id::text, expires_at_unix \
               FROM public.create_admin_login_challenge($1::uuid, $2)",
        )
        .bind(&user_id)
        .bind(challenge_hash.to_vec())
        .fetch_one(self.pool)
        .await
        .map_err(|_| AdminAuthenticationError::Database)?;

        Ok(AdminLoginChallenge {
            token: challenge_token,
            expires_at_unix,
        })
    }

    /// Verify TOTP and atomically consume the challenge/replay step while creating an Admin session.
    pub async fn complete_login(
        &self,
        challenge_token: &str,
        totp_code: &str,
    ) -> Result<AdminLoginSession, AdminAuthenticationError> {
        let challenge_hash =
            session_token_hash(challenge_token).map_err(|_| AdminAuthenticationError::InvalidChallenge)?;

        let credential = sqlx::query_as::<
            _,
            (String, Vec<u8>, Vec<u8>, String, Option<i64>),
        >(
            "SELECT user_id::text, secret_ciphertext, secret_nonce, \
                    encryption_key_version, last_accepted_step \
               FROM public.get_admin_login_totp_credentials($1)",
        )
        .bind(challenge_hash.to_vec())
        .fetch_optional(self.pool)
        .await
        .map_err(|_| AdminAuthenticationError::Database)?
        .ok_or(AdminAuthenticationError::InvalidChallenge)?;

        let (user_id, ciphertext, nonce, key_version_label, last_step) = credential;
        let key_version =
            parse_key_version(&key_version_label).map_err(|_| AdminAuthenticationError::Internal)?;
        let nonce: [u8; 12] = nonce
            .try_into()
            .map_err(|_| AdminAuthenticationError::Internal)?;
        if ciphertext.len() != 36 {
            return Err(AdminAuthenticationError::Internal);
        }

        let encrypted = EncryptedPayload {
            key_version,
            nonce,
            ciphertext,
        };
        let secret = decrypt_admin_totp_secret(self.key_provider, &user_id, &encrypted)
            .map_err(|_| AdminAuthenticationError::Internal)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AdminAuthenticationError::Internal)?
            .as_secs();
        let last_step = last_step
            .map(u64::try_from)
            .transpose()
            .map_err(|_| AdminAuthenticationError::Internal)?;
        let matched_step = match matching_step(&secret, totp_code, now, last_step) {
            Ok(Some(step)) => step,
            Ok(None) | Err(_) => {
                let _: bool = sqlx::query_scalar(
                    "SELECT public.record_admin_login_challenge_failure($1)",
                )
                .bind(challenge_hash.to_vec())
                .fetch_one(self.pool)
                .await
                .map_err(|_| AdminAuthenticationError::Database)?;
                return Err(AdminAuthenticationError::InvalidChallenge);
            }
        };

        let session_token = Zeroizing::new(
            random_session_token().map_err(|_| AdminAuthenticationError::Internal)?,
        );
        let session_hash = hash_session_token(session_token.as_str());
        let _: String = sqlx::query_scalar(
            "SELECT public.complete_admin_login($1, $2, $3)::text",
        )
        .bind(challenge_hash.to_vec())
        .bind(i64::try_from(matched_step).map_err(|_| AdminAuthenticationError::Internal)?)
        .bind(session_hash.to_vec())
        .fetch_one(self.pool)
        .await
        .map_err(|_| AdminAuthenticationError::InvalidChallenge)?;

        Ok(AdminLoginSession {
            user_id,
            token: session_token,
        })
    }

    pub async fn validate_session(
        &self,
        token: &str,
    ) -> Result<AdminAuthenticatedSession, AdminAuthenticationError> {
        let token_hash =
            session_token_hash(token).map_err(|_| AdminAuthenticationError::InvalidSession)?;
        let session = sqlx::query_as::<_, (String, String)>(
            "SELECT session_id::text, user_id::text \
               FROM public.get_active_admin_session($1)",
        )
        .bind(token_hash.to_vec())
        .fetch_optional(self.pool)
        .await
        .map_err(|_| AdminAuthenticationError::Database)?
        .ok_or(AdminAuthenticationError::InvalidSession)?;

        Ok(AdminAuthenticatedSession {
            session_id: session.0,
            user_id: session.1,
        })
    }

    pub async fn logout(&self, token: &str) -> Result<bool, AdminAuthenticationError> {
        let token_hash =
            session_token_hash(token).map_err(|_| AdminAuthenticationError::InvalidSession)?;
        sqlx::query_scalar("SELECT public.revoke_admin_session($1)")
            .bind(token_hash.to_vec())
            .fetch_one(self.pool)
            .await
            .map_err(|_| AdminAuthenticationError::Database)
    }
}
