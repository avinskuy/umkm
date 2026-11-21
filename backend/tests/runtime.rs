use axum::{body::Body, http::Request};
use sqlx::PgPool;
use tower::ServiceExt;
use usahapintar_api::{
    app,
    database::{connect, DatabaseConfig},
};

async fn application_pool() -> PgPool {
    let config = DatabaseConfig::from_app_env()
        .expect("CI must provide APP_DATABASE_URL for the application principal");
    connect(&config)
        .await
        .expect("application principal should connect to PostgreSQL")
}

#[tokio::test]
#[ignore = "requires the disposable least-privilege PostgreSQL application principal"]
async fn runtime_readiness_uses_the_application_database_connection() {
    let pool = application_pool().await;

    let response = app(pool.clone())
        .oneshot(
            Request::builder()
                .uri("/api/v1/readyz")
                .body(Body::empty())
                .expect("readiness request should be valid"),
        )
        .await
        .expect("readiness router should respond");

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let response = app(pool.clone())
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .expect("liveness request should be valid"),
        )
        .await
        .expect("liveness router should respond");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires the disposable least-privilege PostgreSQL application principal"]
async fn identity_runtime_access_uses_only_explicit_database_grants() {
    use serde_json::json;
    use usahapintar_api::identity::{
        append_audit_event, create_client_user, AuditActorType, AuditEvent, AuditOutcome,
        ClientRegistration, ClientRole, ContactChannel,
    };

    let pool = application_pool().await;
    let email = format!("identity-runtime-{}@example.test", std::process::id());

    let registration = ClientRegistration {
        role: ClientRole::BusinessOwner,
        initial_contact_channel: ContactChannel::Email,
        email_normalized: Some(&email),
        phone_e164: None,
        password_hash: Some("$argon2id$test-only-placeholder"),
        google_subject: None,
    };

    let mut transaction = pool
        .begin()
        .await
        .expect("application may start a transaction");
    let user_id = create_client_user(&mut transaction, &registration)
        .await
        .expect("application may create a pending account through the guarded database function");

    let audit_event = AuditEvent {
        actor_type: AuditActorType::System,
        actor_id: None,
        subject_user_id: Some(&user_id),
        action_code: "identity.registration.requested",
        outcome: AuditOutcome::Success,
        reason_code: Some("registration_started"),
        request_id: None,
        metadata: json!({
            "channel": "email",
            "purpose": "registration"
        }),
    };

    append_audit_event(&mut transaction, &audit_event)
        .await
        .expect("application may append a validated audit event");

    transaction
        .commit()
        .await
        .expect("account creation and its audit event should commit atomically");
    assert!(!event_id.is_empty());

    assert!(
        sqlx::query("SELECT id FROM public.users WHERE id = $1::uuid")
            .bind(&user_id)
            .fetch_one(&pool)
            .await
            .is_err(),
        "application must not read account rows directly"
    );

    assert!(
        sqlx::query("SELECT id FROM public.identity_audit_events WHERE id = $1::uuid")
            .bind(&event_id)
            .fetch_one(&pool)
            .await
            .is_err(),
        "application must not read audit records"
    );

    pool.close().await;
}
