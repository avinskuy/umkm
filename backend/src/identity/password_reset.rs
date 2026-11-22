use sqlx::PgPool;
use zeroize::Zeroizing;

use super::{
    authentication::session_token_hash,
    crypto::{random_bytes, CryptoError, KeyProvider},
    delivery::{
        DeliveryChannel, DispatchStatus, PasswordResetDeliveryMessage,
        PasswordResetDeliveryProvider,
    },
    password::{hash_password, PasswordError, PasswordPolicy},
    verification::{
        contact_aliases, decrypt_destination, validate_uuid, SessionRow, VerificationError,
    },
};

const PASSWORD_RESET_TOKEN_BYTES: usize = 32;
const MAX_PASSWORD_BYTES: usize = 1024;

#[derive(Debug)]
pub enum PasswordResetError {
    Database(sqlx::Error),
    Crypto(CryptoError),
    InvalidInput,
    InvalidStoredData,
    PasswordPolicyRejected,
    PasswordHashFailed,
}

impl std::fmt::Display for PasswordResetError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Database(_) => "password reset database operation failed",
            Self::Crypto(_) => "password reset cryptographic operation failed",
            Self::InvalidInput => "password reset input is invalid",
            Self::InvalidStoredData => "stored password reset data is invalid",
            Self::PasswordPolicyRejected => "new password does not meet the password policy",
            Self::PasswordHashFailed => "new password hashing failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PasswordResetError {}

impl From<sqlx::Error> for PasswordResetError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<CryptoError> for PasswordResetError {
    fn from(error: CryptoError) -> Self {
        Self::Crypto(error)
    }
}

impl From<VerificationError> for PasswordResetError {
    fn from(error: VerificationError) -> Self {
        match error {
            VerificationError::Database(error) => Self::Database(error),
            VerificationError::Crypto(error) => Self::Crypto(error),
            VerificationError::InvalidInput => Self::InvalidInput,
            VerificationError::InvalidStoredData => Self::InvalidStoredData,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasswordResetIssueResult {
    pub dispatch_status: DispatchStatus,
    pub expires_at_unix: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasswordResetCompletion {
    pub revoked_session_count: i32,
}

#[derive(sqlx::FromRow)]
struct ApprovedRecoveryDeliveryContext {
    session_id: String,
    channel: String,
    destination_ciphertext: Vec<u8>,
    destination_nonce: Vec<u8>,
    encryption_key_version: String,
    destination_hmac: Vec<u8>,
    hmac_key_version: String,
}

/// Layanan internal untuk menerbitkan token setelah kasus disetujui Admin.
pub struct PasswordResetIssuanceService<'a> {
    pool: &'a PgPool,
    keys: &'a dyn KeyProvider,
    delivery: &'a dyn PasswordResetDeliveryProvider,
    reset_url_base: String,
}

impl<'a> PasswordResetIssuanceService<'a> {
    pub fn new(
        pool: &'a PgPool,
        keys: &'a dyn KeyProvider,
        delivery: &'a dyn PasswordResetDeliveryProvider,
        reset_url_base: String,
    ) -> Result<Self, PasswordResetError> {
        if reset_url_base.len() > 512
            || !reset_url_base.starts_with("https://")
            || reset_url_base.trim() != reset_url_base
            || reset_url_base.contains('?')
            || reset_url_base.contains('#')
            || reset_url_base.chars().any(char::is_whitespace)
        {
            return Err(PasswordResetError::InvalidInput);
        }

        Ok(Self {
            pool,
            keys,
            delivery,
            reset_url_base,
        })
    }

    /// Token hanya dikirim ke kontak terverifikasi yang masih aktif pada akun target.
    /// Token mentah tidak pernah dikembalikan oleh layanan ini.
    pub async fn issue_for_approved_case(
        &self,
        case_id: &str,
    ) -> Result<PasswordResetIssueResult, PasswordResetError> {
        validate_uuid(case_id)?;

        let context = sqlx::query_as::<_, ApprovedRecoveryDeliveryContext>(
            "SELECT session_id::text, channel, destination_ciphertext, destination_nonce, \
                    encryption_key_version, destination_hmac, hmac_key_version \
               FROM public.get_approved_account_recovery_delivery_context($1::uuid)",
        )
        .bind(case_id)
        .fetch_one(self.pool)
        .await?;

        let session: SessionRow = (
            context.channel,
            context.destination_ciphertext,
            context.destination_nonce,
            context.encryption_key_version,
            context.destination_hmac,
            context.hmac_key_version,
        );
        let destination =
            decrypt_destination(self.keys, &context.session_id, "account_recovery", &session)?;
        let aliases = contact_aliases(self.keys, &session.0, &destination, &session)?;
        let alias_bytes = aliases
            .into_iter()
            .map(|alias| alias.digest.to_vec())
            .collect::<Vec<_>>();

        let token_bytes = Zeroizing::new(random_bytes::<PASSWORD_RESET_TOKEN_BYTES>()?);
        let token = Zeroizing::new(encode_hex(&token_bytes[..]));
        let token_hash =
            session_token_hash(&token).map_err(|_| PasswordResetError::InvalidInput)?;

        let expires_at_unix: i64 = sqlx::query_scalar(
            "SELECT expires_at_unix \
               FROM public.create_password_reset_token($1::uuid, $2, $3)",
        )
        .bind(case_id)
        .bind(token_hash.to_vec())
        .bind(alias_bytes)
        .fetch_one(self.pool)
        .await?;

        let mut reset_url = Zeroizing::new(self.reset_url_base.clone());
        reset_url.push_str("#token=");
        reset_url.push_str(&token);

        let channel = match session.0.as_str() {
            "email" => DeliveryChannel::Email,
            "whatsapp" => DeliveryChannel::WhatsApp,
            _ => return Err(PasswordResetError::InvalidStoredData),
        };

        let dispatch_status = self
            .delivery
            .dispatch(PasswordResetDeliveryMessage {
                destination,
                channel,
                reset_url,
                expires_at_unix,
            })
            .await;

        if dispatch_status == DispatchStatus::Failed {
            let _: bool = sqlx::query_scalar("SELECT public.revoke_password_reset_token($1)")
                .bind(token_hash.to_vec())
                .fetch_one(self.pool)
                .await?;
        }

        Ok(PasswordResetIssueResult {
            dispatch_status,
            expires_at_unix,
        })
    }
}

/// Layanan konsumsi token dan penggantian password secara atomik.
pub struct PasswordResetService<'a> {
    pool: &'a PgPool,
}

impl<'a> PasswordResetService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn complete(
        &self,
        token: &str,
        new_password: &str,
        policy: &dyn PasswordPolicy,
    ) -> Result<PasswordResetCompletion, PasswordResetError> {
        if new_password.is_empty() || new_password.len() > MAX_PASSWORD_BYTES {
            return Err(PasswordResetError::InvalidInput);
        }
        if !policy.accepts(new_password) {
            return Err(PasswordResetError::PasswordPolicyRejected);
        }

        let token_hash = session_token_hash(token).map_err(|_| PasswordResetError::InvalidInput)?;
        let password_hash = Zeroizing::new(
            hash_password(new_password.as_bytes())
                .map_err(|_: PasswordError| PasswordResetError::PasswordHashFailed)?,
        );

        let revoked_session_count: i32 = sqlx::query_scalar(
            "SELECT revoked_session_count FROM public.complete_password_reset($1, $2)",
        )
        .bind(token_hash.to_vec())
        .bind(password_hash.as_str())
        .fetch_one(self.pool)
        .await?;

        Ok(PasswordResetCompletion {
            revoked_session_count,
        })
    }
}

pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(*byte >> 4) as usize] as char);
        encoded.push(HEX[(*byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_url_base_requires_https_and_no_existing_fragment_or_query() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://user:pass@localhost/db")
            .expect("lazy pool configuration should be valid");
        assert!(PasswordResetIssuanceService::new(
            &pool,
            &UnavailableKeys,
            &UnavailableDelivery,
            "http://app.example.test/reset-password".to_owned(),
        )
        .is_err());
    }

    struct UnavailableKeys;

    impl KeyProvider for UnavailableKeys {
        fn active_key(
            &self,
            _purpose: super::super::crypto::KeyPurpose,
        ) -> Result<super::super::crypto::VersionedKey, CryptoError> {
            Err(CryptoError::KeyUnavailable)
        }

        fn key(
            &self,
            _purpose: super::super::crypto::KeyPurpose,
            _version: super::super::crypto::KeyVersion,
        ) -> Result<super::super::crypto::SecretKey, CryptoError> {
            Err(CryptoError::KeyUnavailable)
        }
    }

    struct UnavailableDelivery;

    impl PasswordResetDeliveryProvider for UnavailableDelivery {
        fn dispatch<'a>(
            &'a self,
            _message: PasswordResetDeliveryMessage,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = DispatchStatus> + Send + 'a>>
        {
            Box::pin(async { DispatchStatus::Failed })
        }
    }
}
