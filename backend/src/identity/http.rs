use std::{
    future::Future,
    net::{IpAddr, SocketAddr},
    pin::Pin,
    sync::Arc,
};

use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use zeroize::Zeroizing;

use super::{
    crypto::{random_bytes, KeyProvider},
    delivery::OtpDeliveryProvider,
    password::hash_password,
    registration::{ClientRegistration, ClientRole, ContactChannel},
    verification::{VerificationError, VerificationService},
};

const MAX_REQUEST_BYTES: usize = 16 * 1024;
const MAX_DISPLAY_NAME_CHARS: usize = 120;
const GENERIC_REGISTRATION_MESSAGE: &str =
    "Jika pendaftaran dapat diproses, kode verifikasi akan dikirim ke kontak yang dipilih.";
const GENERIC_RESEND_MESSAGE: &str =
    "Jika sesi verifikasi masih berlaku, kode akan dikirim sesuai batas pengiriman.";
const INVALID_CODE_MESSAGE: &str = "Kode tidak valid atau sudah kedaluwarsa.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationAction {
    Create,
    Resend,
    Verify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbuseDecision {
    Allowed,
    Limited,
    Unavailable,
}

/// Guard rate limit yang harus diimplementasikan sebelum router dipasang ke aplikasi publik.
///
/// Alamat IP berasal dari koneksi langsung. Jika aplikasi berada di belakang proxy,
/// integrasi harus memvalidasi proxy tepercaya sebelum meneruskan alamat klien.
pub trait RegistrationAbuseGuard: Send + Sync {
    fn check<'a>(
        &'a self,
        action: RegistrationAction,
        peer_ip: IpAddr,
        subject: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = AbuseDecision> + Send + 'a>>;
}

/// Kebijakan password ditentukan melalui keputusan produk/keamanan dan diinjeksi dari runtime.
pub trait RegistrationPasswordPolicy: Send + Sync {
    fn accepts(&self, password: &str) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationPolicyVersions {
    pub terms_of_service: String,
    pub privacy_policy: String,
    pub marketing_notice: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationApiConfigError {
    InvalidTermsVersion,
    InvalidPrivacyVersion,
    InvalidMarketingVersion,
}

impl RegistrationPolicyVersions {
    pub fn new(
        terms_of_service: String,
        privacy_policy: String,
        marketing_notice: Option<String>,
    ) -> Result<Self, RegistrationApiConfigError> {
        if !valid_version(&terms_of_service) {
            return Err(RegistrationApiConfigError::InvalidTermsVersion);
        }
        if !valid_version(&privacy_policy) {
            return Err(RegistrationApiConfigError::InvalidPrivacyVersion);
        }
        if marketing_notice
            .as_deref()
            .is_some_and(|version| !valid_version(version))
        {
            return Err(RegistrationApiConfigError::InvalidMarketingVersion);
        }

        Ok(Self {
            terms_of_service,
            privacy_policy,
            marketing_notice,
        })
    }
}

fn valid_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.trim() == value
        && !value.chars().any(char::is_whitespace)
}

fn valid_contact_input(
    channel: ContactChannel,
    email: Option<&str>,
    phone: Option<&str>,
) -> bool {
    match channel {
        ContactChannel::Email => match (email, phone) {
            (Some(email), None) => {
                let email = email.trim();
                let mut parts = email.split('@');
                let local = parts.next().unwrap_or_default();
                let domain = parts.next().unwrap_or_default();
                email.len() <= 254
                    && !local.is_empty()
                    && !domain.is_empty()
                    && parts.next().is_none()
                    && !email.chars().any(char::is_whitespace)
            }
            _ => false,
        },
        ContactChannel::Phone => match (email, phone) {
            (None, Some(phone)) => {
                let digits = phone.trim().strip_prefix('+').unwrap_or_default();
                (8..=15).contains(&digits.len())
                    && digits.bytes().all(|byte| byte.is_ascii_digit())
                    && !digits.starts_with('0')
            }
            _ => false,
        },
    }
}

#[derive(Clone)]
pub struct RegistrationApiState {
    pool: PgPool,
    keys: Arc<dyn KeyProvider>,
    delivery: Arc<dyn OtpDeliveryProvider>,
    abuse_guard: Arc<dyn RegistrationAbuseGuard>,
    password_policy: Arc<dyn RegistrationPasswordPolicy>,
    policy_versions: RegistrationPolicyVersions,
}

impl RegistrationApiState {
    pub fn new(
        pool: PgPool,
        keys: Arc<dyn KeyProvider>,
        delivery: Arc<dyn OtpDeliveryProvider>,
        abuse_guard: Arc<dyn RegistrationAbuseGuard>,
        password_policy: Arc<dyn RegistrationPasswordPolicy>,
        policy_versions: RegistrationPolicyVersions,
    ) -> Self {
        Self {
            pool,
            keys,
            delivery,
            abuse_guard,
            password_policy,
            policy_versions,
        }
    }

    fn verification_service(&self) -> VerificationService<'_> {
        VerificationService::new(&self.pool, self.keys.as_ref(), self.delivery.as_ref())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistrationRequest {
    display_name: String,
    role: String,
    contact_channel: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    phone: Option<String>,
    password: String,
    password_confirmation: String,
    terms_accepted: bool,
    privacy_accepted: bool,
    #[serde(default)]
    marketing_consent: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifyRequest {
    code: String,
}

#[derive(Serialize)]
struct RegistrationAccepted {
    message: &'static str,
    verification_id: String,
}

#[derive(Serialize)]
struct MessageResponse {
    message: &'static str,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
}

fn message(status: StatusCode, value: &'static str) -> Response {
    (status, Json(MessageResponse { message: value })).into_response()
}

fn error(status: StatusCode, value: &'static str) -> Response {
    (status, Json(ErrorResponse { error: value })).into_response()
}

fn rate_limit_response(decision: AbuseDecision) -> Option<Response> {
    match decision {
        AbuseDecision::Allowed => None,
        AbuseDecision::Limited => Some(error(
            StatusCode::TOO_MANY_REQUESTS,
            "Terlalu banyak permintaan. Coba lagi nanti.",
        )),
        AbuseDecision::Unavailable => Some(error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Layanan verifikasi sementara tidak tersedia.",
        )),
    }
}

/// Router registrasi yang sengaja tidak dipasang pada `app()` atau `main()`.
///
/// Pemanggil wajib menyediakan key provider, delivery provider, versi kebijakan
/// server-side, password policy, dan abuse guard sebelum router dapat dibuat.
pub fn registration_router(state: RegistrationApiState) -> Router {
    Router::new()
        .route("/api/v1/identity/registrations", post(register))
        .route(
            "/api/v1/identity/registrations/{verification_id}/resend",
            post(resend),
        )
        .route(
            "/api/v1/identity/registrations/{verification_id}/verify",
            post(verify),
        )
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .with_state(state)
}

async fn register(
    State(state): State<RegistrationApiState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(request): Json<RegistrationRequest>,
) -> Response {
    if !request.terms_accepted || !request.privacy_accepted {
        return error(
            StatusCode::BAD_REQUEST,
            "Ketentuan Layanan dan Kebijakan Privasi harus disetujui.",
        );
    }

    let display_name = request.display_name.trim();
    let display_name_len = display_name.chars().count();
    if !(1..=MAX_DISPLAY_NAME_CHARS).contains(&display_name_len) {
        return error(StatusCode::BAD_REQUEST, "Nama tampilan tidak valid.");
    }

    let role = match request.role.as_str() {
        "business_owner" => ClientRole::BusinessOwner,
        "investor" => ClientRole::Investor,
        _ => return error(StatusCode::BAD_REQUEST, "Role pendaftaran tidak valid."),
    };

    let channel = match request.contact_channel.as_str() {
        "email" => ContactChannel::Email,
        "whatsapp" => ContactChannel::Phone,
        _ => return error(StatusCode::BAD_REQUEST, "Kanal verifikasi tidak valid."),
    };
    if !valid_contact_input(channel, request.email.as_deref(), request.phone.as_deref()) {
        return error(StatusCode::BAD_REQUEST, "Kontak pendaftaran tidak valid.");
    }

    let password = Zeroizing::new(request.password);
    let confirmation = Zeroizing::new(request.password_confirmation);
    if password.is_empty() || password.as_str() != confirmation.as_str() {
        return error(StatusCode::BAD_REQUEST, "Password tidak valid atau konfirmasi tidak cocok.");
    }
    if request.marketing_consent && state.policy_versions.marketing_notice.is_none() {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Pilihan marketing belum dapat diproses.",
        );
    }

    let decision = state
        .abuse_guard
        .check(RegistrationAction::Create, peer.ip(), None)
        .await;
    if let Some(response) = rate_limit_response(decision) {
        return response;
    }

    if !state.password_policy.accepts(password.as_str()) {
        return error(StatusCode::BAD_REQUEST, "Password belum memenuhi ketentuan keamanan.");
    }

    let password_hash = match tokio::task::spawn_blocking(move || hash_password(password.as_bytes()))
        .await
    {
        Ok(Ok(hash)) => hash,
        _ => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Pendaftaran sementara tidak dapat diproses.",
            );
        }
    };

    let registration = ClientRegistration {
        display_name,
        role,
        initial_contact_channel: channel,
        email_normalized: request.email.as_deref(),
        phone_e164: request.phone.as_deref(),
        password_hash: &password_hash,
        terms_of_service_version: &state.policy_versions.terms_of_service,
        privacy_policy_version: &state.policy_versions.privacy_policy,
        marketing_consent: request.marketing_consent,
        marketing_notice_version: if request.marketing_consent {
            state.policy_versions.marketing_notice.as_deref()
        } else {
            None
        },
    };

    match state.verification_service().register(&registration).await {
        Ok(start) => (
            StatusCode::ACCEPTED,
            Json(RegistrationAccepted {
                message: GENERIC_REGISTRATION_MESSAGE,
                verification_id: start.session_id,
            }),
        )
            .into_response(),
        Err(VerificationError::Database(sqlx::Error::Database(database_error)))
            if database_error.code().as_deref() == Some("23505") =>
        {
            match opaque_verification_id() {
                Ok(verification_id) => (
                    StatusCode::ACCEPTED,
                    Json(RegistrationAccepted {
                        message: GENERIC_REGISTRATION_MESSAGE,
                        verification_id,
                    }),
                )
                    .into_response(),
                Err(_) => error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Pendaftaran sementara tidak dapat diproses.",
                ),
            }
        }
        Err(VerificationError::InvalidInput) => {
            error(StatusCode::BAD_REQUEST, "Data pendaftaran tidak valid.")
        }
        Err(_) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Pendaftaran sementara tidak dapat diproses.",
        ),
    }
}

async fn resend(
    State(state): State<RegistrationApiState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(verification_id): Path<String>,
) -> Response {
    let decision = state
        .abuse_guard
        .check(
            RegistrationAction::Resend,
            peer.ip(),
            Some(&verification_id),
        )
        .await;
    if let Some(response) = rate_limit_response(decision) {
        return response;
    }

    match state.verification_service().resend(&verification_id).await {
        Ok(_) | Err(VerificationError::InvalidInput) => {
            message(StatusCode::ACCEPTED, GENERIC_RESEND_MESSAGE)
        }
        Err(VerificationError::Database(sqlx::Error::RowNotFound)) => {
            message(StatusCode::ACCEPTED, GENERIC_RESEND_MESSAGE)
        }
        Err(_) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Layanan verifikasi sementara tidak tersedia.",
        ),
    }
}

async fn verify(
    State(state): State<RegistrationApiState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(verification_id): Path<String>,
    Json(request): Json<VerifyRequest>,
) -> Response {
    let decision = state
        .abuse_guard
        .check(
            RegistrationAction::Verify,
            peer.ip(),
            Some(&verification_id),
        )
        .await;
    if let Some(response) = rate_limit_response(decision) {
        return response;
    }

    match state
        .verification_service()
        .verify(&verification_id, &request.code)
        .await
    {
        Ok(true) => message(
            StatusCode::OK,
            "Verifikasi berhasil. Akun sudah aktif dan dapat digunakan untuk masuk.",
        ),
        Ok(false) | Err(VerificationError::InvalidInput) => {
            error(StatusCode::BAD_REQUEST, INVALID_CODE_MESSAGE)
        }
        Err(VerificationError::Database(sqlx::Error::RowNotFound)) => {
            error(StatusCode::BAD_REQUEST, INVALID_CODE_MESSAGE)
        }
        Err(_) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Layanan verifikasi sementara tidak tersedia.",
        ),
    }
}

fn opaque_verification_id() -> Result<String, super::crypto::CryptoError> {
    let mut bytes = random_bytes::<16>()?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_versions_reject_empty_whitespace_and_oversized_labels() {
        assert_eq!(
            RegistrationPolicyVersions::new(String::new(), "privacy-v1".to_owned(), None),
            Err(RegistrationApiConfigError::InvalidTermsVersion)
        );
        assert_eq!(
            RegistrationPolicyVersions::new(
                "terms-v1".to_owned(),
                "privacy v1".to_owned(),
                None
            ),
            Err(RegistrationApiConfigError::InvalidPrivacyVersion)
        );
        assert_eq!(
            RegistrationPolicyVersions::new(
                "terms-v1".to_owned(),
                "privacy-v1".to_owned(),
                Some("m".repeat(65))
            ),
            Err(RegistrationApiConfigError::InvalidMarketingVersion)
        );
    }

    #[test]
    fn policy_versions_accept_server_managed_labels() {
        let versions = RegistrationPolicyVersions::new(
            "terms-2026-12-v1".to_owned(),
            "privacy-2026-12-v1".to_owned(),
            Some("marketing-2026-12-v1".to_owned()),
        )
        .expect("version labels should be valid");

        assert_eq!(versions.terms_of_service, "terms-2026-12-v1");
        assert_eq!(versions.privacy_policy, "privacy-2026-12-v1");
        assert_eq!(
            versions.marketing_notice.as_deref(),
            Some("marketing-2026-12-v1")
        );
    }

    #[test]
    fn contact_input_must_match_the_selected_channel() {
        assert!(valid_contact_input(
            ContactChannel::Email,
            Some(" user@example.test "),
            None
        ));
        assert!(!valid_contact_input(
            ContactChannel::Email,
            Some("user@example.test"),
            Some("+628123456789")
        ));
        assert!(valid_contact_input(
            ContactChannel::Phone,
            None,
            Some("+628123456789")
        ));
        assert!(!valid_contact_input(
            ContactChannel::Phone,
            None,
            Some("08123456789")
        ));
    }

    #[test]
    fn opaque_verification_ids_are_uuid_v4_shaped() {
        let value = opaque_verification_id().expect("opaque ID generation should succeed");
        assert_eq!(value.len(), 36);
        assert_eq!(value.as_bytes()[14], b'4');
        assert!(matches!(value.as_bytes()[19], b'8'..=b'b'));
    }
}
