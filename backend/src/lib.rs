pub mod config;
pub mod database;
pub mod identity;

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

#[cfg(test)]
mod tests {
    use super::app;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

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
