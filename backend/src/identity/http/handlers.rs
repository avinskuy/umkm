use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::{
    opaque_verification_id, valid_contact_input, AbuseDecision, RegistrationAction,
    RegistrationApiState,
};
use super::super::{
    password::hash_password,
    registration::{ClientRegistration, ClientRole, ContactChannel},
    verification::VerificationError,
};
use super::{
    opaque_verification_id, valid_contact_input, AbuseDecision, RegistrationAction,
    RegistrationApiState,
};

const MAX_REQUEST_BYTES: usize = 16 * 1024;
const MAX_DISPLAY_NAME_CHARS: usize = 120;
const GENERIC_REGISTRATION_MESSAGE: &str =
    "Jika pendaftaran dapat diproses, kode verifikasi akan dikirim ke kontak yang dipilih.";
const GENERIC_RESEND_MESSAGE: &str =
    "Jika sesi verifikasi masih berlaku, kode akan dikirim sesuai batas pengiriman.";
const INVALID_CODE_MESSAGE: &str = "Kode tidak valid atau sudah kedaluwarsa.";

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
    let decision = state
        .abuse_guard
        .check(RegistrationAction::Create, peer.ip(), None)
        .await;
    if let Some(response) = rate_limit_response(decision) {
        return response;
    }

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
        return error(
            StatusCode::BAD_REQUEST,
            "Password tidak valid atau konfirmasi tidak cocok.",
        );
    }
    if request.marketing_consent && state.policy_versions.marketing_notice.is_none() {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Pilihan marketing belum dapat diproses.",
        );
    }

    if !state.password_policy.accepts(password.as_str()) {
        return error(
            StatusCode::BAD_REQUEST,
            "Password belum memenuhi ketentuan keamanan.",
        );
    }

    let password_hash =
        match tokio::task::spawn_blocking(move || hash_password(password.as_bytes())).await {
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
