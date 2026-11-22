use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use super::super::{
    recovery::{RecoveryAction, RecoveryCandidate},
    registration::ContactChannel,
    verification::VerificationError,
};
use super::{
    authentication::{is_cross_site_request, resolved_client_ip, session_cookie},
    valid_contact_input, AbuseDecision, RecoveryApiState, RegistrationAction,
};

const MAX_REQUEST_BYTES: usize = 8 * 1024;
const ACCEPTED_START_MESSAGE: &str =
    "Jika kontak dapat diproses, kode verifikasi akan dikirim ke kontak yang dipilih.";
const ACCEPTED_RESEND_MESSAGE: &str =
    "Jika sesi pemulihan masih berlaku, kode akan dikirim sesuai batas pengiriman.";
const INVALID_CODE_MESSAGE: &str = "Kode tidak valid atau sudah kedaluwarsa.";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryStartRequest {
    contact_channel: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    phone: Option<String>,
    requested_action: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifyRequest {
    code: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectAccountRequest {
    user_id: String,
}

#[derive(Serialize)]
struct RecoveryAccepted {
    message: &'static str,
    verification_id: String,
    recovery_case_id: String,
}

#[derive(Serialize)]
struct MessageResponse {
    message: &'static str,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
}

#[derive(Serialize)]
struct CandidateResponse {
    accounts: Vec<RecoveryCandidateItem>,
}

#[derive(Serialize)]
struct RecoveryCandidateItem {
    user_id: String,
    role: String,
    display_name: String,
}

impl From<RecoveryCandidate> for RecoveryCandidateItem {
    fn from(candidate: RecoveryCandidate) -> Self {
        Self {
            user_id: candidate.user_id,
            role: candidate.role,
            display_name: candidate.display_name,
        }
    }
}

fn message(status: StatusCode, value: &'static str) -> Response {
    let mut response = (status, Json(MessageResponse { message: value })).into_response();
    no_store(&mut response);
    response
}

fn error(status: StatusCode, value: &'static str) -> Response {
    let mut response = (status, Json(ErrorResponse { error: value })).into_response();
    no_store(&mut response);
    response
}

fn no_store(response: &mut Response) {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
}

fn database_error_code(error_value: &VerificationError, expected: &str) -> bool {
    matches!(
        error_value,
        VerificationError::Database(sqlx::Error::Database(database_error))
            if database_error.code().as_deref() == Some(expected)
    )
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
            "Layanan pemulihan sementara tidak tersedia.",
        )),
    }
}

fn recovery_error(error_value: VerificationError) -> Response {
    match error_value {
        VerificationError::InvalidInput => {
            error(StatusCode::BAD_REQUEST, "Data pemulihan tidak valid.")
        }
        VerificationError::Database(sqlx::Error::Database(database_error))
            if database_error.code().as_deref() == Some("P0001") =>
        {
            error(
                StatusCode::TOO_MANY_REQUESTS,
                "Terlalu banyak percobaan. Coba lagi nanti.",
            )
        }
        VerificationError::Database(sqlx::Error::Database(database_error))
            if database_error.code().as_deref() == Some("22023") =>
        {
            error(StatusCode::BAD_REQUEST, "Data pemulihan tidak valid.")
        }
        _ => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Layanan pemulihan sementara tidak tersedia.",
        ),
    }
}

fn channel_from_request(value: &str) -> Option<ContactChannel> {
    match value {
        "email" => Some(ContactChannel::Email),
        "whatsapp" => Some(ContactChannel::Phone),
        _ => None,
    }
}

fn action_from_request(value: &str) -> Option<RecoveryAction> {
    match value {
        "password_reset" => Some(RecoveryAction::PasswordReset),
        "forgot_email" => Some(RecoveryAction::ForgotEmail),
        "account_access" => Some(RecoveryAction::AccountAccess),
        _ => None,
    }
}

pub fn recovery_router(state: RecoveryApiState) -> Router {
    Router::new()
        .route("/api/v1/identity/recovery", post(start_recovery))
        .route(
            "/api/v1/identity/recovery/{verification_id}/resend",
            post(resend_recovery),
        )
        .route(
            "/api/v1/identity/recovery/{verification_id}/verify",
            post(verify_recovery),
        )
        .route(
            "/api/v1/identity/recovery/cases/{case_id}/accounts",
            get(list_candidates),
        )
        .route(
            "/api/v1/identity/recovery/cases/{case_id}/select",
            post(select_account),
        )
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .with_state(state)
}

async fn start_recovery(
    State(state): State<RecoveryApiState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<RecoveryStartRequest>,
) -> Response {
    if is_cross_site_request(&headers) {
        return error(
            StatusCode::FORBIDDEN,
            "Permintaan pemulihan lintas situs ditolak.",
        );
    }

    let decision = state
        .abuse_guard
        .check(
            RegistrationAction::RecoveryStart,
            resolved_client_ip(&headers, peer.ip()),
            None,
        )
        .await;
    if let Some(response) = rate_limit_response(decision) {
        return response;
    }

    let Some(channel) = channel_from_request(&request.contact_channel) else {
        return error(StatusCode::BAD_REQUEST, "Kanal kontak tidak valid.");
    };
    if !valid_contact_input(channel, request.email.as_deref(), request.phone.as_deref()) {
        return error(StatusCode::BAD_REQUEST, "Kontak pemulihan tidak valid.");
    }
    let Some(action) = action_from_request(&request.requested_action) else {
        return error(StatusCode::BAD_REQUEST, "Tujuan pemulihan tidak valid.");
    };

    match state
        .recovery_service()
        .start(
            channel,
            request.email.as_deref(),
            request.phone.as_deref(),
            action,
        )
        .await
    {
        Ok(started) => {
            let mut response = (
                StatusCode::ACCEPTED,
                Json(RecoveryAccepted {
                    message: ACCEPTED_START_MESSAGE,
                    verification_id: started.session_id,
                    recovery_case_id: started.case_id,
                }),
            )
                .into_response();
            no_store(&mut response);
            response
        }
        Err(error_value) => recovery_error(error_value),
    }
}

async fn resend_recovery(
    State(state): State<RecoveryApiState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(verification_id): Path<String>,
) -> Response {
    if is_cross_site_request(&headers) {
        return error(
            StatusCode::FORBIDDEN,
            "Permintaan pengiriman ulang lintas situs ditolak.",
        );
    }

    let decision = state
        .abuse_guard
        .check(
            RegistrationAction::RecoveryResend,
            resolved_client_ip(&headers, peer.ip()),
            Some(&verification_id),
        )
        .await;
    if let Some(response) = rate_limit_response(decision) {
        return response;
    }

    match state.recovery_service().resend(&verification_id).await {
        Ok(_) | Err(VerificationError::InvalidInput) => {
            message(StatusCode::ACCEPTED, ACCEPTED_RESEND_MESSAGE)
        }
        Err(VerificationError::Database(sqlx::Error::RowNotFound)) => {
            message(StatusCode::ACCEPTED, ACCEPTED_RESEND_MESSAGE)
        }
        Err(error_value) if database_error_code(&error_value, "P0002") => {
            message(StatusCode::ACCEPTED, ACCEPTED_RESEND_MESSAGE)
        }
        Err(error_value) => recovery_error(error_value),
    }
}

async fn verify_recovery(
    State(state): State<RecoveryApiState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(verification_id): Path<String>,
    Json(request): Json<VerifyRequest>,
) -> Response {
    if is_cross_site_request(&headers) {
        return error(
            StatusCode::FORBIDDEN,
            "Permintaan verifikasi lintas situs ditolak.",
        );
    }

    let decision = state
        .abuse_guard
        .check(
            RegistrationAction::RecoveryVerify,
            resolved_client_ip(&headers, peer.ip()),
            Some(&verification_id),
        )
        .await;
    if let Some(response) = rate_limit_response(decision) {
        return response;
    }

    match state
        .recovery_service()
        .verify(&verification_id, &request.code)
        .await
    {
        Ok(true) => message(
            StatusCode::OK,
            "Kontak berhasil diverifikasi. Silakan pilih akun yang ingin dipulihkan.",
        ),
        Ok(false) | Err(VerificationError::InvalidInput) => {
            error(StatusCode::BAD_REQUEST, INVALID_CODE_MESSAGE)
        }
        Err(VerificationError::Database(sqlx::Error::RowNotFound)) => {
            error(StatusCode::BAD_REQUEST, INVALID_CODE_MESSAGE)
        }
        Err(error_value) if database_error_code(&error_value, "P0002") => {
            error(StatusCode::BAD_REQUEST, INVALID_CODE_MESSAGE)
        }
        Err(error_value) => recovery_error(error_value),
    }
}

async fn list_candidates(
    State(state): State<RecoveryApiState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(case_id): Path<String>,
) -> Response {
    if is_cross_site_request(&headers) {
        return error(
            StatusCode::FORBIDDEN,
            "Permintaan daftar akun lintas situs ditolak.",
        );
    }

    let decision = state
        .abuse_guard
        .check(
            RegistrationAction::RecoveryCandidates,
            resolved_client_ip(&headers, peer.ip()),
            Some(&case_id),
        )
        .await;
    if let Some(response) = rate_limit_response(decision) {
        return response;
    }

    match state.recovery_service().list_candidates(&case_id).await {
        Ok(candidates) => {
            let mut response = (
                StatusCode::OK,
                Json(CandidateResponse {
                    accounts: candidates.into_iter().map(Into::into).collect(),
                }),
            )
                .into_response();
            no_store(&mut response);
            response
        }
        Err(VerificationError::InvalidInput)
        | Err(VerificationError::Database(sqlx::Error::RowNotFound)) => {
            error(StatusCode::NOT_FOUND, "Kasus pemulihan tidak ditemukan.")
        }
        Err(error_value) if database_error_code(&error_value, "P0002") => {
            error(StatusCode::NOT_FOUND, "Kasus pemulihan tidak ditemukan.")
        }
        Err(error_value) => recovery_error(error_value),
    }
}

async fn select_account(
    State(state): State<RecoveryApiState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(case_id): Path<String>,
    Json(request): Json<SelectAccountRequest>,
) -> Response {
    if is_cross_site_request(&headers) {
        return error(
            StatusCode::FORBIDDEN,
            "Permintaan pemilihan akun lintas situs ditolak.",
        );
    }

    let decision = state
        .abuse_guard
        .check(
            RegistrationAction::RecoverySelect,
            resolved_client_ip(&headers, peer.ip()),
            Some(&case_id),
        )
        .await;
    if let Some(response) = rate_limit_response(decision) {
        return response;
    }

    let session_token = session_cookie(&headers);
    let session_token = session_token.as_ref().map(|token| token.as_str());

    match state
        .recovery_service()
        .select_account_with_session(&case_id, &request.user_id, session_token)
        .await
    {
        Ok(_) => message(
            StatusCode::ACCEPTED,
            "Permintaan pemulihan diteruskan untuk pemeriksaan keamanan.",
        ),
        Err(VerificationError::InvalidInput)
        | Err(VerificationError::Database(sqlx::Error::RowNotFound)) => {
            error(StatusCode::BAD_REQUEST, "Pilihan akun tidak valid.")
        }
        Err(error_value)
            if database_error_code(&error_value, "P0001")
                || database_error_code(&error_value, "P0002") =>
        {
            error(StatusCode::BAD_REQUEST, "Pilihan akun tidak valid.")
        }
        Err(error_value) => recovery_error(error_value),
    }
}
