pub mod config;
pub mod database;

use axum::{routing::get, Json, Router};
use serde::Serialize;

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

pub fn app() -> Router {
    Router::new().route("/api/v1/health", get(health))
}

#[cfg(test)]
mod tests {
    use super::app;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let response = app()
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
