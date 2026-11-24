use sqlx::PgPool;
use zeroize::Zeroizing;

use super::{
    crypto::{
        contact_fingerprint, contact_fingerprint_aliases, decrypt, encrypt, key_version_label,
        otp_digest, parse_key_version, random_bytes, verify_otp_digest, EncryptedPayload,
        KeyProvider, OtpVerificationContext,
    },
    delivery::{DeliveryChannel, DispatchStatus, OtpDeliveryMessage, OtpDeliveryProvider},
    flowkirim::{FlowKirimProviderOutcome, FlowKirimVerifyProvider},
    registration::{create_client_user, ClientRegistration, ContactChannel},
};

const OTP_DIGITS: u32 = 6;
const OTP_RANGE: u64 = 1_000_000;
const UUID_LEN: usize = 16;

pub(crate) type SessionRow = (String, Vec<u8>, Vec<u8>, String, Vec<u8>, String);
type ChallengeRow = (String, Vec<u8>, String);
type NormalizedContact = (&'static str, String, Option<String>, Option<String>);

pub struct VerificationService<'a> {
    pool: &'a PgPool,
    keys: &'a dyn KeyProvider,
    delivery: &'a dyn OtpDeliveryProvider,
    flowkirim: Option<&'a dyn FlowKirimVerifyProvider>,
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
    ProviderUnavailable,
}

impl std::fmt::Display for VerificationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Database(_) => "verification database operation failed",
            Self::Crypto(_) => "verification cryptographic operation failed",
            Self::InvalidInput => "verification input is invalid",
            Self::InvalidStoredData => "stored verification data is invalid",
            Self::ProviderUnavailable => "verification provider is unavailable",
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
            flowkirim: None,
        }
    }

    pub fn new_with_flowkirim(
        pool: &'a PgPool,
        keys: &'a dyn KeyProvider,
        delivery: &'a dyn OtpDeliveryProvider,
        flowkirim: &'a dyn FlowKirimVerifyProvider,
    ) -> Self {
        Self {
            pool,
            keys,
            delivery,
            flowkirim: Some(flowkirim),
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
        let aad = destination_associated_data(&session_id, verification_channel, "registration");
        let encrypted = encrypt(self.keys, contact.as_bytes(), &aad)?;
        let fingerprint = contact_fingerprint(self.keys, verification_channel, &contact)?;

        let normalized_registration = ClientRegistration {
            display_name: registration.display_name,
            role: registration.role,
            initial_contact_channel: registration.initial_contact_channel,
            email_normalized: email.as_deref(),
            phone_e164: phone.as_deref(),
            password_hash: registration.password_hash,
            terms_of_service_version: registration.terms_of_service_version,
            privacy_policy_version: registration.privacy_policy_version,
            marketing_consent: registration.marketing_consent,
            marketing_notice_version: registration.marketing_notice_version,
        };

        let mut transaction = self.pool.begin().await?;
        let user_id = create_client_user(&mut transaction, &normalized_registration).await?;

        let persisted_session_id = sqlx::query_scalar::<_, String>(
            "SELECT public.create_registration_verification_session(                $1::uuid, $2::uuid, $3, $4, $5, $6, $7, $8,                 clock_timestamp() + INTERVAL '10 minutes')::text",
        )
        .bind(&session_id)
        .bind(&user_id)
        .bind(verification_channel)
        .bind(encrypted.ciphertext)
        .bind(encrypted.nonce.to_vec())
        .bind(key_version_label(encrypted.key_version)?)
        .bind(fingerprint.digest.to_vec())
        .bind(key_version_label(fingerprint.key_version)?)
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
    pub async fn verify(&self, session_id: &str, code: &str) -> Result<bool, VerificationError> {
        validate_uuid(session_id)?;
        let session = load_verification_session(self.pool, session_id, "registration").await?;
        if session.0 == "whatsapp" {
            let provider = self
                .flowkirim
                .ok_or(VerificationError::ProviderUnavailable)?;
            return verify_flowkirim_session(self.pool, self.keys, provider, session_id, code)
                    .await;
        }
        verify_otp_session(self.pool, self.keys, session_id, "registration", code).await
    }

    async fn dispatch_new_challenge(
        &self,
        session_id: &str,
    ) -> Result<DispatchStatus, VerificationError> {
        let session = load_verification_session(self.pool, session_id, "registration").await?;
        if session.0 == "whatsapp" {
            let provider = self.flowkirim.ok_or(VerificationError::ProviderUnavailable)?;
            return dispatch_flowkirim_challenge(self.pool, self.keys, provider, session_id)
                    .await;
        }
        dispatch_otp_challenge(
            self.pool,
            self.keys,
            self.delivery,
            session_id,
            "registration",
        )
        .await
    }
}

async fn dispatch_flowkirim_challenge(
    pool: &PgPool,
    keys: &dyn KeyProvider,
    provider: &dyn FlowKirimVerifyProvider,
    session_id: &str,
) -> Result<DispatchStatus, VerificationError> {
    validate_uuid(session_id)?;
    let session = load_verification_session(pool, session_id, "registration").await?;
    if session.0 != "whatsapp" {
        return Err(VerificationError::InvalidInput);
    }
    let destination = decrypt_destination(keys, session_id, "registration", &session)?;
    let aliases = contact_aliases(keys, "whatsapp", &destination, &session)?;
    let alias_bytes = aliases
        .into_iter()
        .map(|alias| alias.digest.to_vec())
        .collect::<Vec<_>>();
    let challenge_id = new_uuid()?;

    let (reserved_challenge_id, delivery_event_id) = sqlx::query_as::<_, (String, String)>(
        "SELECT challenge_id::text, delivery_event_id::text FROM public.reserve_flowkirim_otp_challenge($1::uuid, $2::uuid, $3)",
    )
    .bind(session_id)
    .bind(&challenge_id)
    .bind(alias_bytes)
    .fetch_one(pool)
    .await?;

    if reserved_challenge_id != challenge_id {
        return Err(VerificationError::InvalidStoredData);
    }

    let idempotency_key = format!("registration-whatsapp-{challenge_id}");
    let status = match provider.start(destination, idempotency_key).await {
        FlowKirimProviderOutcome::Accepted(request_id) => {
            if request_id.trim().is_empty() || request_id.len() > 256 {
                DispatchStatus::Unknown
            } else {
                sqlx::query("SELECT public.bind_flowkirim_otp_request($1::uuid, $2::uuid, $3)")
                    .bind(session_id)
                    .bind(&challenge_id)
                    .bind(request_id)
                    .execute(pool)
                    .await?;
                DispatchStatus::Accepted
            }
        }
        FlowKirimProviderOutcome::Rejected => DispatchStatus::Failed,
        FlowKirimProviderOutcome::Unavailable => DispatchStatus::Unknown,
    };

    sqlx::query("SELECT public.record_otp_delivery_result($1::uuid, $2)")
        .bind(delivery_event_id)
        .bind(status.as_db_value())
        .execute(pool)
        .await?;

    Ok(status)
}

async fn verify_flowkirim_session(
    pool: &PgPool,
    keys: &dyn KeyProvider,
    provider: &dyn FlowKirimVerifyProvider,
    session_id: &str,
    code: &str,
) -> Result<bool, VerificationError> {
    if code.len() != OTP_DIGITS as usize || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(VerificationError::InvalidInput);
    }
    let session = load_verification_session(pool, session_id, "registration").await?;
    if session.0 != "whatsapp" {
        return Err(VerificationError::InvalidInput);
    }
    let destination = decrypt_destination(keys, session_id, "registration", &session)?;
    let aliases = contact_aliases(keys, "whatsapp", &destination, &session)?;
    let (challenge_id, request_id) = sqlx::query_as::<_, (String, String)>(
        "SELECT challenge_id::text, provider_request_id FROM public.get_pending_flowkirim_otp_challenge($1::uuid)",
    )
    .bind(session_id)
    .fetch_one(pool)
    .await?;

    let provider_matches = match provider
        .check(request_id, Zeroizing::new(code.to_owned()))
        .await
    {
        FlowKirimProviderOutcome::Accepted(matches) => matches,
        FlowKirimProviderOutcome::Rejected => false,
        FlowKirimProviderOutcome::Unavailable => {
            return Err(VerificationError::ProviderUnavailable)
        }
    };
    let alias_bytes = aliases
        .into_iter()
        .map(|alias| alias.digest.to_vec())
        .collect::<Vec<_>>();
    sqlx::query_scalar::<_, bool>(
        "SELECT public.record_otp_verification_attempt($1::uuid, $2::uuid, $3, $4)",
    )
    .bind(session_id)
    .bind(challenge_id)
    .bind(provider_matches)
    .bind(alias_bytes)
    .fetch_one(pool)
    .await
    .map_err(VerificationError::from)
}

pub(crate) async fn dispatch_otp_challenge(
    pool: &PgPool,
    keys: &dyn KeyProvider,
    delivery: &dyn OtpDeliveryProvider,
    session_id: &str,
    purpose: &str,
) -> Result<DispatchStatus, VerificationError> {
    validate_uuid(session_id)?;
    let session = load_verification_session(pool, session_id, purpose).await?;
    let channel = parse_delivery_channel(&session.0)?;
    let destination = decrypt_destination(keys, session_id, purpose, &session)?;
    let aliases = contact_aliases(keys, channel, &destination, &session)?;
    let challenge_id = new_uuid()?;
    let code = generate_otp()?;
    let context = OtpVerificationContext {
        challenge_id: challenge_id.as_bytes(),
        session_id: session_id.as_bytes(),
        purpose,
        channel: &session.0,
        code: code.as_bytes(),
    };
    let digest = otp_digest(keys, &context)?;
    let alias_bytes = aliases
        .into_iter()
        .map(|alias| alias.digest.to_vec())
        .collect::<Vec<_>>();

    let mut transaction = pool.begin().await?;
    let delivery_event_id = sqlx::query_scalar::<_, String>(
        "SELECT delivery_event_id::text FROM public.reserve_otp_challenge($1::uuid, $2::uuid, $3, $4, $5)",
    )
    .bind(session_id)
    .bind(&challenge_id)
    .bind(digest.digest.to_vec())
    .bind(key_version_label(digest.key_version)?)
    .bind(alias_bytes)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;

    let dispatch_status = delivery
        .dispatch(OtpDeliveryMessage {
            destination,
            channel: delivery_channel(channel),
            code,
        })
        .await;

    sqlx::query("SELECT public.record_otp_delivery_result($1::uuid, $2)")
        .bind(delivery_event_id)
        .bind(dispatch_status.as_db_value())
        .execute(pool)
        .await?;

    Ok(dispatch_status)
}

pub(crate) async fn verify_otp_session(
    pool: &PgPool,
    keys: &dyn KeyProvider,
    session_id: &str,
    purpose: &str,
    code: &str,
) -> Result<bool, VerificationError> {
    validate_uuid(session_id)?;
    if !matches!(purpose, "registration" | "account_recovery")
        || code.len() != OTP_DIGITS as usize
        || !code.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(VerificationError::InvalidInput);
    }

    let session = load_verification_session(pool, session_id, purpose).await?;
    let channel = parse_delivery_channel(&session.0)?;
    let destination = decrypt_destination(keys, session_id, purpose, &session)?;
    let aliases = contact_aliases(keys, channel, &destination, &session)?;
    let (challenge_id, stored_hash, hash_version) = sqlx::query_as::<_, ChallengeRow>(
        "SELECT challenge_id::text, code_hash, hash_key_version FROM public.get_pending_otp_challenge($1::uuid)",
    )
    .bind(session_id)
    .fetch_one(pool)
    .await?;

    let expected_digest: [u8; 32] = stored_hash
        .as_slice()
        .try_into()
        .map_err(|_| VerificationError::InvalidStoredData)?;
    let context = OtpVerificationContext {
        challenge_id: challenge_id.as_bytes(),
        session_id: session_id.as_bytes(),
        purpose,
        channel: &session.0,
        code: code.as_bytes(),
    };
    let matches = verify_otp_digest(
        keys,
        parse_key_version(&hash_version)?,
        &context,
        &expected_digest,
    )?;

    let alias_bytes = aliases
        .into_iter()
        .map(|alias| alias.digest.to_vec())
        .collect::<Vec<_>>();
    sqlx::query_scalar::<_, bool>(
        "SELECT public.record_otp_verification_attempt($1::uuid, $2::uuid, $3, $4)",
    )
    .bind(session_id)
    .bind(&challenge_id)
    .bind(matches)
    .bind(alias_bytes)
    .fetch_one(pool)
    .await
    .map_err(VerificationError::from)
}

pub(crate) async fn load_verification_session(
    pool: &PgPool,
    session_id: &str,
    purpose: &str,
) -> Result<SessionRow, VerificationError> {
    let query = match purpose {
        "registration" => {
            "SELECT channel, destination_ciphertext, destination_nonce, encryption_key_version, destination_hmac, hmac_key_version FROM public.get_registration_verification_session($1::uuid)"
        }
        "account_recovery" => {
            "SELECT channel, destination_ciphertext, destination_nonce, encryption_key_version, destination_hmac, hmac_key_version FROM public.get_account_recovery_verification_session($1::uuid)"
        }
        _ => return Err(VerificationError::InvalidInput),
    };
    sqlx::query_as::<_, SessionRow>(query)
        .bind(session_id)
        .fetch_one(pool)
        .await
        .map_err(VerificationError::from)
}

pub(crate) fn decrypt_destination(
    keys: &dyn KeyProvider,
    session_id: &str,
    purpose: &str,
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
    let aad = destination_associated_data(session_id, &session.0, purpose);
    let plaintext = decrypt(keys, &payload, &aad)?;
    let contact =
        String::from_utf8(plaintext.to_vec()).map_err(|_| VerificationError::InvalidStoredData)?;
    Ok(Zeroizing::new(contact))
}

pub(crate) fn contact_aliases(
    keys: &dyn KeyProvider,
    channel: &str,
    destination: &str,
    session: &SessionRow,
) -> Result<Vec<super::crypto::KeyedDigest>, VerificationError> {
    let aliases = contact_fingerprint_aliases(keys, channel, destination)?;
    let stored_version = parse_key_version(&session.5)?;
    let stored_fingerprint_matches = aliases.iter().any(|alias| {
        alias.key_version == stored_version && alias.digest.as_slice() == session.4.as_slice()
    });
    if !stored_fingerprint_matches {
        return Err(VerificationError::InvalidStoredData);
    }
    Ok(aliases)
}

pub(crate) fn normalize_contact(
    channel: ContactChannel,
    email: Option<&str>,
    phone: Option<&str>,
) -> Result<(&'static str, String), VerificationError> {
    match channel {
        ContactChannel::Email => {
            if phone.is_some() {
                return Err(VerificationError::InvalidInput);
            }
            let email = email
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
                || email.len() > 254
            {
                return Err(VerificationError::InvalidInput);
            }
            Ok(("email", email))
        }
        ContactChannel::Phone => {
            if email.is_some() {
                return Err(VerificationError::InvalidInput);
            }
            let phone = phone
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
            Ok(("whatsapp", phone))
        }
    }
}

fn normalized_registration_contact(
    registration: &ClientRegistration<'_>,
) -> Result<NormalizedContact, VerificationError> {
    let (channel, contact) = normalize_contact(
        registration.initial_contact_channel,
        registration.email_normalized,
        registration.phone_e164,
    )?;
    match registration.initial_contact_channel {
        ContactChannel::Email => Ok((channel, contact.clone(), Some(contact), None)),
        ContactChannel::Phone => Ok((channel, contact.clone(), None, Some(contact))),
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

pub(crate) fn destination_associated_data(
    session_id: &str,
    channel: &str,
    purpose: &str,
) -> Vec<u8> {
    format!("identity:verification-destination:v1:{session_id}:{purpose}:{channel}").into_bytes()
}

pub(crate) fn validate_uuid(value: &str) -> Result<(), VerificationError> {
    if value.len() != 36
        || !value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte),
        })
    {
        return Err(VerificationError::InvalidInput);
    }
    Ok(())
}

pub(crate) fn new_uuid() -> Result<String, VerificationError> {
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
            display_name: "Test Owner",
            role: super::super::registration::ClientRole::BusinessOwner,
            initial_contact_channel: ContactChannel::Email,
            email_normalized: Some("  USER@example.test "),
            phone_e164: None,
            password_hash: "encoded-hash",
            terms_of_service_version: "terms-v1",
            privacy_policy_version: "privacy-v1",
            marketing_consent: false,
            marketing_notice_version: None,
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
            display_name: "Test Investor",
            role: super::super::registration::ClientRole::Investor,
            initial_contact_channel: ContactChannel::Phone,
            email_normalized: Some("user@example.test"),
            phone_e164: Some("+628123456789"),
            password_hash: "encoded-hash",
            terms_of_service_version: "terms-v1",
            privacy_policy_version: "privacy-v1",
            marketing_consent: false,
            marketing_notice_version: None,
        };
        assert!(matches!(
            normalized_registration_contact(&registration),
            Err(VerificationError::InvalidInput)
        ));
    }
}
