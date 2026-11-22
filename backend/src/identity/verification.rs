use sqlx::PgPool;
use zeroize::Zeroizing;

use super::{
    crypto::{
        contact_fingerprint, contact_fingerprint_aliases, decrypt, encrypt, key_version_label,
        otp_digest, parse_key_version, random_bytes, verify_otp_digest, EncryptedPayload,
        KeyProvider, OtpVerificationContext,
    },
    delivery::{DeliveryChannel, DispatchStatus, OtpDeliveryMessage, OtpDeliveryProvider},
    registration::{create_client_user, ClientRegistration, ContactChannel},
};

const OTP_DIGITS: u32 = 6;
const OTP_RANGE: u64 = 1_000_000;
const UUID_LEN: usize = 16;

type SessionRow = (String, Vec<u8>, Vec<u8>, String, Vec<u8>, String);
type ChallengeRow = (String, Vec<u8>, String);

pub struct VerificationService<'a> {
    pool: &'a PgPool,
    keys: &'a dyn KeyProvider,
    delivery: &'a dyn OtpDeliveryProvider,
}

pub struct RegistrationStart {
    pub user_id: String,
    pub session_id: String,
    pub dispatch_status: DispatchStatus,
}

#[derive(Debug)]
pub enum VerificationError {
    Database(sqlx::Error),
    Crypto(super::crypto::CryptoError),
    InvalidInput,
    InvalidStoredData,
}

impl std::fmt::Display for VerificationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Database(_) => "verification database operation failed",
            Self::Crypto(_) => "verification cryptographic operation failed",
            Self::InvalidInput => "verification input is invalid",
            Self::InvalidStoredData => "stored verification data is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for VerificationError {}

impl From<sqlx::Error> for VerificationError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<super::crypto::CryptoError> for VerificationError {
    fn from(error: super::crypto::CryptoError) -> Self {
        Self::Crypto(error)
    }
}

impl<'a> VerificationService<'a> {
    pub fn new(
        pool: &'a PgPool,
        keys: &'a dyn KeyProvider,
        delivery: &'a dyn OtpDeliveryProvider,
    ) -> Self {
        Self {
            pool,
            keys,
            delivery,
        }
    }

    /// Membuat akun pending dan sesi OTP dalam satu transaksi, lalu dispatch setelah commit.
    pub async fn register(
        &self,
        registration: &ClientRegistration<'_>,
    ) -> Result<RegistrationStart, VerificationError> {
        let (verification_channel, contact, email, phone) =
            normalized_registration_contact(registration)?;
        let session_id = new_uuid()?;
        let aad = destination_associated_data(&session_id, verification_channel);
        let encrypted = encrypt(self.keys, contact.as_bytes(), &aad)?;
        let fingerprint = contact_fingerprint(self.keys, verification_channel, &contact)?;

        let normalized_registration = ClientRegistration {
            role: registration.role,
            initial_contact_channel: registration.initial_contact_channel,
            email_normalized: email.as_deref(),
            phone_e164: phone.as_deref(),
            password_hash: registration.password_hash,
        };

        let mut transaction = self.pool.begin().await?;
        let user_id = create_client_user(&mut *transaction, &normalized_registration).await?;

        let persisted_session_id = sqlx::query_scalar::<_, String>(
            "SELECT public.create_registration_verification_session(                $1::uuid, $2::uuid, $3, $4, $5, $6, $7, $8,                 clock_timestamp() + INTERVAL '10 minutes')::text",
        )
        .bind(&session_id)
        .bind(&user_id)
        .bind(verification_channel)
        .bind(encrypted.ciphertext)
        .bind(encrypted.nonce.to_vec())
        .bind(key_version_label(encrypted.key_version))
        .bind(fingerprint.digest.to_vec())
        .bind(key_version_label(fingerprint.key_version))
        .fetch_one(&mut *transaction)
        .await?;
        if persisted_session_id != session_id {
            return Err(VerificationError::InvalidStoredData);
        }

        transaction.commit().await?;

        let dispatch_status = self.dispatch_new_challenge(&session_id).await?;

        Ok(RegistrationStart {
            user_id,
            session_id,
            dispatch_status,
        })
    }

    /// Menerbitkan challenge baru setelah database menyetujui cooldown dan kuota.
    pub async fn resend(&self, session_id: &str) -> Result<DispatchStatus, VerificationError> {
        validate_uuid(session_id)?;
        self.dispatch_new_challenge(session_id).await
    }

    /// Memeriksa OTP, mencatat percobaan, dan mengaktifkan akun secara atomik di database.
    pub async fn verify(
        &self,
        session_id: &str,
        code: &str,
    ) -> Result<bool, VerificationError> {
        validate_uuid(session_id)?;
        if code.len() != OTP_DIGITS as usize || !code.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(VerificationError::InvalidInput);
        }

        let session = self.load_session(session_id).await?;
        let channel = parse_delivery_channel(&session.0)?;
        let destination = self.decrypt_destination(session_id, &session)?;
        let aliases = self.contact_aliases(channel, &destination, &session)?;
        let (challenge_id, stored_hash, hash_version) = sqlx::query_as::<_, ChallengeRow>(
            "SELECT challenge_id::text, code_hash, hash_key_version              FROM public.get_pending_otp_challenge($1::uuid)",
        )
        .bind(session_id)
        .fetch_one(self.pool)
        .await?;

        let expected_digest: [u8; 32] = stored_hash
            .as_slice()
            .try_into()
            .map_err(|_| VerificationError::InvalidStoredData)?;
        let context = OtpVerificationContext {
            challenge_id: challenge_id.as_bytes(),
            session_id: session_id.as_bytes(),
            purpose: "registration",
            channel: &session.0,
            code: code.as_bytes(),
        };
        let matches = verify_otp_digest(
            self.keys,
            parse_key_version(&hash_version)?,
            &context,
            &expected_digest,
        )?;

        let alias_bytes = aliases
            .into_iter()
            .map(|alias| alias.digest.to_vec())
            .collect::<Vec<_>>();
        sqlx::query_scalar::<_, bool>(
            "SELECT public.record_otp_verification_attempt(                $1::uuid, $2::uuid, $3, $4)",
        )
        .bind(session_id)
        .bind(&challenge_id)
        .bind(matches)
        .bind(alias_bytes)
        .fetch_one(self.pool)
        .await
        .map_err(VerificationError::from)
    }

    async fn dispatch_new_challenge(
        &self,
        session_id: &str,
    ) -> Result<DispatchStatus, VerificationError> {
        let session = self.load_session(session_id).await?;
        let channel = parse_delivery_channel(&session.0)?;
        let destination = self.decrypt_destination(session_id, &session)?;
        let aliases = self.contact_aliases(channel, &destination, &session)?;
        let challenge_id = new_uuid()?;
        let code = generate_otp()?;
        let context = OtpVerificationContext {
            challenge_id: challenge_id.as_bytes(),
            session_id: session_id.as_bytes(),
            purpose: "registration",
            channel: &session.0,
            code: code.as_bytes(),
        };
        let digest = otp_digest(self.keys, &context)?;
        let alias_bytes = aliases
            .into_iter()
            .map(|alias| alias.digest.to_vec())
            .collect::<Vec<_>>();

        let mut transaction = self.pool.begin().await?;
        let delivery_event_id = sqlx::query_scalar::<_, String>(
            "SELECT delivery_event_id::text              FROM public.reserve_otp_challenge(                $1::uuid, $2::uuid, $3, $4, $5)",
        )
        .bind(session_id)
        .bind(&challenge_id)
        .bind(digest.digest.to_vec())
        .bind(key_version_label(digest.key_version))
        .bind(alias_bytes)
        .fetch_one(&mut *transaction)
        .await?;
        transaction.commit().await?;

        let dispatch_status = self
            .delivery
            .dispatch(OtpDeliveryMessage {
                destination,
                channel: delivery_channel(channel),
                code,
            })
            .await;

        sqlx::query("SELECT public.record_otp_delivery_result($1::uuid, $2)")
            .bind(delivery_event_id)
            .bind(dispatch_status.as_db_value())
            .execute(self.pool)
            .await?;

        Ok(dispatch_status)
    }

    async fn load_session(&self, session_id: &str) -> Result<SessionRow, VerificationError> {
        sqlx::query_as::<_, SessionRow>(
            "SELECT channel, destination_ciphertext, destination_nonce,                     encryption_key_version, destination_hmac, hmac_key_version              FROM public.get_registration_verification_session($1::uuid)",
        )
        .bind(session_id)
        .fetch_one(self.pool)
        .await
        .map_err(VerificationError::from)
    }

    fn decrypt_destination(
        &self,
        session_id: &str,
        session: &SessionRow,
    ) -> Result<Zeroizing<String>, VerificationError> {
        let nonce: [u8; 12] = session
            .2
            .as_slice()
            .try_into()
            .map_err(|_| VerificationError::InvalidStoredData)?;
        let payload = EncryptedPayload {
            key_version: parse_key_version(&session.3)?,
            nonce,
            ciphertext: session.1.clone(),
        };
        let aad = destination_associated_data(session_id, &session.0);
        let plaintext = decrypt(self.keys, &payload, &aad)?;
        let contact = String::from_utf8(plaintext.to_vec())
            .map_err(|_| VerificationError::InvalidStoredData)?;
        Ok(Zeroizing::new(contact))
    }

    fn contact_aliases(
        &self,
        channel: &str,
        destination: &str,
        session: &SessionRow,
    ) -> Result<Vec<super::crypto::KeyedDigest>, VerificationError> {
        let aliases = contact_fingerprint_aliases(self.keys, channel, destination)?;
        let stored_version = parse_key_version(&session.5)?;
        let stored_fingerprint_matches = aliases.iter().any(|alias| {
            alias.key_version == stored_version && alias.digest.as_slice() == session.4.as_slice()
        });
        if !stored_fingerprint_matches {
            return Err(VerificationError::InvalidStoredData);
        }
        Ok(aliases)
    }
}

fn normalized_registration_contact<'a>(
    registration: &'a ClientRegistration<'_>,
) -> Result<(&'static str, String, Option<String>, Option<String>), VerificationError> {
    match registration.initial_contact_channel {
        ContactChannel::Email => {
            if registration.phone_e164.is_some() {
                return Err(VerificationError::InvalidInput);
            }
            let email = registration
                .email_normalized
                .ok_or(VerificationError::InvalidInput)?
                .trim()
                .to_ascii_lowercase();
            let mut parts = email.split('@');
            let local = parts.next().unwrap_or_default();
            let domain = parts.next().unwrap_or_default();
            if local.is_empty()
                || domain.is_empty()
                || parts.next().is_some()
                || email.chars().any(char::is_whitespace)
            {
                return Err(VerificationError::InvalidInput);
            }
            Ok(("email", email.clone(), Some(email), None))
        }
        ContactChannel::Phone => {
            if registration.email_normalized.is_some() {
                return Err(VerificationError::InvalidInput);
            }
            let phone = registration
                .phone_e164
                .ok_or(VerificationError::InvalidInput)?
                .trim()
                .to_owned();
            let digits = phone.strip_prefix('+').unwrap_or_default();
            if !(8..=15).contains(&digits.len())
                || !digits.bytes().all(|byte| byte.is_ascii_digit())
                || digits.starts_with('0')
            {
                return Err(VerificationError::InvalidInput);
            }
            Ok(("whatsapp", phone.clone(), None, Some(phone)))
        }
    }
}

fn parse_delivery_channel(channel: &str) -> Result<&'static str, VerificationError> {
    match channel {
        "email" => Ok("email"),
        "whatsapp" => Ok("whatsapp"),
        _ => Err(VerificationError::InvalidStoredData),
    }
}

fn delivery_channel(channel: &str) -> DeliveryChannel {
    match channel {
        "email" => DeliveryChannel::Email,
        _ => DeliveryChannel::WhatsApp,
    }
}

fn destination_associated_data(session_id: &str, channel: &str) -> Vec<u8> {
    format!("identity:verification-destination:v1:{session_id}:registration:{channel}").into_bytes()
}

fn validate_uuid(value: &str) -> Result<(), VerificationError> {
    if value.len() != 36
        || !value
            .bytes()
            .enumerate()
            .all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte),
            })
    {
        return Err(VerificationError::InvalidInput);
    }
    Ok(())
}

fn new_uuid() -> Result<String, VerificationError> {
    let mut bytes = random_bytes::<UUID_LEN>()?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    ))
}

fn generate_otp() -> Result<Zeroizing<String>, VerificationError> {
    let acceptance_limit = (u32::MAX as u64 + 1) / OTP_RANGE * OTP_RANGE;
    loop {
        let bytes = random_bytes::<4>()?;
        let value = u32::from_be_bytes(bytes) as u64;
        if value < acceptance_limit {
            return Ok(Zeroizing::new(format!(
                "{:0width$}",
                value % OTP_RANGE,
                width = OTP_DIGITS as usize
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_generator_creates_valid_v4_identifiers() {
        let value = new_uuid().expect("uuid generation should succeed");
        validate_uuid(&value).expect("generated UUID should be valid");
        assert_eq!(value.as_bytes()[14], b'4');
        assert!(matches!(value.as_bytes()[19], b'8'..=b'b'));
    }

    #[test]
    fn otp_generator_returns_six_ascii_digits() {
        for _ in 0..100 {
            let code = generate_otp().expect("OTP generation should succeed");
            assert_eq!(code.len(), OTP_DIGITS as usize);
            assert!(code.bytes().all(|byte| byte.is_ascii_digit()));
        }
    }

    #[test]
    fn registration_contact_is_normalized_and_channel_bound() {
        let registration = ClientRegistration {
            role: super::super::registration::ClientRole::BusinessOwner,
            initial_contact_channel: ContactChannel::Email,
            email_normalized: Some("  USER@example.test "),
            phone_e164: None,
            password_hash: "encoded-hash",
        };
        let (channel, contact, email, phone) =
            normalized_registration_contact(&registration).unwrap();
        assert_eq!(channel, "email");
        assert_eq!(contact, "user@example.test");
        assert_eq!(email.as_deref(), Some("user@example.test"));
        assert!(phone.is_none());
    }

    #[test]
    fn registration_rejects_mismatched_contact_channels() {
        let registration = ClientRegistration {
            role: super::super::registration::ClientRole::Investor,
            initial_contact_channel: ContactChannel::Phone,
            email_normalized: Some("user@example.test"),
            phone_e164: Some("+628123456789"),
            password_hash: "encoded-hash",
        };
        assert!(matches!(
            normalized_registration_contact(&registration),
            Err(VerificationError::InvalidInput)
        ));
    }
}
