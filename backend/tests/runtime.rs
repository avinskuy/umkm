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
                KeyPurpose::AdminTotpSecret => 0x44,
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

    use usahapintar_api::identity::{
        authentication::{hash_session_token, AuthenticationRateLimitKey},
        http::{authentication_router, AuthenticationApiState},
    };

    let authentication = authentication_router(AuthenticationApiState::new(
        pool.clone(),
        AuthenticationRateLimitKey::from_hex(&"ab".repeat(32))
            .expect("test HMAC key should be valid"),
    ));

    let wrong_role = authentication
        .clone()
        .oneshot(make_request(
            "/api/v1/identity/login/investor".to_owned(),
            serde_json::json!({
                "identifier": email,
                "password": "test-password-with-enough-length"
            }),
        ))
        .await
        .expect("wrong-role login should respond");
    assert_eq!(wrong_role.status(), axum::http::StatusCode::UNAUTHORIZED);

    let wrong_password = authentication
        .clone()
        .oneshot(make_request(
            "/api/v1/identity/login/owner".to_owned(),
            serde_json::json!({
                "identifier": email,
                "password": "incorrect-password"
            }),
        ))
        .await
        .expect("wrong-password login should respond");
    assert_eq!(
        wrong_password.status(),
        axum::http::StatusCode::UNAUTHORIZED
    );
    assert!(
        wrong_password.headers().get(header::SET_COOKIE).is_none(),
        "failed login must not create a session cookie"
    );

    let logged_in = authentication
        .clone()
        .oneshot(make_request(
            "/api/v1/identity/login/owner".to_owned(),
            serde_json::json!({
                "identifier": email,
                "password": "test-password-with-enough-length"
            }),
        ))
        .await
        .expect("owner login should respond");
    assert_eq!(logged_in.status(), axum::http::StatusCode::OK);
    let set_cookie = logged_in
        .headers()
        .get(header::SET_COOKIE)
        .expect("successful login should set a session cookie")
        .to_str()
        .expect("session cookie should be valid header text")
        .to_owned();
    assert!(set_cookie.starts_with("__Host-usahapintar_session="));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("Secure"));
    assert!(set_cookie.contains("SameSite=Lax"));
    let token = set_cookie
        .split(';')
        .next()
        .and_then(|cookie| cookie.split_once('='))
        .map(|(_, value)| value.to_owned())
        .expect("session cookie should contain the opaque token");
    assert_eq!(token.len(), 64);

    let mut logout_request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/identity/logout")
        .header(
            header::COOKIE,
            format!("__Host-usahapintar_session={token}"),
        )
        .body(Body::empty())
        .expect("logout request should be valid");
    logout_request.extensions_mut().insert(ConnectInfo(
        "127.0.0.1:40000"
            .parse::<SocketAddr>()
            .expect("test socket address should be valid"),
    ));
    let logged_out = authentication
        .clone()
        .oneshot(logout_request)
        .await
        .expect("logout route should respond");
    assert_eq!(logged_out.status(), axum::http::StatusCode::NO_CONTENT);
    let cleared_cookie = logged_out
        .headers()
        .get(header::SET_COOKIE)
        .expect("logout should clear the session cookie")
        .to_str()
        .expect("clearing cookie should be valid header text");
    assert!(cleared_cookie.contains("Max-Age=0"));

    let token_hash = hash_session_token(&token);
    let admin_url = std::env::var("CI_TARGET_ADMIN_DATABASE_URL")
        .expect("CI must provide the disposable DBA connection");
    let admin_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_url)
        .await
        .expect("disposable DBA connection should succeed");
    let revoked_reason: String =
        sqlx::query_scalar("SELECT revocation_reason FROM public.sessions WHERE token_hash = $1")
            .bind(token_hash.to_vec())
            .fetch_one(&admin_pool)
            .await
            .expect("logged-out session should remain as a revoked audit record");
    assert_eq!(revoked_reason, "logout");

    let logged_in_again = authentication
        .clone()
        .oneshot(make_request(
            "/api/v1/identity/login/owner".to_owned(),
            serde_json::json!({
                "identifier": email,
                "password": "test-password-with-enough-length"
            }),
        ))
        .await
        .expect("second owner login should respond");
    assert_eq!(logged_in_again.status(), axum::http::StatusCode::OK);
    let second_cookie = logged_in_again
        .headers()
        .get(header::SET_COOKIE)
        .expect("second login should set a session cookie")
        .to_str()
        .expect("second cookie should be valid header text")
        .to_owned();
    let second_token = second_cookie
        .split(';')
        .next()
        .and_then(|cookie| cookie.split_once('='))
        .map(|(_, value)| value.to_owned())
        .expect("second cookie should contain the session token");

    let make_session_request = |method: Method, uri: &str, token: &str| {
        Request::builder()
            .method(method)
            .uri(uri)
            .header(
                header::COOKIE,
                format!("__Host-usahapintar_session={token}"),
            )
            .body(Body::empty())
            .expect("session request should be valid")
    };

    let current_session = authentication
        .clone()
        .oneshot(make_session_request(
            Method::GET,
            "/api/v1/identity/me",
            &second_token,
        ))
        .await
        .expect("session introspection should respond");
    assert_eq!(current_session.status(), axum::http::StatusCode::OK);
    let current_body = current_session
        .into_body()
        .collect()
        .await
        .expect("current-session body should be readable")
        .to_bytes();
    let current_payload: serde_json::Value =
        serde_json::from_slice(&current_body).expect("current-session body should be JSON");
    assert_eq!(current_payload["role"], "business_owner");

    let refreshed = authentication
        .clone()
        .oneshot(make_session_request(
            Method::POST,
            "/api/v1/identity/session/refresh",
            &second_token,
        ))
        .await
        .expect("session refresh should respond");
    assert_eq!(refreshed.status(), axum::http::StatusCode::OK);
    let refreshed_cookie = refreshed
        .headers()
        .get(header::SET_COOKIE)
        .expect("session refresh should rotate the browser cookie")
        .to_str()
        .expect("refreshed cookie should be valid header text")
        .to_owned();
    let refreshed_token = refreshed_cookie
        .split(';')
        .next()
        .and_then(|cookie| cookie.split_once('='))
        .map(|(_, value)| value.to_owned())
        .expect("refreshed cookie should contain the new token");
    assert_ne!(refreshed_token, second_token);

    let old_token_response = authentication
        .clone()
        .oneshot(make_session_request(
            Method::GET,
            "/api/v1/identity/me",
            &second_token,
        ))
        .await
        .expect("old token validation should respond");
    assert_eq!(
        old_token_response.status(),
        axum::http::StatusCode::UNAUTHORIZED
    );

    let refreshed_session = authentication
        .clone()
        .oneshot(make_session_request(
            Method::GET,
            "/api/v1/identity/me",
            &refreshed_token,
        ))
        .await
        .expect("rotated token should authenticate");
    assert_eq!(refreshed_session.status(), axum::http::StatusCode::OK);

    let logged_out_all = authentication
        .clone()
        .oneshot(make_session_request(
            Method::POST,
            "/api/v1/identity/logout-all",
            &refreshed_token,
        ))
        .await
        .expect("logout-all should respond");
    assert_eq!(logged_out_all.status(), axum::http::StatusCode::NO_CONTENT);
    let refreshed_hash = hash_session_token(&refreshed_token);
    let all_revoked_reason: String =
        sqlx::query_scalar("SELECT revocation_reason FROM public.sessions WHERE token_hash = $1")
            .bind(refreshed_hash.to_vec())
            .fetch_one(&admin_pool)
            .await
            .expect("logout-all should persist the revocation reason");
    assert_eq!(all_revoked_reason, "logout_all");

    let after_logout_all = authentication
        .oneshot(make_session_request(
            Method::GET,
            "/api/v1/identity/me",
            &refreshed_token,
        ))
        .await
        .expect("revoked token validation should respond");
    assert_eq!(
        after_logout_all.status(),
        axum::http::StatusCode::UNAUTHORIZED
    );
    admin_pool.close().await;

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

#[tokio::test]
#[ignore = "requires the disposable least-privilege PostgreSQL application principal"]
async fn account_recovery_service_verifies_contact_and_binds_a_matching_account() {
    use usahapintar_api::identity::{
        authentication::hash_session_token,
        crypto::{
            contact_fingerprint, CryptoError, KeyProvider, KeyPurpose, KeyVersion, SecretKey,
            VersionedKey,
        },
        delivery::{
            DeliveryChannel, DispatchStatus, OtpDeliveryMessage, OtpDeliveryProvider,
            PasswordResetDeliveryMessage, PasswordResetDeliveryProvider,
        },
        flowkirim::{FlowKirimProviderOutcome, FlowKirimVerifyProvider},
        password::{hash_password, verify_password},
        password_reset::PasswordResetIssuanceService,
        ClientRegistration, ClientRole, ContactChannel, RecoveryReviewDecision,
        RecoveryReviewReason, RecoveryReviewService, VerificationService,
    };

    struct RecoveryKeys;

    impl KeyProvider for RecoveryKeys {
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
                KeyPurpose::AdminTotpSecret => 0x44,
            };
            Ok(SecretKey::from_bytes([marker; 32]))
        }
    }

    struct RecoveryDelivery {
        last_code: Arc<Mutex<Option<String>>>,
    }

    struct FakeRecoveryFlowKirim;

    impl FlowKirimVerifyProvider for FakeRecoveryFlowKirim {
        fn start<'a>(
            &'a self,
            _destination: Zeroizing<String>,
            _idempotency_key: String,
        ) -> Pin<Box<dyn Future<Output = FlowKirimProviderOutcome<String>> + Send + 'a>> {
            Box::pin(async {
                FlowKirimProviderOutcome::Accepted("flowkirim-recovery-runtime-request".to_owned())
            })
        }

        fn check<'a>(
            &'a self,
            _request_id: String,
            code: Zeroizing<String>,
        ) -> Pin<Box<dyn Future<Output = FlowKirimProviderOutcome<bool>> + Send + 'a>> {
            Box::pin(async move { FlowKirimProviderOutcome::Accepted(code.as_str() == "123456") })
        }
    }

    struct CapturingPasswordResetDelivery {
        reset_url: Arc<Mutex<Option<String>>>,
        destination: Arc<Mutex<Option<String>>>,
        expires_at_unix: Arc<Mutex<Option<i64>>>,
        status: DispatchStatus,
    }

    impl PasswordResetDeliveryProvider for CapturingPasswordResetDelivery {
        fn dispatch<'a>(
            &'a self,
            message: PasswordResetDeliveryMessage,
        ) -> Pin<Box<dyn Future<Output = DispatchStatus> + Send + 'a>> {
            Box::pin(async move {
                *self
                    .reset_url
                    .lock()
                    .expect("reset URL capture lock should be available") =
                    Some(message.reset_url.to_string());
                *self
                    .destination
                    .lock()
                    .expect("reset destination capture lock should be available") =
                    Some(message.destination.to_string());
                *self
                    .expires_at_unix
                    .lock()
                    .expect("reset expiry capture lock should be available") =
                    Some(message.expires_at_unix);
                self.status
            })
        }
    }

    struct ResetPasswordPolicy;

    impl usahapintar_api::identity::http::RegistrationPasswordPolicy for ResetPasswordPolicy {
        fn accepts(&self, password: &str) -> bool {
            (12..=1024).contains(&password.len())
        }
    }

    impl OtpDeliveryProvider for RecoveryDelivery {
        fn dispatch<'a>(
            &'a self,
            message: OtpDeliveryMessage,
        ) -> Pin<Box<dyn Future<Output = DispatchStatus> + Send + 'a>> {
            Box::pin(async move {
                *self
                    .last_code
                    .lock()
                    .expect("recovery test delivery lock should be available") =
                    Some(message.code.to_string());
                match message.channel {
                    DeliveryChannel::Email | DeliveryChannel::WhatsApp => DispatchStatus::Accepted,
                }
            })
        }
    }

    let pool = application_pool().await;
    let keys = Arc::new(RecoveryKeys);
    let last_code = Arc::new(Mutex::new(None));
    let delivery = RecoveryDelivery {
        last_code: last_code.clone(),
    };
    let email = format!(
        "recovery-runtime-{}-{}@example.test",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos()
    );
    let password_hash = hash_password(b"recovery-runtime-password-strong").unwrap();

    let registration = ClientRegistration {
        display_name: "Recovery Runtime User",
        role: ClientRole::BusinessOwner,
        initial_contact_channel: ContactChannel::Email,
        email_normalized: Some(&email),
        phone_e164: None,
        password_hash: &password_hash,
        terms_of_service_version: "terms-test-v1",
        privacy_policy_version: "privacy-test-v1",
        marketing_consent: false,
        marketing_notice_version: None,
    };
    let verification = VerificationService::new(&pool, keys.as_ref(), &delivery);
    let registered = verification
        .register(&registration)
        .await
        .expect("runtime should register a pending account");
    let registration_code = last_code
        .lock()
        .expect("recovery test delivery lock should be available")
        .clone()
        .expect("registration OTP should be captured");
    assert!(verification
        .verify(&registered.session_id, &registration_code)
        .await
        .expect("registration OTP should be verified"));

    let admin_url = std::env::var("CI_TARGET_ADMIN_DATABASE_URL")
        .expect("CI must provide a disposable DBA connection");
    let admin_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_url)
        .await
        .expect("disposable DBA connection should succeed");
    let recovery_email = format!("recovery-{email}");
    let fingerprint = contact_fingerprint(keys.as_ref(), "email", &recovery_email)
        .expect("recovery contact fingerprint should be generated");
    let contact_session_id: String = sqlx::query_scalar(
        "INSERT INTO public.verification_sessions ( \
             id, user_id, purpose, channel, destination_ciphertext, destination_nonce, \
             encryption_key_version, destination_hmac, hmac_key_version, expires_at \
         ) VALUES ( \
             gen_random_uuid(), $1::uuid, 'recovery_contact_add', 'email', \
             decode('0102','hex'), decode('010203040506070809101112','hex'), \
             'v1', $2, 'v1', clock_timestamp() + INTERVAL '10 minutes' \
         ) RETURNING id::text",
    )
    .bind(&registered.user_id)
    .bind(fingerprint.digest.to_vec())
    .fetch_one(&admin_pool)
    .await
    .expect("verified recovery-contact session should be created for test setup");

    sqlx::query(
        "UPDATE public.verification_sessions \
            SET status = 'verified', verified_at = clock_timestamp() \
          WHERE id = $1::uuid",
    )
    .bind(&contact_session_id)
    .execute(&admin_pool)
    .await
    .expect("test recovery-contact session should be verified");

    sqlx::query(
        "INSERT INTO public.recovery_contacts ( \
             id, user_id, channel, contact_ciphertext, contact_nonce, encryption_key_version, \
             contact_hmac, hmac_key_version, verification_session_id, verified_at \
         ) \
         SELECT gen_random_uuid(), user_id, channel, destination_ciphertext, destination_nonce, \
                encryption_key_version, destination_hmac, hmac_key_version, id, verified_at \
           FROM public.verification_sessions WHERE id = $1::uuid AND status = 'verified'",
    )
    .bind(&contact_session_id)
    .execute(&admin_pool)
    .await
    .expect("test account should have a verified recovery contact");

    let session_token = "a".repeat(64);
    let session_token_hash = hash_session_token(&session_token);
    sqlx::query(
        "INSERT INTO public.sessions (user_id, token_hash, expires_at) \
         VALUES ($1::uuid, $2, clock_timestamp() + INTERVAL '30 days')",
    )
    .bind(&registered.user_id)
    .bind(session_token_hash.to_vec())
    .execute(&admin_pool)
    .await
    .expect("a prior active session should exist for recovery signal testing");

    use usahapintar_api::identity::http::{
        recovery_router, AbuseDecision, RecoveryApiState, RegistrationAbuseGuard,
        RegistrationAction,
    };

    struct AllowRecoveryGuard;

    impl RegistrationAbuseGuard for AllowRecoveryGuard {
        fn check<'a>(
            &'a self,
            _action: RegistrationAction,
            _peer_ip: IpAddr,
            _subject: Option<&'a str>,
        ) -> Pin<Box<dyn Future<Output = AbuseDecision> + Send + 'a>> {
            Box::pin(async { AbuseDecision::Allowed })
        }
    }

    let recovery_router = recovery_router(
        RecoveryApiState::new(
            pool.clone(),
            keys.clone(),
            Arc::new(RecoveryDelivery {
                last_code: last_code.clone(),
            }),
            Arc::new(AllowRecoveryGuard),
        )
        .with_flowkirim_provider(Arc::new(FakeRecoveryFlowKirim))
        .with_password_policy(Arc::new(ResetPasswordPolicy)),
    );
    let make_recovery_request =
        |method: Method, uri: String, body: Option<serde_json::Value>, cross_site: bool| {
            let mut builder = Request::builder().method(method).uri(uri);
            if body.is_some() {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
            }
            if cross_site {
                builder = builder.header("sec-fetch-site", "cross-site");
            }
            let mut request = builder
                .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
                .expect("recovery HTTP request should be valid");
            request.extensions_mut().insert(ConnectInfo(
                "127.0.0.1:40001"
                    .parse::<SocketAddr>()
                    .expect("recovery test socket address should be valid"),
            ));
            request
        };

    let recovery_phone = format!("+62812345{:06}", std::process::id() % 1_000_000);
    let whatsapp_started = recovery_router
        .clone()
        .oneshot(make_recovery_request(
            Method::POST,
            "/api/v1/identity/recovery".to_owned(),
            Some(serde_json::json!({
                "contact_channel": "whatsapp",
                "phone": recovery_phone,
                "requested_action": "password_reset"
            })),
            false,
        ))
        .await
        .expect("WhatsApp recovery start route should respond");
    assert_eq!(whatsapp_started.status(), axum::http::StatusCode::ACCEPTED);
    let whatsapp_started_body = whatsapp_started
        .into_body()
        .collect()
        .await
        .expect("WhatsApp recovery response should be readable")
        .to_bytes();
    let whatsapp_started_payload: serde_json::Value =
        serde_json::from_slice(&whatsapp_started_body)
            .expect("WhatsApp recovery response should be JSON");
    let whatsapp_verification_id = whatsapp_started_payload["verification_id"]
        .as_str()
        .expect("WhatsApp recovery should return a verification ID")
        .to_owned();
    let whatsapp_case_id = whatsapp_started_payload["recovery_case_id"]
        .as_str()
        .expect("WhatsApp recovery should return a case ID")
        .to_owned();

    let whatsapp_verified = recovery_router
        .clone()
        .oneshot(make_recovery_request(
            Method::POST,
            format!("/api/v1/identity/recovery/{whatsapp_verification_id}/verify"),
            Some(serde_json::json!({ "code": "123456" })),
            false,
        ))
        .await
        .expect("WhatsApp recovery verify route should respond");
    assert_eq!(
        whatsapp_verified.status(),
        axum::http::StatusCode::OK,
        "FlowKirim Verify should validate WhatsApp account recovery"
    );
    let whatsapp_case_status: String =
        sqlx::query_scalar("SELECT status FROM public.account_recovery_cases WHERE id = $1::uuid")
            .bind(&whatsapp_case_id)
            .fetch_one(&pool)
            .await
            .expect("WhatsApp recovery case status should be queryable");
    assert_eq!(whatsapp_case_status, "account_selection_required");

    let started = recovery_router
        .clone()
        .oneshot(make_recovery_request(
            Method::POST,
            "/api/v1/identity/recovery".to_owned(),
            Some(serde_json::json!({
                "contact_channel": "email",
                "email": recovery_email.clone(),
                "requested_action": "password_reset"
            })),
            false,
        ))
        .await
        .expect("recovery start route should respond");
    assert_eq!(started.status(), axum::http::StatusCode::ACCEPTED);
    assert_eq!(
        started.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-store"
    );
    let started_body = started
        .into_body()
        .collect()
        .await
        .expect("recovery start response should be readable")
        .to_bytes();
    let started_payload: serde_json::Value =
        serde_json::from_slice(&started_body).expect("recovery start response should be JSON");
    let verification_id = started_payload["verification_id"]
        .as_str()
        .expect("recovery start should return an opaque verification ID")
        .to_owned();
    let recovery_case_id = started_payload["recovery_case_id"]
        .as_str()
        .expect("recovery start should return an opaque case ID")
        .to_owned();

    let resend = recovery_router
        .clone()
        .oneshot(make_recovery_request(
            Method::POST,
            format!("/api/v1/identity/recovery/{verification_id}/resend"),
            None,
            false,
        ))
        .await
        .expect("recovery resend route should respond");
    assert_eq!(
        resend.status(),
        axum::http::StatusCode::TOO_MANY_REQUESTS,
        "OTP delivery cooldown must be enforced across resend requests"
    );

    let premature_candidates = recovery_router
        .clone()
        .oneshot(make_recovery_request(
            Method::GET,
            format!("/api/v1/identity/recovery/cases/{recovery_case_id}/accounts"),
            None,
            false,
        ))
        .await
        .expect("premature candidate request should respond");
    assert_eq!(
        premature_candidates.status(),
        axum::http::StatusCode::NOT_FOUND,
        "account candidates must not be disclosed before OTP verification"
    );

    let recovery_code = last_code
        .lock()
        .expect("recovery test delivery lock should be available")
        .clone()
        .expect("recovery OTP should be captured");
    let cross_site_verify = recovery_router
        .clone()
        .oneshot(make_recovery_request(
            Method::POST,
            format!("/api/v1/identity/recovery/{verification_id}/verify"),
            Some(serde_json::json!({ "code": recovery_code })),
            true,
        ))
        .await
        .expect("cross-site verification request should respond");
    assert_eq!(
        cross_site_verify.status(),
        axum::http::StatusCode::FORBIDDEN
    );

    let recovery_code = last_code
        .lock()
        .expect("recovery test delivery lock should be available")
        .clone()
        .expect("recovery OTP should remain available for the valid request");
    let verified = recovery_router
        .clone()
        .oneshot(make_recovery_request(
            Method::POST,
            format!("/api/v1/identity/recovery/{verification_id}/verify"),
            Some(serde_json::json!({ "code": recovery_code })),
            false,
        ))
        .await
        .expect("recovery verify route should respond");
    assert_eq!(verified.status(), axum::http::StatusCode::OK);

    let candidates = recovery_router
        .clone()
        .oneshot(make_recovery_request(
            Method::GET,
            format!("/api/v1/identity/recovery/cases/{recovery_case_id}/accounts"),
            None,
            false,
        ))
        .await
        .expect("candidate list route should respond");
    assert_eq!(candidates.status(), axum::http::StatusCode::OK);
    assert_eq!(
        candidates.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-store"
    );
    let candidates_body = candidates
        .into_body()
        .collect()
        .await
        .expect("candidate response should be readable")
        .to_bytes();
    let candidates_payload: serde_json::Value =
        serde_json::from_slice(&candidates_body).expect("candidate response should be JSON");
    assert_eq!(candidates_payload["accounts"].as_array().unwrap().len(), 1);
    assert_eq!(
        candidates_payload["accounts"][0]["user_id"],
        registered.user_id
    );
    assert_eq!(candidates_payload["accounts"][0]["role"], "business_owner");

    let mut select_request = make_recovery_request(
        Method::POST,
        format!("/api/v1/identity/recovery/cases/{recovery_case_id}/select"),
        Some(serde_json::json!({ "user_id": registered.user_id })),
        false,
    );
    select_request.headers_mut().insert(
        header::COOKIE,
        format!("__Host-usahapintar_session={session_token}")
            .parse()
            .expect("session cookie header should be valid"),
    );
    let selected = recovery_router
        .clone()
        .oneshot(select_request)
        .await
        .expect("account selection route should respond");
    assert_eq!(selected.status(), axum::http::StatusCode::ACCEPTED);

    let case_status: String =
        sqlx::query_scalar("SELECT status FROM public.account_recovery_cases WHERE id = $1::uuid")
            .bind(&recovery_case_id)
            .fetch_one(&admin_pool)
            .await
            .expect("DBA should inspect recovery case state");
    assert_eq!(case_status, "under_review");

    let matched_signal_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.account_recovery_case_signals \
          WHERE recovery_case_id = $1::uuid AND signal_code = 'active_session_match'",
    )
    .bind(&recovery_case_id)
    .fetch_one(&admin_pool)
    .await
    .expect("DBA should inspect recorded prior-session signal");
    assert_eq!(matched_signal_count, 1);

    let existing_admin_id: Option<String> = sqlx::query_scalar(
        "SELECT id::text FROM public.users WHERE primary_role = 'admin' AND account_status = 'active'",
    )
    .fetch_optional(&admin_pool)
    .await
    .expect("existing Admin account should be queryable");
    let reviewer_id = if let Some(admin_id) = existing_admin_id {
        admin_id
    } else {
        let admin_email = format!("recovery-review-admin-{email}");
        sqlx::query_scalar(
            "INSERT INTO public.users (primary_role, account_status, email_normalized, initial_contact_channel, email_verified_at, initial_contact_verified_at, password_hash) \
             VALUES ('admin', 'active', $1, 'email', clock_timestamp(), clock_timestamp(), '$argon2id$test-review-hash') \
             RETURNING id::text",
        )
        .bind(&admin_email)
        .fetch_one(&admin_pool)
        .await
        .expect("a single active Admin reviewer should be provisioned for the test")
    };

    let reviewer_session_token = "b".repeat(64);
    let reviewer_session_hash = hash_session_token(&reviewer_session_token);
    sqlx::query(
        "INSERT INTO public.sessions (user_id, token_hash, expires_at) \
         VALUES ($1::uuid, $2, clock_timestamp() + INTERVAL '30 days')",
    )
    .bind(&reviewer_id)
    .bind(reviewer_session_hash.to_vec())
    .execute(&admin_pool)
    .await
    .expect("active Admin session should be created for review");

    let revoked_reviewer_session_token = "c".repeat(64);
    let revoked_reviewer_session_hash = hash_session_token(&revoked_reviewer_session_token);
    sqlx::query(
        "INSERT INTO public.sessions (user_id, token_hash, expires_at) \
         VALUES ($1::uuid, $2, clock_timestamp() + INTERVAL '30 days')",
    )
    .bind(&reviewer_id)
    .bind(revoked_reviewer_session_hash.to_vec())
    .execute(&admin_pool)
    .await
    .expect("a second Admin session should be created for revocation testing");
    sqlx::query(
        "UPDATE public.sessions SET revoked_at = clock_timestamp(), revocation_reason = 'logout' \
          WHERE token_hash = $1",
    )
    .bind(revoked_reviewer_session_hash.to_vec())
    .execute(&admin_pool)
    .await
    .expect("Admin session should be revoked for the negative test");

    let review_service = RecoveryReviewService::new(&pool);
    assert!(
        review_service
            .review_case(
                &recovery_case_id,
                &registered.user_id,
                &session_token,
                RecoveryReviewDecision::Approve,
                RecoveryReviewReason::OwnershipConfirmed,
            )
            .await
            .is_err(),
        "a non-Admin account must not approve recovery"
    );

    assert!(
        review_service
            .review_case(
                &recovery_case_id,
                &reviewer_id,
                &revoked_reviewer_session_token,
                RecoveryReviewDecision::Approve,
                RecoveryReviewReason::OwnershipConfirmed,
            )
            .await
            .is_err(),
        "a revoked Admin session must not approve recovery"
    );

    review_service
        .review_case(
            &recovery_case_id,
            &reviewer_id,
            &reviewer_session_token,
            RecoveryReviewDecision::Approve,
            RecoveryReviewReason::OwnershipConfirmed,
        )
        .await
        .expect("an active Admin may approve a selected recovery case");

    let reviewed_status: String =
        sqlx::query_scalar("SELECT status FROM public.account_recovery_cases WHERE id = $1::uuid")
            .bind(&recovery_case_id)
            .fetch_one(&admin_pool)
            .await
            .expect("DBA should inspect reviewed recovery case");
    assert_eq!(reviewed_status, "approved");

    let review_event_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.account_recovery_review_decisions WHERE recovery_case_id = $1::uuid",
    )
    .bind(&recovery_case_id)
    .fetch_one(&admin_pool)
    .await
    .expect("DBA should inspect the append-only review decision");
    assert_eq!(review_event_count, 1);

    let reset_url_capture = Arc::new(Mutex::new(None));
    let reset_destination_capture = Arc::new(Mutex::new(None));
    let reset_expiry_capture = Arc::new(Mutex::new(None));
    {
        let failed_delivery = CapturingPasswordResetDelivery {
            reset_url: reset_url_capture.clone(),
            destination: reset_destination_capture.clone(),
            expires_at_unix: reset_expiry_capture.clone(),
            status: DispatchStatus::Failed,
        };
        let failed_issuer = PasswordResetIssuanceService::new(
            &pool,
            keys.as_ref(),
            &failed_delivery,
            "https://app.example.test/reset-password".to_owned(),
        )
        .expect("valid HTTPS password reset URL should be accepted");
        let failed_result = failed_issuer
            .issue_for_approved_case(&recovery_case_id)
            .await
            .expect("failed delivery should still be recorded");
        assert_eq!(failed_result.dispatch_status, DispatchStatus::Failed);
    }

    let failed_delivery_token_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.password_reset_tokens \
          WHERE recovery_case_id = $1::uuid AND revocation_reason = 'delivery_failed'",
    )
    .bind(&recovery_case_id)
    .fetch_one(&admin_pool)
    .await
    .expect("failed delivery token should be revoked");
    assert_eq!(failed_delivery_token_count, 1);

    let reset_delivery = CapturingPasswordResetDelivery {
        reset_url: reset_url_capture.clone(),
        destination: reset_destination_capture.clone(),
        expires_at_unix: reset_expiry_capture.clone(),
        status: DispatchStatus::Accepted,
    };
    let reset_issuer = PasswordResetIssuanceService::new(
        &pool,
        keys.as_ref(),
        &reset_delivery,
        "https://app.example.test/reset-password".to_owned(),
    )
    .expect("valid HTTPS password reset URL should be accepted");
    let issue_result = reset_issuer
        .issue_for_approved_case(&recovery_case_id)
        .await
        .expect("approved recovery case should issue a reset token");
    assert_eq!(issue_result.dispatch_status, DispatchStatus::Accepted);
    assert!(
        reset_issuer
            .issue_for_approved_case(&recovery_case_id)
            .await
            .is_err(),
        "an active token must prevent duplicate reset delivery"
    );

    let delivered_url = reset_url_capture
        .lock()
        .expect("reset URL capture lock should be available")
        .clone()
        .expect("password reset provider should receive a URL");
    assert!(delivered_url.starts_with("https://app.example.test/reset-password#token="));
    assert!(!delivered_url.contains("?token="));
    let reset_token = delivered_url
        .split_once("#token=")
        .expect("reset token should be placed in the URL fragment")
        .1
        .to_owned();
    assert_eq!(reset_token.len(), 64);
    assert!(reset_token
        .chars()
        .all(|character| character.is_ascii_hexdigit()));

    assert_eq!(
        reset_destination_capture
            .lock()
            .expect("reset destination capture lock should be available")
            .as_deref(),
        Some(recovery_email.as_str())
    );
    assert_eq!(
        *reset_expiry_capture
            .lock()
            .expect("reset expiry capture lock should be available"),
        Some(issue_result.expires_at_unix)
    );

    let database_now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&admin_pool)
            .await
            .expect("database clock should be readable");
    assert!(issue_result.expires_at_unix > database_now);
    assert!(issue_result.expires_at_unix <= database_now + 15 * 60);

    let reset_uri = "/api/v1/identity/recovery/password-reset".to_owned();
    let reset_password = "recovery-password-reset-strong";
    let mismatch = recovery_router
        .clone()
        .oneshot(make_recovery_request(
            Method::POST,
            reset_uri.clone(),
            Some(serde_json::json!({
                "token": reset_token.clone(),
                "new_password": reset_password,
                "confirm_password": "different-password"
            })),
            false,
        ))
        .await
        .expect("mismatched reset password request should respond");
    assert_eq!(mismatch.status(), axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(
        mismatch.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-store"
    );

    let rejected_password = recovery_router
        .clone()
        .oneshot(make_recovery_request(
            Method::POST,
            reset_uri.clone(),
            Some(serde_json::json!({
                "token": reset_token.clone(),
                "new_password": "short",
                "confirm_password": "short"
            })),
            false,
        ))
        .await
        .expect("weak reset password request should respond");
    assert_eq!(
        rejected_password.status(),
        axum::http::StatusCode::BAD_REQUEST
    );

    let cross_site_reset = recovery_router
        .clone()
        .oneshot(make_recovery_request(
            Method::POST,
            reset_uri.clone(),
            Some(serde_json::json!({
                "token": reset_token.clone(),
                "new_password": reset_password,
                "confirm_password": reset_password
            })),
            true,
        ))
        .await
        .expect("cross-site reset request should respond");
    assert_eq!(cross_site_reset.status(), axum::http::StatusCode::FORBIDDEN);
    assert_eq!(
        cross_site_reset
            .headers()
            .get(header::CACHE_CONTROL)
            .unwrap(),
        "no-store"
    );

    let make_valid_reset_request = || {
        make_recovery_request(
            Method::POST,
            reset_uri.clone(),
            Some(serde_json::json!({
                "token": reset_token.clone(),
                "new_password": reset_password,
                "confirm_password": reset_password
            })),
            false,
        )
    };
    let first_reset = recovery_router.clone().oneshot(make_valid_reset_request());
    let second_reset = recovery_router.clone().oneshot(make_valid_reset_request());
    let (first_reset, second_reset) = tokio::join!(first_reset, second_reset);
    let first_reset = first_reset.expect("first concurrent reset request should complete");
    let second_reset = second_reset.expect("second concurrent reset request should complete");
    let first_succeeded = first_reset.status() == axum::http::StatusCode::OK;
    let second_succeeded = second_reset.status() == axum::http::StatusCode::OK;
    assert_ne!(
        first_succeeded, second_succeeded,
        "concurrent password reset attempts must have exactly one winner"
    );

    let success_response = if first_succeeded {
        first_reset
    } else {
        second_reset
    };
    assert_eq!(
        success_response
            .headers()
            .get(header::CACHE_CONTROL)
            .unwrap(),
        "no-store"
    );
    assert!(
        success_response
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("__Host-usahapintar_session=")
                && value.contains("Max-Age=0")),
        "successful password reset should clear the current browser cookie"
    );

    let replay = recovery_router
        .clone()
        .oneshot(make_valid_reset_request())
        .await
        .expect("replayed reset token request should respond");
    assert_eq!(replay.status(), axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(
        replay.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-store"
    );

    let updated_password_hash: String =
        sqlx::query_scalar("SELECT password_hash FROM public.users WHERE id = $1::uuid")
            .bind(&registered.user_id)
            .fetch_one(&admin_pool)
            .await
            .expect("updated password hash should be present");
    assert!(
        verify_password(reset_password.as_bytes(), &updated_password_hash)
            .expect("updated Argon2id password hash should be valid")
    );

    let active_session_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.sessions WHERE user_id = $1::uuid AND revoked_at IS NULL",
    )
    .bind(&registered.user_id)
    .fetch_one(&admin_pool)
    .await
    .expect("active session count should be queryable");
    assert_eq!(active_session_count, 0);

    let reset_revoked_session_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.sessions WHERE user_id = $1::uuid AND revocation_reason = 'password_reset'",
    )
    .bind(&registered.user_id)
    .fetch_one(&admin_pool)
    .await
    .expect("password-reset session revocations should be queryable");
    assert_eq!(reset_revoked_session_count, 1);

    let completed_case_status: String =
        sqlx::query_scalar("SELECT status FROM public.account_recovery_cases WHERE id = $1::uuid")
            .bind(&recovery_case_id)
            .fetch_one(&admin_pool)
            .await
            .expect("completed recovery case should be queryable");
    assert_eq!(completed_case_status, "completed");

    let consumed_token_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.password_reset_tokens \
          WHERE recovery_case_id = $1::uuid AND consumed_at IS NOT NULL AND revoked_at IS NULL",
    )
    .bind(&recovery_case_id)
    .fetch_one(&admin_pool)
    .await
    .expect("consumed reset token should be queryable");
    assert_eq!(consumed_token_count, 1);

    let reset_audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.identity_audit_events \
          WHERE action_code = 'identity.password.reset.completed' AND subject_user_id = $1::uuid",
    )
    .bind(&registered.user_id)
    .fetch_one(&admin_pool)
    .await
    .expect("password reset audit event should be queryable");
    assert_eq!(reset_audit_count, 1);

    admin_pool.close().await;
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL application and DBA principals"]
async fn admin_http_authentication_uses_isolated_secure_cookies_end_to_end() {
    use serde_json::json;
    use sqlx::postgres::PgPoolOptions;
    use usahapintar_api::{
        app_with_authentication_and_admin,
        identity::{
            crypto::{encrypt_admin_totp_secret, key_version_label, AdminTotpKeyring, KeyVersion},
            http::{AdminAuthenticationApiState, AuthenticationApiState},
            password::hash_password,
            totp::code_for_step,
            AuthenticationRateLimitKey,
        },
    };

    const OPERATOR_ID: &str = "00000000-0000-4000-8000-000000000002";
    const ADMIN_PASSWORD: &str = "Admin-HTTP-Integration-Password-2026!";

    let pool = application_pool().await;
    let admin_url = std::env::var("CI_TARGET_ADMIN_DATABASE_URL")
        .expect("CI must provide the disposable DBA connection");
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_url)
        .await
        .expect("disposable DBA connection should succeed");

    let existing_admin: Option<(String, String)> = sqlx::query_as(
        "SELECT id::text, email_normalized FROM public.users \
          WHERE primary_role = 'admin' AND account_status = 'active'",
    )
    .fetch_optional(&admin_pool)
    .await
    .expect("active Admin fixture should be queryable");
    let (admin_id, admin_email) = if let Some(existing) = existing_admin {
        existing
    } else {
        let email = format!(
            "admin-http-{}-{}@example.test",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos()
        );
        let user_id: String = sqlx::query_scalar(
            "INSERT INTO public.users ( \
                 primary_role, account_status, email_normalized, initial_contact_channel, \
                 email_verified_at, initial_contact_verified_at, password_hash \
             ) VALUES ( \
                 'admin', 'active', $1, 'email', clock_timestamp(), clock_timestamp(), $2 \
             ) RETURNING id::text",
        )
        .bind(&email)
        .bind(hash_password(ADMIN_PASSWORD.as_bytes()).expect("Admin password should hash"))
        .fetch_one(&admin_pool)
        .await
        .expect("a single active Admin fixture should be provisioned");
        (user_id, email)
    };

    let password_hash =
        hash_password(ADMIN_PASSWORD.as_bytes()).expect("Admin password should hash");
    sqlx::query("UPDATE public.users SET password_hash = $2 WHERE id = $1::uuid")
        .bind(&admin_id)
        .bind(password_hash)
        .execute(&admin_pool)
        .await
        .expect("Admin test password should be installed");

    let key_hex = "44".repeat(32);
    let keyring = Arc::new(
        AdminTotpKeyring::from_hex_entries(KeyVersion(1), &[(KeyVersion(1), key_hex.as_str())])
            .expect("test Admin keyring should be valid"),
    );
    let totp_secret = [0x6B_u8; 20];
    let encrypted = encrypt_admin_totp_secret(keyring.as_ref(), &admin_id, &totp_secret)
        .expect("Admin TOTP secret should be encrypted");
    let key_version =
        key_version_label(encrypted.key_version).expect("Admin key version label should be valid");
    let current_step: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()) / 30)::bigint")
            .fetch_one(&admin_pool)
            .await
            .expect("current TOTP step should be available");

    let credential_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM public.admin_totp_credentials WHERE user_id = $1::uuid)",
    )
    .bind(&admin_id)
    .fetch_one(&admin_pool)
    .await
    .expect("Admin TOTP credential state should be queryable");
    let prepared: bool = if credential_exists {
        sqlx::query_scalar(
            "SELECT public.reset_admin_totp_enrollment( \
                $1::uuid, $2, $3, $4, 'authenticator_lost', $5::uuid \
            )",
        )
        .bind(&admin_id)
        .bind(&encrypted.ciphertext)
        .bind(encrypted.nonce.to_vec())
        .bind(&key_version)
        .bind(OPERATOR_ID)
        .fetch_one(&admin_pool)
        .await
        .expect("operator should reset the Admin TOTP credential")
    } else {
        sqlx::query_scalar(
            "SELECT public.prepare_admin_totp_enrollment($1::uuid, $2, $3, $4, $5::uuid)",
        )
        .bind(&admin_id)
        .bind(&encrypted.ciphertext)
        .bind(encrypted.nonce.to_vec())
        .bind(&key_version)
        .bind(OPERATOR_ID)
        .fetch_one(&admin_pool)
        .await
        .expect("operator should prepare the initial Admin TOTP credential")
    };
    assert!(prepared);

    let activated: bool =
        sqlx::query_scalar("SELECT public.activate_admin_totp_enrollment($1::uuid, $2, $3::uuid)")
            .bind(&admin_id)
            .bind(current_step - 1)
            .bind(OPERATOR_ID)
            .fetch_one(&admin_pool)
            .await
            .expect("operator should activate the Admin TOTP credential");
    assert!(activated);

    let rate_limit_key = AuthenticationRateLimitKey::from_hex(&"ab".repeat(32))
        .expect("test rate-limit key should be valid");
    let owner_authentication = AuthenticationApiState::new(pool.clone(), rate_limit_key.clone());
    let admin_authentication =
        AdminAuthenticationApiState::new(pool.clone(), rate_limit_key, keyring);
    let app =
        app_with_authentication_and_admin(pool.clone(), owner_authentication, admin_authentication);

    let make_request = |method: Method,
                        uri: &'static str,
                        body: Option<serde_json::Value>,
                        cookie: Option<String>| {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::HOST, "app.example.test")
            .header(header::ORIGIN, "https://app.example.test");
        if body.is_some() {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
        }
        if let Some(cookie) = cookie {
            builder = builder.header(header::COOKIE, cookie);
        }
        let mut request = builder
            .body(Body::from(
                body.map(|value| value.to_string()).unwrap_or_default(),
            ))
            .expect("Admin HTTP request should be valid");
        request.extensions_mut().insert(ConnectInfo(
            "127.0.0.1:40000"
                .parse::<SocketAddr>()
                .expect("test socket address should be valid"),
        ));
        request
    };

    let password_response = app
        .clone()
        .oneshot(make_request(
            Method::POST,
            "/api/v1/identity/admin/login",
            Some(json!({
                "identifier": admin_email,
                "password": ADMIN_PASSWORD
            })),
            None,
        ))
        .await
        .expect("Admin password login route should respond");
    assert_eq!(password_response.status(), axum::http::StatusCode::ACCEPTED);
    assert_eq!(
        password_response
            .headers()
            .get(header::CACHE_CONTROL)
            .unwrap(),
        "no-store"
    );
    let challenge_cookie = password_response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .find_map(|value| {
            let cookie = value.to_str().ok()?;
            cookie
                .starts_with("__Host-usahapintar_admin_challenge=")
                .then(|| cookie.split(';').next().unwrap_or_default().to_owned())
        })
        .expect("password login should issue the HttpOnly Admin challenge cookie");
    let challenge_header = password_response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .find(|value| {
            value
                .to_str()
                .is_ok_and(|cookie| cookie.starts_with("__Host-usahapintar_admin_challenge="))
        })
        .expect("Admin challenge Set-Cookie header should exist")
        .to_str()
        .expect("Admin challenge cookie header should be valid");
    assert!(challenge_header.contains("HttpOnly"));
    assert!(challenge_header.contains("Secure"));
    assert!(challenge_header.contains("SameSite=Strict"));
    assert!(!challenge_header.to_ascii_lowercase().contains("domain="));

    let password_body = password_response
        .into_body()
        .collect()
        .await
        .expect("password login response body should be readable")
        .to_bytes();
    let password_payload: serde_json::Value =
        serde_json::from_slice(&password_body).expect("password login response should be JSON");
    assert!(password_payload.get("token").is_none());
    assert!(password_payload.get("challenge_token").is_none());

    let totp_code = code_for_step(&totp_secret, current_step as u64)
        .expect("current Admin TOTP code should be generated");
    let totp_response = app
        .clone()
        .oneshot(make_request(
            Method::POST,
            "/api/v1/identity/admin/login/totp",
            Some(json!({ "code": totp_code })),
            Some(challenge_cookie),
        ))
        .await
        .expect("Admin TOTP completion route should respond");
    assert_eq!(totp_response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        totp_response.headers().get(header::CACHE_CONTROL).unwrap(),
        "no-store"
    );
    let session_cookie = totp_response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .find_map(|value| {
            let cookie = value.to_str().ok()?;
            cookie
                .starts_with("__Host-usahapintar_admin_session=")
                .then(|| cookie.split(';').next().unwrap_or_default().to_owned())
        })
        .expect("successful TOTP should issue the isolated Admin session cookie");
    let session_header = totp_response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .find(|value| {
            value
                .to_str()
                .is_ok_and(|cookie| cookie.starts_with("__Host-usahapintar_admin_session="))
        })
        .expect("Admin session Set-Cookie header should exist")
        .to_str()
        .expect("Admin session cookie header should be valid");
    assert!(session_header.contains("Path=/"));
    assert!(session_header.contains("Max-Age=28800"));
    assert!(session_header.contains("HttpOnly"));
    assert!(session_header.contains("Secure"));
    assert!(session_header.contains("SameSite=Strict"));
    assert!(!session_header.to_ascii_lowercase().contains("domain="));

    let totp_body = totp_response
        .into_body()
        .collect()
        .await
        .expect("TOTP response body should be readable")
        .to_bytes();
    let totp_payload: serde_json::Value =
        serde_json::from_slice(&totp_body).expect("TOTP response should be JSON");
    assert!(totp_payload.get("token").is_none());
    assert!(totp_payload.get("session_token").is_none());

    let current_admin = app
        .clone()
        .oneshot(make_request(
            Method::GET,
            "/api/v1/identity/admin/me",
            None,
            Some(session_cookie.clone()),
        ))
        .await
        .expect("Admin current-session route should respond");
    assert_eq!(current_admin.status(), axum::http::StatusCode::OK);
    let current_admin_body = current_admin
        .into_body()
        .collect()
        .await
        .expect("Admin session response should be readable")
        .to_bytes();
    let current_admin_payload: serde_json::Value =
        serde_json::from_slice(&current_admin_body).expect("Admin session response should be JSON");
    assert_eq!(current_admin_payload["user_id"], admin_id);
    assert_eq!(current_admin_payload["role"], "admin");

    let owner_route_with_admin_cookie = app
        .clone()
        .oneshot(make_request(
            Method::GET,
            "/api/v1/identity/me",
            None,
            Some(session_cookie.clone()),
        ))
        .await
        .expect("Owner session route should reject an Admin cookie");
    assert_eq!(
        owner_route_with_admin_cookie.status(),
        axum::http::StatusCode::UNAUTHORIZED
    );

    let logout = app
        .clone()
        .oneshot(make_request(
            Method::POST,
            "/api/v1/identity/admin/logout",
            None,
            Some(session_cookie.clone()),
        ))
        .await
        .expect("Admin logout route should respond");
    assert_eq!(logout.status(), axum::http::StatusCode::NO_CONTENT);
    assert_eq!(
        logout
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter(|value| {
                value
                    .to_str()
                    .is_ok_and(|cookie| cookie.contains("Max-Age=0"))
            })
            .count(),
        2,
        "logout should clear both Admin cookies"
    );

    let after_logout = app
        .oneshot(make_request(
            Method::GET,
            "/api/v1/identity/admin/me",
            None,
            Some(session_cookie),
        ))
        .await
        .expect("Admin route should reject a logged-out session");
    assert_eq!(after_logout.status(), axum::http::StatusCode::UNAUTHORIZED);

    admin_pool.close().await;
    pool.close().await;
}
