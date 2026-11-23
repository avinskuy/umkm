use std::{
    net::SocketAddr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, Extension, Json, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use zeroize::Zeroizing;

use super::authentication::resolved_client_ip;
use crate::identity::{
    crypto::KeyProvider, AdminAuthenticatedSession, AdminAuthenticationError,
    AdminAuthenticationService, AuthenticationRateLimitKey,
};

const MAX_REQUEST_BYTES: usize = 8 * 1024;
const ADMIN_SESSION_COOKIE: &str = "__Host-usahapintar_admin_session";
const ADMIN_CHALLENGE_COOKIE: &str = "__Host-usahapintar_admin_challenge";
const ADMIN_SESSION_MAX_AGE_SECONDS: u32 = 8 * 60 * 60;
const ADMIN_CHALLENGE_MAX_AGE_SECONDS: i64 = 5 * 60;

#[derive(Clone)]
pub struct AdminAuthenticationApiState {
    pool: PgPool,
    rate_limit_key: AuthenticationRateLimitKey,
    key_provider: Arc<dyn KeyProvider>,
}

impl AdminAuthenticationApiState {
    pub fn new(
        pool: PgPool,
        rate_limit_key: AuthenticationRateLimitKey,
        key_provider: Arc<dyn KeyProvider>,
    ) -> Self {
        Self {
            pool,
            rate_limit_key,
            key_provider,
        }
    }

    fn service(&self) -> AdminAuthenticationService<'_> {
        AdminAuthenticationService::new(
            &self.pool,
            &self.rate_limit_key,
            self.key_provider.as_ref(),
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginRequest {
    identifier: String,
    password: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TotpRequest {
    code: String,
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
struct CurrentAdminResponse {
    user_id: String,
    role: &'static str,
}

fn response(status: StatusCode, body: impl IntoResponse) -> Response {
    let mut response = (status, body).into_response();
    no_store(&mut response);
    response
}

fn error(status: StatusCode, message: &'static str) -> Response {
    response(status, Json(ErrorResponse { error: message }))
}

fn no_store(response: &mut Response) {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
}

fn authentication_error(error_value: AdminAuthenticationError) -> Response {
    match error_value {
        AdminAuthenticationError::InvalidCredentials
        | AdminAuthenticationError::InvalidChallenge
        | AdminAuthenticationError::InvalidSession => {
            error(StatusCode::UNAUTHORIZED, "Autentikasi Admin tidak valid.")
        }
        AdminAuthenticationError::RateLimited => error(
            StatusCode::TOO_MANY_REQUESTS,
            "Terlalu banyak percobaan. Coba lagi beberapa menit.",
        ),
        AdminAuthenticationError::InvalidInput => error(
            StatusCode::BAD_REQUEST,
            "Permintaan autentikasi Admin tidak valid.",
        ),
        AdminAuthenticationError::Database | AdminAuthenticationError::Internal => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Layanan autentikasi Admin sementara tidak tersedia.",
        ),
    }
}

/// Require an exact same-origin Origin header for browser state-changing requests.
/// HTTP is allowed only for loopback development hosts; production cookies require Secure.
fn is_same_origin_request(headers: &HeaderMap) -> bool {
    if headers
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case("cross-site"))
    {
        return false;
    }

    let Some(host) = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let Some(origin) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let Some((scheme, remainder)) = origin.split_once("://") else {
        return false;
    };
    let authority = remainder.split('/').next().unwrap_or_default();
    if authority.is_empty() || !same_authority(authority, host, scheme) {
        return false;
    }

    scheme.eq_ignore_ascii_case("https")
        || (scheme.eq_ignore_ascii_case("http") && is_loopback_authority(authority))
}

fn same_authority(origin_authority: &str, request_host: &str, scheme: &str) -> bool {
    if origin_authority.eq_ignore_ascii_case(request_host) {
        return true;
    }

    let default_port = if scheme.eq_ignore_ascii_case("https") {
        Some("443")
    } else if scheme.eq_ignore_ascii_case("http") {
        Some("80")
    } else {
        None
    };
    let Some(default_port) = default_port else {
        return false;
    };

    format!("{origin_authority}:{default_port}").eq_ignore_ascii_case(request_host)
        || format!("{request_host}:{default_port}").eq_ignore_ascii_case(origin_authority)
}

fn is_loopback_authority(authority: &str) -> bool {
    authority.eq_ignore_ascii_case("localhost")
        || authority
            .get(..10)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("localhost:"))
        || authority == "127.0.0.1"
        || authority.starts_with("127.0.0.1:")
        || authority == "[::1]"
        || authority.starts_with("[::1]:")
}

pub fn admin_authentication_router(state: AdminAuthenticationApiState) -> Router {
    let public_routes = Router::new()
        .route("/api/v1/identity/admin/login", post(begin_login))
        .route("/api/v1/identity/admin/login/totp", post(complete_login))
        .route("/api/v1/identity/admin/logout", post(logout))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .with_state(state.clone());

    let protected_routes = Router::new()
        .route("/api/v1/identity/admin/me", get(current_admin))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_admin_session,
        ))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .with_state(state);

    public_routes.merge(protected_routes)
}

async fn begin_login(
    State(state): State<AdminAuthenticationApiState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    payload: Result<Json<LoginRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if !is_same_origin_request(&headers) {
        return error(
            StatusCode::FORBIDDEN,
            "Permintaan lintas situs tidak diizinkan.",
        );
    }

    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(_) => return error(StatusCode::BAD_REQUEST, "Data login Admin tidak valid."),
    };
    let identifier = Zeroizing::new(request.identifier);
    let password = Zeroizing::new(request.password);
    match state
        .service()
        .begin_login(
            identifier.as_str(),
            password.as_str(),
            resolved_client_ip(&headers, peer.ip()),
        )
        .await
    {
        Ok(challenge) => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or_default();
            let max_age = challenge
                .expires_at_unix
                .saturating_sub(now)
                .clamp(1, ADMIN_CHALLENGE_MAX_AGE_SECONDS);
            let cookie = admin_challenge_cookie_header(challenge.token.as_str(), max_age);
            let Ok(cookie_value) = HeaderValue::from_str(cookie.as_str()) else {
                return error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Layanan autentikasi Admin sementara tidak tersedia.",
                );
            };
            let mut response = (
                StatusCode::ACCEPTED,
                Json(MessageResponse {
                    message: "Password diterima. Masukkan kode authenticator Admin.",
                }),
            )
                .into_response();
            response
                .headers_mut()
                .insert(header::SET_COOKIE, cookie_value);
            no_store(&mut response);
            response
        }
        Err(error_value) => {
            let mut response = authentication_error(error_value);
            clear_admin_challenge_cookie(&mut response);
            response
        }
    }
}

async fn complete_login(
    State(state): State<AdminAuthenticationApiState>,
    headers: HeaderMap,
    payload: Result<Json<TotpRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if !is_same_origin_request(&headers) {
        return error(
            StatusCode::FORBIDDEN,
            "Permintaan lintas situs tidak diizinkan.",
        );
    }

    let Some(challenge_token) = admin_challenge_cookie(&headers) else {
        return error(
            StatusCode::UNAUTHORIZED,
            "Challenge Admin tidak valid atau kedaluwarsa.",
        );
    };
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(_) => return error(StatusCode::BAD_REQUEST, "Kode authenticator tidak valid."),
    };
    let code = Zeroizing::new(request.code);

    match state
        .service()
        .complete_login(challenge_token.as_str(), code.as_str())
        .await
    {
        Ok(session) => {
            let session_cookie = admin_session_cookie_header(session.token.as_str());
            let Ok(session_cookie_value) = HeaderValue::from_str(session_cookie.as_str()) else {
                return error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Layanan sesi Admin sementara tidak tersedia.",
                );
            };

            let mut response = (
                StatusCode::OK,
                Json(MessageResponse {
                    message: "Login Admin berhasil.",
                }),
            )
                .into_response();
            response
                .headers_mut()
                .append(header::SET_COOKIE, session_cookie_value);
            clear_admin_challenge_cookie(&mut response);
            no_store(&mut response);
            response
        }
        Err(error_value) => authentication_error(error_value),
    }
}

async fn require_admin_session(
    State(state): State<AdminAuthenticationApiState>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(token) = admin_session_cookie(request.headers()) else {
        return unauthorized_admin_session();
    };

    match state.service().validate_session(token.as_str()).await {
        Ok(session) => {
            request.extensions_mut().insert(session);
            next.run(request).await
        }
        Err(AdminAuthenticationError::InvalidSession) => unauthorized_admin_session(),
        Err(AdminAuthenticationError::Database | AdminAuthenticationError::Internal) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Layanan sesi Admin sementara tidak tersedia.",
        ),
        Err(_) => unauthorized_admin_session(),
    }
}

async fn current_admin(Extension(session): Extension<AdminAuthenticatedSession>) -> Response {
    response(
        StatusCode::OK,
        Json(CurrentAdminResponse {
            user_id: session.user_id,
            role: "admin",
        }),
    )
}

async fn logout(State(state): State<AdminAuthenticationApiState>, headers: HeaderMap) -> Response {
    if !is_same_origin_request(&headers) {
        return error(
            StatusCode::FORBIDDEN,
            "Permintaan lintas situs tidak diizinkan.",
        );
    }

    if let Some(token) = admin_session_cookie(&headers) {
        if let Err(error_value) = state.service().logout(token.as_str()).await {
            return authentication_error(error_value);
        }
    }

    let mut response = StatusCode::NO_CONTENT.into_response();
    clear_admin_cookies(&mut response);
    no_store(&mut response);
    response
}

fn unauthorized_admin_session() -> Response {
    let mut response = error(
        StatusCode::UNAUTHORIZED,
        "Sesi Admin tidak valid atau sudah berakhir.",
    );
    clear_admin_session_cookie(&mut response);
    response
}

fn admin_session_cookie(headers: &HeaderMap) -> Option<Zeroizing<String>> {
    cookie_value(headers, ADMIN_SESSION_COOKIE)
}

fn admin_challenge_cookie(headers: &HeaderMap) -> Option<Zeroizing<String>> {
    cookie_value(headers, ADMIN_CHALLENGE_COOKIE)
}

fn cookie_value(headers: &HeaderMap, expected_name: &str) -> Option<Zeroizing<String>> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    cookies.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        if name != expected_name
            || value.len() != 64
            || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return None;
        }
        Some(Zeroizing::new(value.to_owned()))
    })
}

fn admin_session_cookie_header(token: &str) -> Zeroizing<String> {
    Zeroizing::new(format!(
        "{ADMIN_SESSION_COOKIE}={token}; Path=/; Max-Age={ADMIN_SESSION_MAX_AGE_SECONDS}; HttpOnly; Secure; SameSite=Strict"
    ))
}

fn admin_challenge_cookie_header(token: &str, max_age: i64) -> Zeroizing<String> {
    Zeroizing::new(format!(
        "{ADMIN_CHALLENGE_COOKIE}={token}; Path=/; Max-Age={max_age}; HttpOnly; Secure; SameSite=Strict"
    ))
}

fn clear_admin_session_cookie(response: &mut Response) {
    append_cookie(
        response,
        format!(
            "{ADMIN_SESSION_COOKIE}=; Path=/; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT; HttpOnly; Secure; SameSite=Strict"
        ),
    );
}

fn clear_admin_challenge_cookie(response: &mut Response) {
    append_cookie(
        response,
        format!(
            "{ADMIN_CHALLENGE_COOKIE}=; Path=/; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT; HttpOnly; Secure; SameSite=Strict"
        ),
    );
}

fn clear_admin_cookies(response: &mut Response) {
    clear_admin_session_cookie(response);
    clear_admin_challenge_cookie(response);
}

fn append_cookie(response: &mut Response, cookie: String) {
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_with_origin(origin: &'static str, host: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::ORIGIN, HeaderValue::from_static(origin));
        headers.insert(header::HOST, HeaderValue::from_static(host));
        headers
    }

    fn test_state() -> AdminAuthenticationApiState {
        use crate::identity::crypto::{AdminTotpKeyring, KeyVersion};

        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://app_user:secret@127.0.0.1/usahapintar_test")
            .expect("lazy test pool should be valid");
        let rate_limit_key = AuthenticationRateLimitKey::from_hex(&"ab".repeat(32))
            .expect("test rate-limit key should be valid");
        let keyring = AdminTotpKeyring::from_hex_entries(
            KeyVersion(1),
            &[(
                KeyVersion(1),
                "1111111111111111111111111111111111111111111111111111111111111111",
            )],
        )
        .expect("test Admin keyring should be valid");
        AdminAuthenticationApiState::new(pool, rate_limit_key, Arc::new(keyring))
    }

    #[tokio::test]
    async fn admin_login_rejects_cross_site_and_missing_origin_before_database_access() {
        use axum::body::Body;
        use tower::ServiceExt;

        let router = admin_authentication_router(test_state());
        let peer: SocketAddr = "127.0.0.1:3000".parse().unwrap();
        let cross_site = axum::http::Request::builder()
            .method("POST")
            .uri("/api/v1/identity/admin/login")
            .header(header::HOST, "app.example.test")
            .header(header::ORIGIN, "https://evil.example.test")
            .header("sec-fetch-site", "cross-site")
            .header(header::CONTENT_TYPE, "application/json")
            .extension(ConnectInfo(peer))
            .body(Body::from("{}"))
            .unwrap();
        let response = router
            .clone()
            .oneshot(cross_site)
            .await
            .expect("router should answer cross-site login");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let missing_origin = axum::http::Request::builder()
            .method("POST")
            .uri("/api/v1/identity/admin/login")
            .header(header::HOST, "app.example.test")
            .header(header::CONTENT_TYPE, "application/json")
            .extension(ConnectInfo(peer))
            .body(Body::from("{}"))
            .unwrap();
        let response = router
            .oneshot(missing_origin)
            .await
            .expect("router should answer login without Origin");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn admin_me_requires_its_own_session_cookie() {
        use axum::body::Body;
        use tower::ServiceExt;

        let response = admin_authentication_router(test_state())
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/identity/admin/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("router should answer unauthenticated Admin session request");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let cookies: Vec<_> = response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|value| value.to_str().unwrap().to_owned())
            .collect();
        assert_eq!(cookies.len(), 1);
        assert!(cookies[0].starts_with(&format!("{ADMIN_SESSION_COOKIE}=")));
        assert!(cookies[0].contains("SameSite=Strict"));
    }

    #[test]
    fn admin_cookies_are_separate_host_only_secure_and_strict() {
        let token = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_str(&format!(
                "theme=dark; {ADMIN_SESSION_COOKIE}={token}; {ADMIN_CHALLENGE_COOKIE}={token}"
            ))
            .unwrap(),
        );
        assert_eq!(admin_session_cookie(&headers).unwrap().as_str(), token);
        assert_eq!(admin_challenge_cookie(&headers).unwrap().as_str(), token);

        let mut response = StatusCode::NO_CONTENT.into_response();
        clear_admin_cookies(&mut response);
        let cookies: Vec<_> = response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|value| value.to_str().unwrap().to_owned())
            .collect();
        assert_eq!(cookies.len(), 2);
        assert!(cookies.iter().all(|cookie| {
            cookie.contains("Path=/")
                && cookie.contains("Max-Age=0")
                && cookie.contains("HttpOnly")
                && cookie.contains("Secure")
                && cookie.contains("SameSite=Strict")
                && !cookie.to_ascii_lowercase().contains("domain=")
        }));
    }

    #[test]
    fn issued_admin_cookies_use_strict_host_only_attributes_and_expected_lifetimes() {
        let session = admin_session_cookie_header("a".repeat(64).as_str());
        assert!(session.contains("Path=/"));
        assert!(session.contains("Max-Age=28800"));
        assert!(session.contains("HttpOnly"));
        assert!(session.contains("Secure"));
        assert!(session.contains("SameSite=Strict"));
        assert!(!session.to_ascii_lowercase().contains("domain="));

        let challenge = admin_challenge_cookie_header("b".repeat(64).as_str(), 300);
        assert!(challenge.contains("Max-Age=300"));
        assert!(challenge.contains("HttpOnly"));
        assert!(challenge.contains("Secure"));
        assert!(challenge.contains("SameSite=Strict"));
        assert!(!challenge.to_ascii_lowercase().contains("domain="));
    }

    #[test]
    fn admin_cookie_parser_rejects_malformed_tokens() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("__Host-usahapintar_admin_session=not-a-token"),
        );
        assert!(admin_session_cookie(&headers).is_none());
        assert!(admin_challenge_cookie(&headers).is_none());
    }

    #[test]
    fn admin_mutations_require_exact_same_origin_and_secure_scheme() {
        assert!(is_same_origin_request(&headers_with_origin(
            "https://app.example.test",
            "app.example.test"
        )));
        assert!(is_same_origin_request(&headers_with_origin(
            "http://localhost:5173",
            "localhost:5173"
        )));
        assert!(!is_same_origin_request(&headers_with_origin(
            "https://evil.example.test",
            "app.example.test"
        )));
        assert!(!is_same_origin_request(&headers_with_origin(
            "http://app.example.test",
            "app.example.test"
        )));
        assert!(!is_same_origin_request(&HeaderMap::new()));
    }

    #[test]
    fn default_origin_ports_are_normalized_without_accepting_other_hosts() {
        assert!(is_same_origin_request(&headers_with_origin(
            "https://app.example.test",
            "app.example.test:443"
        )));
        assert!(is_same_origin_request(&headers_with_origin(
            "http://localhost",
            "localhost:80"
        )));
        assert!(!is_same_origin_request(&headers_with_origin(
            "https://evil.example.test",
            "app.example.test:443"
        )));
    }
}
