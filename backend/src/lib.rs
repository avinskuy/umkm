pub mod config;
pub mod database;
pub mod identity;

use crate::identity::http::{
    admin_authentication_router, authentication_router, recovery_router, registration_router,
    AdminAuthenticationApiState, AuthenticationApiState, RecoveryApiState, RegistrationApiState,
};
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "usahapintar-api",
    })
}

async fn readiness(State(pool): State<PgPool>) -> (StatusCode, Json<HealthResponse>) {
    let database_ready = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
        .is_ok();

    if database_ready {
        (
            StatusCode::OK,
            Json(HealthResponse {
                status: "ready",
                service: "usahapintar-api",
            }),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthResponse {
                status: "unavailable",
                service: "usahapintar-api",
            }),
        )
    }
}

/// Build the HTTP application using the runtime application's PostgreSQL pool.
pub fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/readyz", get(readiness))
        .with_state(pool)
}

/// Build the public API with password authentication routes mounted.
pub fn app_with_authentication(pool: PgPool, authentication: AuthenticationApiState) -> Router {
    app(pool).merge(authentication_router(authentication))
}

/// Build the public API with Owner/Investor and isolated Admin authentication routes.
pub fn app_with_authentication_and_admin(
    pool: PgPool,
    authentication: AuthenticationApiState,
    admin_authentication: AdminAuthenticationApiState,
) -> Router {
    app_with_authentication(pool, authentication)
        .merge(admin_authentication_router(admin_authentication))
}

/// Build the public API with password authentication and account recovery routes mounted.
pub fn app_with_authentication_and_recovery(
    pool: PgPool,
    authentication: AuthenticationApiState,
    recovery: RecoveryApiState,
) -> Router {
    app_with_authentication(pool, authentication).merge(recovery_router(recovery))
}

/// Build the public API with password authentication, isolated Admin authentication,
/// and verified client registration routes mounted.
pub fn app_with_authentication_admin_and_registration(
    pool: PgPool,
    authentication: AuthenticationApiState,
    admin_authentication: AdminAuthenticationApiState,
    registration: RegistrationApiState,
) -> Router {
    app_with_authentication_and_admin(pool, authentication, admin_authentication)
        .merge(registration_router(registration))
}

/// Build the runtime API with authentication, Admin, registration, and account recovery.
pub fn app_with_authentication_admin_registration_and_recovery(
    pool: PgPool,
    authentication: AuthenticationApiState,
    admin_authentication: AdminAuthenticationApiState,
    registration: RegistrationApiState,
    recovery: RecoveryApiState,
) -> Router {
    app_with_authentication_admin_and_registration(
        pool,
        authentication,
        admin_authentication,
        registration,
    )
    .merge(recovery_router(recovery))
}

#[cfg(test)]
mod tests {
    use super::{
        app, app_with_authentication_admin_and_registration,
        app_with_authentication_admin_registration_and_recovery,
    };
    use crate::identity::{
        crypto::{AdminTotpKeyring, KeyVersion},
        flowkirim_http::FlowKirimHttpProvider,
        http::{
            AdminAuthenticationApiState, AuthenticationApiState, PostgresRegistrationAbuseGuard,
            RecoveryApiState, RegistrationApiState, RegistrationPolicyVersions,
        },
        password::SimplePasswordPolicy,
        resend_http::ResendHttpProvider,
        AuthenticationRateLimitKey,
    };
    use axum::body::Body;
    use http_body_util::BodyExt;
    use sqlx::postgres::PgPoolOptions;
    use std::sync::Arc;
    use tower::ServiceExt;
    use zeroize::Zeroizing;

    #[tokio::test]
    async fn registration_http_routes_are_not_mounted_in_the_default_application() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://app_user:secret@127.0.0.1/usahapintar_test")
            .expect("lazy pool configuration should be valid");

        let response = app(pool)
            .oneshot(
                axum::http::Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/api/v1/identity/registrations")
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .expect("request should be valid"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn account_recovery_routes_are_not_mounted_in_the_default_application() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://app_user:secret@127.0.0.1/usahapintar_test")
            .expect("lazy pool configuration should be valid");

        let response = app(pool)
            .oneshot(
                axum::http::Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/api/v1/identity/recovery")
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .expect("request should be valid"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn runtime_application_mounts_registration_and_recovery_routes() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://app_user:secret@127.0.0.1/usahapintar_test")
            .expect("lazy pool configuration should be valid");
        let rate_limit_key = AuthenticationRateLimitKey::from_hex(&"11".repeat(32))
            .expect("test rate-limit key should be valid");
        let admin_keyring = || {
            Arc::new(
                AdminTotpKeyring::from_hex_entries(
                    KeyVersion(1),
                    &[(
                        KeyVersion(1),
                        "2222222222222222222222222222222222222222222222222222222222222222",
                    )],
                )
                .expect("test Admin keyring should be valid"),
            )
        };
        let resend = || {
            Arc::new(
                ResendHttpProvider::new(
                    Zeroizing::new("test-resend-key".to_owned()),
                    "USahapintar <noreply@example.test>".to_owned(),
                )
                .expect("test Resend provider should be valid"),
            )
        };
        let flowkirim = || {
            Arc::new(
                FlowKirimHttpProvider::new(Zeroizing::new("test-flowkirim-key".to_owned()))
                    .expect("test FlowKirim provider should be valid"),
            )
        };

        let authentication = AuthenticationApiState::new(pool.clone(), rate_limit_key.clone());
        let admin_authentication =
            AdminAuthenticationApiState::new(pool.clone(), rate_limit_key.clone(), admin_keyring());
        let registration = RegistrationApiState::new(
            pool.clone(),
            admin_keyring(),
            resend(),
            Arc::new(PostgresRegistrationAbuseGuard::new(
                pool.clone(),
                rate_limit_key.clone(),
            )),
            Arc::new(SimplePasswordPolicy),
            RegistrationPolicyVersions::new(
                "terms-test-v1".to_owned(),
                "privacy-test-v1".to_owned(),
                None,
            )
            .expect("test policy versions should be valid"),
        )
        .with_flowkirim_provider(flowkirim());
        let recovery = RecoveryApiState::new(
            pool.clone(),
            admin_keyring(),
            resend(),
            Arc::new(PostgresRegistrationAbuseGuard::new(
                pool.clone(),
                rate_limit_key,
            )),
        )
        .with_flowkirim_provider(flowkirim())
        .with_password_policy(Arc::new(SimplePasswordPolicy));

        let app = app_with_authentication_admin_registration_and_recovery(
            pool,
            authentication,
            admin_authentication,
            registration,
            recovery,
        );

        let registration_response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method(axum::http::Method::GET)
                    .uri("/api/v1/identity/registrations")
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .await
            .expect("router should respond");
        assert_eq!(
            registration_response.status(),
            axum::http::StatusCode::METHOD_NOT_ALLOWED
        );

        let recovery_response = app
            .oneshot(
                axum::http::Request::builder()
                    .method(axum::http::Method::GET)
                    .uri("/api/v1/identity/recovery")
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .await
            .expect("router should respond");
        assert_eq!(
            recovery_response.status(),
            axum::http::StatusCode::METHOD_NOT_ALLOWED
        );
    }

    #[tokio::test]
    async fn health_endpoint_is_a_database_independent_liveness_check() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://app_user:secret@127.0.0.1/usahapintar_test")
            .expect("lazy pool configuration should be valid");

        let response = app(pool)
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .expect("request should be valid"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("body should be readable")
            .to_bytes();

        let payload: serde_json::Value =
            serde_json::from_slice(&body).expect("body should contain JSON");

        assert_eq!(payload["status"], "ok");
        assert_eq!(payload["service"], "usahapintar-api");
    }
}
