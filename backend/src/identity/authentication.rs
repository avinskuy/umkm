//! Autentikasi password dan lifecycle sesi opaque.

use ring::{digest, hmac, rand::SecureRandom};
use sqlx::PgPool;
use std::{net::IpAddr, sync::Arc};
use zeroize::Zeroizing;

use super::{
    audit::{append_audit_event, AuditActorType, AuditEvent, AuditOutcome},
    password::{hash_password, verify_password},
};

#[derive(Clone)]
pub struct AuthenticationRateLimitKey(Arc<Zeroizing<[u8; 32]>>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationConfigError {
    InvalidRateLimitKey,
}

impl std::fmt::Display for AuthenticationConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .write_str("AUTH_RATE_LIMIT_HMAC_KEY must contain exactly 64 hexadecimal characters")
    }
}

impl std::error::Error for AuthenticationConfigError {}

impl AuthenticationRateLimitKey {
    pub fn from_hex(value: &str) -> Result<Self, AuthenticationConfigError> {
        if value.len() != 64 {
            return Err(AuthenticationConfigError::InvalidRateLimitKey);
        }

        let mut key = Zeroizing::new([0_u8; 32]);
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            let high = hex_nibble(pair[0]).ok_or(AuthenticationConfigError::InvalidRateLimitKey)?;
            let low = hex_nibble(pair[1]).ok_or(AuthenticationConfigError::InvalidRateLimitKey)?;
            key[index] = (high << 4) | low;
        }

        if key.iter().all(|byte| *byte == 0) {
            return Err(AuthenticationConfigError::InvalidRateLimitKey);
        }

        Ok(Self(Arc::new(key)))
    }

    fn fingerprint(&self, purpose: &[u8], value: &[u8]) -> [u8; 32] {
        let key = hmac::Key::new(hmac::HMAC_SHA256, &self.0[..]);
        let mut input = Zeroizing::new(Vec::with_capacity(purpose.len() + value.len() + 1));
        input.extend_from_slice(purpose);
        input.push(0);
        input.extend_from_slice(value);
        let tag = hmac::sign(&key, &input);
        let mut output = [0_u8; 32];
        output.copy_from_slice(tag.as_ref());
        output
    }
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginRole {
    BusinessOwner,
    Investor,
}

impl LoginRole {
    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::BusinessOwner => "business_owner",
            Self::Investor => "investor",
        }
    }

    fn from_db_value(value: &str) -> Option<Self> {
        match value {
            "business_owner" => Some(Self::BusinessOwner),
            "investor" => Some(Self::Investor),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationError {
    Database,
    InvalidCredentials,
    InvalidSession,
    RateLimited,
    InvalidInput,
    Internal,
}

impl std::fmt::Display for AuthenticationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Database => "authentication database operation failed",
            Self::InvalidCredentials => "identifier or password is invalid",
            Self::InvalidSession => "authentication session is invalid or expired",
            Self::RateLimited => "too many login attempts",
            Self::InvalidInput => "login input is invalid",
            Self::Internal => "authentication operation failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for AuthenticationError {}

pub struct LoginSession {
    pub user_id: String,
    pub role: LoginRole,
    pub token: Zeroizing<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedSession {
    pub session_id: String,
    pub user_id: String,
    pub role: LoginRole,
    pub expires_in_seconds: i64,
}

pub struct RotatedSession {
    pub session_id: String,
    pub user_id: String,
    pub role: LoginRole,
    pub token: Zeroizing<String>,
    pub expires_in_seconds: i64,
}

pub struct AuthenticationService<'a> {
    pool: &'a PgPool,
    rate_limit_key: &'a AuthenticationRateLimitKey,
}

impl<'a> AuthenticationService<'a> {
    pub fn new(pool: &'a PgPool, rate_limit_key: &'a AuthenticationRateLimitKey) -> Self {
        Self {
            pool,
            rate_limit_key,
        }
    }

    pub async fn login(
        &self,
        identifier: &str,
        password: &str,
        role: LoginRole,
        peer_ip: IpAddr,
    ) -> Result<LoginSession, AuthenticationError> {
        let normalized = normalize_identifier(identifier)?;
        if password.is_empty() || password.len() > 1024 {
            return Err(AuthenticationError::InvalidInput);
        }

        let identifier_hash = self
            .rate_limit_key
            .fingerprint(b"usahapintar:login-identifier:v1", normalized.as_bytes());
        let ip_subject = ip_rate_limit_subject(peer_ip);
        let ip_hash = self
            .rate_limit_key
            .fingerprint(b"usahapintar:login-ip:v1", &ip_subject);

        let allowed: bool =
            sqlx::query_scalar("SELECT public.reserve_password_login_attempt($1, $2)")
                .bind(identifier_hash.to_vec())
                .bind(ip_hash.to_vec())
                .fetch_one(self.pool)
                .await
                .map_err(|_| AuthenticationError::Database)?;

        if !allowed {
            return Err(AuthenticationError::RateLimited);
        }

        let credential = sqlx::query_as::<_, (String, String)>(
            "SELECT user_id::text, password_hash \
             FROM public.find_active_password_login($1, $2)",
        )
        .bind(&normalized)
        .bind(role.as_db_value())
        .fetch_optional(self.pool)
        .await
        .map_err(|_| AuthenticationError::Database)?;

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
        .map_err(|_| AuthenticationError::Internal)?
        .map_err(|_| AuthenticationError::Internal)?;

        let Some(user_id) = verified else {
            self.append_failed_login(role).await?;
            return Err(AuthenticationError::InvalidCredentials);
        };

        let raw_token = random_session_token()?;
        let token_hash = hash_session_token(&raw_token);

        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthenticationError::Database)?;
        let session_id: String =
            sqlx::query_scalar("SELECT public.create_password_login_session($1::uuid, $2)::text")
                .bind(&user_id)
                .bind(token_hash.to_vec())
                .fetch_one(&mut *transaction)
                .await
                .map_err(|_| AuthenticationError::Database)?;

        sqlx::query("SELECT public.clear_password_login_identifier_limit($1)")
            .bind(identifier_hash.to_vec())
            .execute(&mut *transaction)
            .await
            .map_err(|_| AuthenticationError::Database)?;

        let audit_event = AuditEvent {
            actor_type: AuditActorType::User,
            actor_id: Some(&user_id),
            subject_user_id: Some(&user_id),
            action_code: "identity.login.succeeded",
            outcome: AuditOutcome::Success,
            reason_code: None,
            request_id: None,
            metadata: serde_json::json!({ "role": role.as_db_value(), "session_id": session_id }),
        };
        append_audit_event(&mut transaction, &audit_event)
            .await
            .map_err(|_| AuthenticationError::Database)?;
        transaction
            .commit()
            .await
            .map_err(|_| AuthenticationError::Database)?;

        Ok(LoginSession {
            user_id,
            role,
            token: Zeroizing::new(raw_token),
        })
    }

    async fn append_failed_login(&self, role: LoginRole) -> Result<(), AuthenticationError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthenticationError::Database)?;
        let audit_event = AuditEvent {
            actor_type: AuditActorType::System,
            actor_id: None,
            subject_user_id: None,
            action_code: "identity.login.failed",
            outcome: AuditOutcome::Failure,
            reason_code: Some("invalid_credentials"),
            request_id: None,
            metadata: serde_json::json!({ "role": role.as_db_value() }),
        };
        append_audit_event(&mut transaction, &audit_event)
            .await
            .map_err(|_| AuthenticationError::Database)?;
        transaction
            .commit()
            .await
            .map_err(|_| AuthenticationError::Database)
    }

    pub async fn validate_session(
        &self,
        token: &str,
    ) -> Result<AuthenticatedSession, AuthenticationError> {
        let token_hash = session_token_hash(token)?;
        let row = sqlx::query_as::<_, (String, String, String, i64)>(
            "SELECT session_id::text, user_id::text, primary_role, expires_in_seconds FROM public.get_active_password_login_session($1)",
        )
        .bind(token_hash.to_vec())
        .fetch_optional(self.pool)
        .await
        .map_err(|_| AuthenticationError::Database)?
        .ok_or(AuthenticationError::InvalidSession)?;

        Ok(AuthenticatedSession {
            session_id: row.0,
            user_id: row.1,
            role: LoginRole::from_db_value(&row.2).ok_or(AuthenticationError::InvalidSession)?,
            expires_in_seconds: row.3,
        })
    }

    pub async fn rotate_session(&self, token: &str) -> Result<RotatedSession, AuthenticationError> {
        let old_token_hash = session_token_hash(token)?;
        let new_token = random_session_token()?;
        let new_token_hash = hash_session_token(&new_token);

        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthenticationError::Database)?;
        let row = sqlx::query_as::<_, (String, String, String, i64)>(
            "SELECT session_id::text, user_id::text, primary_role, expires_in_seconds FROM public.rotate_active_password_login_session($1, $2)",
        )
        .bind(old_token_hash.to_vec())
        .bind(new_token_hash.to_vec())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| AuthenticationError::Database)?
        .ok_or(AuthenticationError::InvalidSession)?;

        let role = LoginRole::from_db_value(&row.2).ok_or(AuthenticationError::InvalidSession)?;
        let audit_event = AuditEvent {
            actor_type: AuditActorType::User,
            actor_id: Some(&row.1),
            subject_user_id: Some(&row.1),
            action_code: "identity.session.rotated",
            outcome: AuditOutcome::Success,
            reason_code: None,
            request_id: None,
            metadata: serde_json::json!({
                "role": role.as_db_value(),
                "session_id": row.0
            }),
        };
        append_audit_event(&mut transaction, &audit_event)
            .await
            .map_err(|_| AuthenticationError::Database)?;
        transaction
            .commit()
            .await
            .map_err(|_| AuthenticationError::Database)?;

        Ok(RotatedSession {
            session_id: row.0,
            user_id: row.1,
            role,
            token: Zeroizing::new(new_token),
            expires_in_seconds: row.3,
        })
    }

    pub async fn logout_all(
        &self,
        user_id: &str,
        role: LoginRole,
    ) -> Result<i32, AuthenticationError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| AuthenticationError::Database)?;
        let revoked_count: i32 =
            sqlx::query_scalar("SELECT public.revoke_all_password_login_sessions($1::uuid)")
                .bind(user_id)
                .fetch_one(&mut *transaction)
                .await
                .map_err(|_| AuthenticationError::Database)?;
        let count_bucket = match revoked_count {
            0 => "0",
            1 => "1",
            2..=5 => "2-5",
            _ => "6+",
        };
        let audit_event = AuditEvent {
            actor_type: AuditActorType::User,
            actor_id: Some(user_id),
            subject_user_id: Some(user_id),
            action_code: "identity.sessions.logout_all",
            outcome: AuditOutcome::Success,
            reason_code: None,
            request_id: None,
            metadata: serde_json::json!({
                "role": role.as_db_value(),
                "count_bucket": count_bucket
            }),
        };
        append_audit_event(&mut transaction, &audit_event)
            .await
            .map_err(|_| AuthenticationError::Database)?;
        transaction
            .commit()
            .await
            .map_err(|_| AuthenticationError::Database)?;
        Ok(revoked_count)
    }

    pub async fn logout(&self, token: &str) -> Result<(), AuthenticationError> {
        let Ok(token_hash) = session_token_hash(token) else {
            return Ok(());
        };
        sqlx::query_scalar::<_, bool>("SELECT public.revoke_password_login_session($1)")
            .bind(token_hash.to_vec())
            .fetch_one(self.pool)
            .await
            .map_err(|_| AuthenticationError::Database)?;
        Ok(())
    }
}

pub fn normalize_identifier(value: &str) -> Result<String, AuthenticationError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 254 {
        return Err(AuthenticationError::InvalidInput);
    }

    if value.contains('@') {
        let normalized = value.to_ascii_lowercase();
        let mut parts = normalized.split('@');
        let local = parts.next().unwrap_or_default();
        let domain = parts.next().unwrap_or_default();
        if local.is_empty()
            || domain.is_empty()
            || parts.next().is_some()
            || normalized.chars().any(char::is_whitespace)
        {
            return Err(AuthenticationError::InvalidInput);
        }
        return Ok(normalized);
    }

    let compact = value
        .chars()
        .filter(|character| !matches!(character, ' ' | '-' | '(' | ')'))
        .collect::<String>();
    let digits = compact
        .strip_prefix('+')
        .ok_or(AuthenticationError::InvalidInput)?;
    if !(8..=15).contains(&digits.len())
        || digits.starts_with('0')
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(AuthenticationError::InvalidInput);
    }
    Ok(compact)
}

fn ip_rate_limit_subject(ip: IpAddr) -> Vec<u8> {
    match ip {
        IpAddr::V4(address) => {
            let mut value = vec![4_u8];
            value.extend_from_slice(&address.octets());
            value
        }
        IpAddr::V6(address) => {
            let mut value = vec![6_u8];
            value.extend_from_slice(&address.octets()[..8]);
            value
        }
    }
}

fn random_session_token() -> Result<String, AuthenticationError> {
    let mut bytes = Zeroizing::new([0_u8; 32]);
    ring::rand::SystemRandom::new()
        .fill(&mut bytes[..])
        .map_err(|_| AuthenticationError::Internal)?;
    Ok(hex_encode(&bytes[..]))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn session_token_hash(token: &str) -> Result<[u8; 32], AuthenticationError> {
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AuthenticationError::InvalidSession);
    }
    Ok(hash_session_token(token))
}

pub fn hash_session_token(token: &str) -> [u8; 32] {
    let digest = digest::digest(&digest::SHA256, token.as_bytes());
    let mut output = [0_u8; 32];
    output.copy_from_slice(digest.as_ref());
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_email_and_e164_phone_identifiers() {
        assert_eq!(
            normalize_identifier("  User.Name@Example.Test ").unwrap(),
            "user.name@example.test"
        );
        assert_eq!(
            normalize_identifier("+62 812-3456-7890").unwrap(),
            "+6281234567890"
        );
    }

    #[test]
    fn rejects_invalid_login_identifiers() {
        assert_eq!(
            normalize_identifier(""),
            Err(AuthenticationError::InvalidInput)
        );
        assert_eq!(
            normalize_identifier("user@@example.test"),
            Err(AuthenticationError::InvalidInput)
        );
        assert_eq!(
            normalize_identifier("081234567890"),
            Err(AuthenticationError::InvalidInput)
        );
        assert_eq!(
            normalize_identifier("6281234567890"),
            Err(AuthenticationError::InvalidInput)
        );
        assert_eq!(
            normalize_identifier("+62abcdefgh"),
            Err(AuthenticationError::InvalidInput)
        );
    }

    #[test]
    fn rate_limit_key_requires_a_nonzero_256_bit_hex_secret() {
        assert!(AuthenticationRateLimitKey::from_hex("00").is_err());
        assert!(AuthenticationRateLimitKey::from_hex(&"00".repeat(32)).is_err());
        assert!(AuthenticationRateLimitKey::from_hex(&"ab".repeat(32)).is_ok());
    }

    #[test]
    fn session_tokens_are_sha256_hashed_and_not_stored_in_plaintext() {
        let first = hash_session_token("a".repeat(64).as_str());
        let second = hash_session_token("b".repeat(64).as_str());
        assert_eq!(first.len(), 32);
        assert_ne!(first, second);
    }

    #[test]
    fn ipv6_rate_limit_fingerprint_groups_addresses_by_network_prefix() {
        let first: IpAddr = "2001:db8:abcd:12::1".parse().unwrap();
        let second: IpAddr = "2001:db8:abcd:12::ffff".parse().unwrap();
        assert_eq!(ip_rate_limit_subject(first), ip_rate_limit_subject(second));
    }
}
