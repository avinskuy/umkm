use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{header, Method, Request},
};
use http_body_util::BodyExt;
use sqlx::PgPool;
use std::{
    future::Future,
    net::{IpAddr, SocketAddr},
    pin::Pin,
    sync::{Arc, Mutex},
};
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
        display_name: "Runtime Test User",
        role: ClientRole::BusinessOwner,
        initial_contact_channel: ContactChannel::Email,
        email_normalized: Some(&email),
        phone_e164: None,
        password_hash: "$argon2id$test-only-placeholder",
        terms_of_service_version: "terms-v1",
        privacy_policy_version: "privacy-v1",
        marketing_consent: false,
        marketing_notice_version: None,
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
    assert!(
        sqlx::query("SELECT id FROM public.users WHERE id = $1::uuid")
            .bind(&user_id)
            .fetch_one(&pool)
            .await
            .is_err(),
        "application must not read account rows directly"
    );

    assert!(
        sqlx::query("SELECT count(*) FROM public.identity_audit_events")
            .fetch_one(&pool)
            .await
            .is_err(),
        "application must not read audit records"
    );

    let admin_url = std::env::var("CI_TARGET_ADMIN_DATABASE_URL")
        .expect("CI must provide a disposable DBA connection for audit verification");
    let admin_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_url)
        .await
        .expect("disposable DBA connection should succeed");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.identity_audit_events \
         WHERE subject_user_id = $1::uuid \
           AND action_code = 'identity.registration.requested'",
    )
    .bind(&user_id)
    .fetch_one(&admin_pool)
    .await
    .expect("DBA should verify the append-only audit event was persisted");
    assert_eq!(audit_count, 1);

    admin_pool.close().await;
    pool.close().await;
}


#[tokio::test]
#[ignore = "requires the disposable least-privilege PostgreSQL application principal"]
async fn registration_http_router_creates_and_verifies_an_account_without_being_mounted_by_default()
{
    use usahapintar_api::identity::{
        crypto::{CryptoError, KeyProvider, KeyPurpose, KeyVersion, SecretKey, VersionedKey},
        delivery::{DeliveryChannel, DispatchStatus, OtpDeliveryMessage, OtpDeliveryProvider},
        http::{
            registration_router, AbuseDecision, RegistrationAbuseGuard, RegistrationAction,
            RegistrationApiState, RegistrationPasswordPolicy, RegistrationPolicyVersions,
        },
    };

    struct RuntimeKeys;

    impl KeyProvider for RuntimeKeys {
        fn active_key(&self, purpose: KeyPurpose) -> Result<VersionedKey, CryptoError> {
            self.key(purpose, KeyVersion(1))
                .map(|key| VersionedKey::new(KeyVersion(1), key))
        }

        fn key(&self, purpose: KeyPurpose, version: KeyVersion) -> Result<SecretKey, CryptoError> {
            if version != KeyVersion(1) {
                return Err(CryptoError::KeyUnavailable);
            }
            let marker = match purpose {
                KeyPurpose::DataEncryption => 0x11,
                KeyPurpose::ContactFingerprint => 0x22,
                KeyPurpose::OtpVerification => 0x33,
            };
            Ok(SecretKey::from_bytes([marker; 32]))
        }
    }

    struct RuntimeDelivery {
        last_code: Arc<Mutex<Option<String>>>,
    }

    impl OtpDeliveryProvider for RuntimeDelivery {
        fn dispatch<'a>(
            &'a self,
            message: OtpDeliveryMessage,
        ) -> Pin<Box<dyn Future<Output = DispatchStatus> + Send + 'a>> {
            Box::pin(async move {
                *self
                    .last_code
                    .lock()
                    .expect("test delivery lock should be available") =
                    Some(message.code.to_string());
                match message.channel {
                    DeliveryChannel::Email | DeliveryChannel::WhatsApp => DispatchStatus::Accepted,
                }
            })
        }
    }

    struct AllowAllTestGuard;

    impl RegistrationAbuseGuard for AllowAllTestGuard {
        fn check<'a>(
            &'a self,
            _action: RegistrationAction,
            _peer_ip: IpAddr,
            _subject: Option<&'a str>,
        ) -> Pin<Box<dyn Future<Output = AbuseDecision> + Send + 'a>> {
            Box::pin(async { AbuseDecision::Allowed })
        }
    }

    struct TestPasswordPolicy;

    impl RegistrationPasswordPolicy for TestPasswordPolicy {
        fn accepts(&self, password: &str) -> bool {
            (12..=128).contains(&password.len())
        }
    }

    let pool = application_pool().await;
    let last_code = Arc::new(Mutex::new(None));
    let state = RegistrationApiState::new(
        pool.clone(),
        Arc::new(RuntimeKeys),
        Arc::new(RuntimeDelivery {
            last_code: last_code.clone(),
        }),
        Arc::new(AllowAllTestGuard),
        Arc::new(TestPasswordPolicy),
        RegistrationPolicyVersions::new(
            "terms-test-v1".to_owned(),
            "privacy-test-v1".to_owned(),
            Some("marketing-test-v1".to_owned()),
        )
        .expect("test policy versions should be valid"),
    );
    let router = registration_router(state);
    let email = format!(
        "identity-http-{}-{}@example.test",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos()
    );

    let registration_body = serde_json::json!({
        "display_name": "HTTP Test Owner",
        "role": "business_owner",
        "contact_channel": "email",
        "email": email,
        "password": "test-password-with-enough-length",
        "password_confirmation": "test-password-with-enough-length",
        "terms_accepted": true,
        "privacy_accepted": true,
        "marketing_consent": false
    });
    let make_request = |uri: String, body: serde_json::Value| {
        let mut request = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .expect("HTTP request should be valid");
        request.extensions_mut().insert(ConnectInfo(
            "127.0.0.1:40000"
                .parse::<SocketAddr>()
                .expect("test socket address should be valid"),
        ));
        request
    };

    let created = router
        .clone()
        .oneshot(make_request(
            "/api/v1/identity/registrations".to_owned(),
            registration_body.clone(),
        ))
        .await
        .expect("registration route should respond");
    assert_eq!(created.status(), axum::http::StatusCode::ACCEPTED);
    let created_body = created
        .into_body()
        .collect()
        .await
        .expect("registration response should be readable")
        .to_bytes();
    let created_payload: serde_json::Value =
        serde_json::from_slice(&created_body).expect("registration response should be JSON");
    let verification_id = created_payload["verification_id"]
        .as_str()
        .expect("registration response should include verification ID")
        .to_owned();
    let code = last_code
        .lock()
        .expect("test delivery lock should be available")
        .clone()
        .expect("fake provider should capture the OTP");

    let verified = router
        .clone()
        .oneshot(make_request(
            format!("/api/v1/identity/registrations/{verification_id}/verify"),
            serde_json::json!({ "code": code }),
        ))
        .await
        .expect("verification route should respond");
    assert_eq!(verified.status(), axum::http::StatusCode::OK);

    let duplicate = router
        .oneshot(make_request(
            "/api/v1/identity/registrations".to_owned(),
            registration_body,
        ))
        .await
        .expect("duplicate registration should receive a generic response");
    assert_eq!(duplicate.status(), axum::http::StatusCode::ACCEPTED);
    let duplicate_body = duplicate
        .into_body()
        .collect()
        .await
        .expect("duplicate response should be readable")
        .to_bytes();
    let duplicate_payload: serde_json::Value =
        serde_json::from_slice(&duplicate_body).expect("duplicate response should be JSON");
    assert_eq!(
        duplicate_payload["message"], created_payload["message"],
        "duplicate identifier must not be disclosed in the public response"
    );
    assert_ne!(
        duplicate_payload["verification_id"], created_payload["verification_id"],
        "duplicate response must not expose the original verification session"
    );

    pool.close().await;
}
