use axum::{
    extract::{ConnectInfo, Json, State},
    http::{HeaderMap, Response, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;
use zeroize::Zeroizing;

use crate::identity::{
    authentication::session_token_hash,
    password_reset::{encode_hex, PasswordResetError, PasswordResetService},
};

use super::{
    authentication::{
        clear_session_cookie, is_cross_site_request, resolved_client_ip,
    },
    recovery::{error, message, no_store, rate_limit_response},
    AbuseDecision, RecoveryApiState, RegistrationAction,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PasswordResetRequest {
    token: String,
    new_password: String,
    confirm_password: String,
}

pub(super) async fn complete(
    State(state): State<RecoveryApiState>,
    ConnectInfo(connect_info): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    payload: Result<Json<PasswordResetRequest>, axum::extract::rejection::JsonRejection>,
) -> Response<axum::body::Body> {
    if is_cross_site_request(&headers) {
        return secure_error(
            StatusCode::FORBIDDEN,
            "Permintaan lintas situs tidak diizinkan.",
        );
    }

    let Some(policy) = state.password_policy.as_ref() else {
        return secure_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Layanan reset password belum tersedia.",
        );
    };

    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(_) => {
            return secure_error(
                StatusCode::BAD_REQUEST,
                "Permintaan reset password tidak valid.",
            );
        }
    };

    let token = Zeroizing::new(request.token);
    let new_password = Zeroizing::new(request.new_password);
    let confirm_password = Zeroizing::new(request.confirm_password);

    if new_password.as_str() != confirm_password.as_str() {
        return secure_error(
            StatusCode::BAD_REQUEST,
            "Password baru tidak cocok atau tidak memenuhi kebijakan keamanan.",
        );
    }

    let token_hash = match session_token_hash(&token) {
        Ok(hash) => hash,
        Err(_) => {
            return secure_error(
                StatusCode::BAD_REQUEST,
                "Token reset tidak valid atau kedaluwarsa.",
            );
        }
    };

    let token_fingerprint = encode_hex(&token_hash);
    let client_ip = resolved_client_ip(&headers, &connect_info.0);
    let decision = state
        .abuse_guard
        .check(
            RegistrationAction::RecoveryPasswordResetComplete,
            &client_ip,
            Some(&token_fingerprint),
        )
        .await;

    if let Some(mut response) = rate_limit_response(decision) {
        no_store(&mut response);
        return response;
    }
    if decision == AbuseDecision::Deny {
        return secure_error(
            StatusCode::FORBIDDEN,
            "Permintaan reset password tidak diizinkan.",
        );
    }

    match PasswordResetService::new(&state.pool)
        .complete(&token, &new_password, policy.as_ref())
        .await
    {
        Ok(_) => {
            let mut response = message(
                StatusCode::OK,
                "Password berhasil diperbarui. Silakan masuk kembali.",
            );
            clear_session_cookie(&mut response);
            no_store(&mut response);
            response
        }
        Err(PasswordResetError::PasswordPolicyRejected) => secure_error(
            StatusCode::BAD_REQUEST,
            "Password baru tidak memenuhi kebijakan keamanan.",
        ),
        Err(PasswordResetError::InvalidInput) => secure_error(
            StatusCode::BAD_REQUEST,
            "Token reset tidak valid atau kedaluwarsa.",
        ),
        Err(PasswordResetError::Database(error))
            if database_error_code(&error, "P0001") =>
        {
            secure_error(
                StatusCode::TOO_MANY_REQUESTS,
                "Terlalu banyak permintaan. Coba lagi nanti.",
            )
        }
        Err(PasswordResetError::Database(error))
            if database_error_code(&error, "P0002")
                || matches!(error, sqlx::Error::RowNotFound) =>
        {
            secure_error(
                StatusCode::BAD_REQUEST,
                "Token reset tidak valid atau kedaluwarsa.",
            )
        }
        Err(_) => secure_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Reset password belum dapat diproses.",
        ),
    }
}

fn database_error_code(error: &sqlx::Error, expected: &str) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some(expected)
    )
}

fn secure_error(status: StatusCode, text: &'static str) -> Response<axum::body::Body> {
    let mut response = error(status, text);
    no_store(&mut response);
    response
}

impl IntoResponse for PasswordResetError {
    fn into_response(self) -> axum::response::Response {
        secure_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Reset password belum dapat diproses.",
        )
    }
}
