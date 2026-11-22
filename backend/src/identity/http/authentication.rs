use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::net::SocketAddr;
use zeroize::Zeroizing;

use super::super::authentication::{
    AuthenticationError, AuthenticationRateLimitKey, AuthenticationService, LoginRole,
};

const MAX_REQUEST_BYTES: usize = 8 * 1024;
const SESSION_COOKIE: &str = "__Host-usahapintar_session";
const SESSION_MAX_AGE_SECONDS: u32 = 30 * 24 * 60 * 60;

#[derive(Clone)]
pub struct AuthenticationApiState {
    pool: PgPool,
    rate_limit_key: AuthenticationRateLimitKey,
}

impl AuthenticationApiState {
    pub fn new(pool: PgPool, rate_limit_key: AuthenticationRateLimitKey) -> Self {
        Self {
            pool,
            rate_limit_key,
        }
    }

    fn service(&self) -> AuthenticationService<'_> {
        AuthenticationService::new(&self.pool, &self.rate_limit_key)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginRequest {
    identifier: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    message: &'static str,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
}

fn error(status: StatusCode, message: &'static str) -> Response {
    (status, Json(ErrorResponse { error: message })).into_response()
}

fn login_error(error_value: AuthenticationError) -> Response {
    match error_value {
        AuthenticationError::InvalidCredentials => {
            error(StatusCode::UNAUTHORIZED, "Email/nomor telepon atau password salah.")
        }
        AuthenticationError::RateLimited => error(
            StatusCode::TOO_MANY_REQUESTS,
            "Terlalu banyak percobaan login. Coba lagi dalam beberapa menit.",
        ),
        AuthenticationError::InvalidInput => {
            error(StatusCode::BAD_REQUEST, "Data login tidak valid.")
        }
        AuthenticationError::Database | AuthenticationError::Internal => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Layanan login sementara tidak tersedia.",
        ),
    }
}

pub fn authentication_router(state: AuthenticationApiState) -> Router {
    Router::new()
        .route("/api/v1/identity/login/owner", post(login_owner))
        .route("/api/v1/identity/login/investor", post(login_investor))
        .route("/api/v1/identity/logout", post(logout))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .with_state(state)
}

async fn login_owner(
    State(state): State<AuthenticationApiState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(request): Json<LoginRequest>,
) -> Response {
    login(state, peer, request, LoginRole::BusinessOwner).await
}

async fn login_investor(
    State(state): State<AuthenticationApiState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(request): Json<LoginRequest>,
) -> Response {
    login(state, peer, request, LoginRole::Investor).await
}

async fn login(
    state: AuthenticationApiState,
    peer: SocketAddr,
    request: LoginRequest,
    role: LoginRole,
) -> Response {
    let password = Zeroizing::new(request.password);
    match state
        .service()
        .login(&request.identifier, password.as_str(), role, peer.ip())
        .await
    {
        Ok(session) => {
            let cookie = format!(
                "{SESSION_COOKIE}={}; Path=/; Max-Age={SESSION_MAX_AGE_SECONDS}; HttpOnly; Secure; SameSite=Lax",
                session.token
            );
            let Ok(cookie_value) = HeaderValue::from_str(&cookie) else {
                return error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Layanan login sementara tidak tersedia.",
                );
            };
            let mut response = (
                StatusCode::OK,
                Json(LoginResponse {
                    message: "Login berhasil.",
                }),
            )
                .into_response();
            response.headers_mut().insert(header::SET_COOKIE, cookie_value);
            response
        }
        Err(authentication_error) => login_error(authentication_error),
    }
}

async fn logout(
    State(state): State<AuthenticationApiState>,
    headers: HeaderMap,
) -> Response {
    let token = session_cookie(&headers);
    if let Some(token) = token {
        if state.service().logout(&token).await.is_err() {
            let mut response = error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Sesi lokal dibersihkan, tetapi pencabutan sesi belum dapat dikonfirmasi.",
            );
            clear_session_cookie(&mut response);
            return response;
        }
    }

    let mut response = StatusCode::NO_CONTENT.into_response();
    clear_session_cookie(&mut response);
    response
}

fn session_cookie(headers: &HeaderMap) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    cookies.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        if name != SESSION_COOKIE
            || value.len() != 64
            || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return None;
        }
        Some(value.to_owned())
    })
}

fn clear_session_cookie(response: &mut Response) {
    let cookie = format!(
        "{SESSION_COOKIE}=; Path=/; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT; HttpOnly; Secure; SameSite=Lax"
    );
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_parser_accepts_only_the_expected_opaque_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("theme=dark; __Host-usahapintar_session=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
        );
        assert_eq!(
            session_cookie(&headers).as_deref(),
            Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
        );

        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("__Host-usahapintar_session=not-a-token"),
        );
        assert!(session_cookie(&headers).is_none());
    }

    #[test]
    fn logout_cookie_is_host_only_secure_and_immediately_expired() {
        let mut response = StatusCode::NO_CONTENT.into_response();
        clear_session_cookie(&mut response);
        let cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .expect("logout should clear the session cookie")
            .to_str()
            .expect("cookie should be valid header text");
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("Max-Age=0"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(!cookie.to_ascii_lowercase().contains("domain="));
    }
}
