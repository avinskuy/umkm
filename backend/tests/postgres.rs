use std::{future::Future, pin::Pin, sync::Mutex};

use sqlx::{postgres::PgPoolOptions, raw_sql, PgPool};
use usahapintar_api::{
    database::{connect, migrations::run, DatabaseConfig},
    identity::{
        crypto::{
            encrypt_admin_totp_secret, key_version_label, CryptoError, KeyProvider, KeyPurpose,
            KeyVersion, SecretKey, VersionedKey,
        },
        flowkirim::{FlowKirimProviderOutcome, FlowKirimVerifyProvider},
        password::hash_password,
        AdminAuthenticationService, AuthenticationRateLimitKey, ClientRegistration, ClientRole,
        ContactChannel, DispatchStatus, OtpDeliveryMessage, OtpDeliveryProvider,
        VerificationService,
    },
};
use zeroize::Zeroizing;

struct VerificationTestKeys;

impl KeyProvider for VerificationTestKeys {
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

struct FakeOtpDelivery {
    messages: Mutex<Vec<OtpDeliveryMessage>>,
    status: DispatchStatus,
}

impl OtpDeliveryProvider for FakeOtpDelivery {
    fn dispatch<'a>(
        &'a self,
        message: OtpDeliveryMessage,
    ) -> Pin<Box<dyn Future<Output = DispatchStatus> + Send + 'a>> {
        Box::pin(async move {
            self.messages
                .lock()
                .expect("fake delivery message lock should be available")
                .push(message);
            self.status
        })
    }
}

struct FakeFlowKirim;

impl FlowKirimVerifyProvider for FakeFlowKirim {
    fn start<'a>(
        &'a self,
        _destination: Zeroizing<String>,
        _idempotency_key: String,
    ) -> Pin<Box<dyn Future<Output = FlowKirimProviderOutcome<String>> + Send + 'a>> {
        Box::pin(async {
            FlowKirimProviderOutcome::Accepted("flowkirim-test-request-001".to_owned())
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

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL database; enabled explicitly in CI"]
async fn postgres_connection_and_identity_schema_invariants_work() {
    let config = DatabaseConfig::from_env().expect("CI must provide DATABASE_URL");
    let pool = connect(&config)
        .await
        .expect("PostgreSQL connection should succeed");

    run(&pool)
        .await
        .expect("SQLx migrations should complete successfully");
    run(&pool)
        .await
        .expect("running the migration runner again should be safe");

    let result: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("database should execute a simple query");
    assert_eq!(result, 1);

    assert_user_constraints(&pool).await;
    assert_role_and_registration_provenance_are_immutable(&pool).await;
    assert_contact_reservations_and_release(&pool).await;
    assert_password_only_authentication_invariants(&pool).await;
    assert_client_creation_function(&pool).await;
    assert_profile_and_policy_records(&pool).await;
    assert_verification_schema_invariants(&pool).await;
    assert_verification_database_operations(&pool).await;
    assert_flowkirim_verification_database_operations(&pool).await;
    assert_otp_resend_during_verification_lock(&pool).await;
    assert_authentication_session_invariants(&pool).await;
    assert_authentication_database_operations(&pool).await;
    assert_account_recovery_schema_invariants(&pool).await;
    assert_identity_audit_event_invariants(&pool).await;
    assert_single_admin_totp_schema(&pool).await;
    assert_admin_totp_operator_workflows(&pool).await;
    assert_admin_authentication_database_runtime(&pool).await;
    assert_admin_authentication_service(&pool).await;
    assert_identity_abuse_rate_limits(&pool).await;
    assert_database_principal_bootstrap(&pool).await;

    pool.close().await;
    assert_pool_closed(&pool);
}

async fn assert_user_constraints(pool: &PgPool) {
    let pending_email = insert_pending_email(
        pool,
        "business_owner",
        "pending-owner@example.test",
        Some("encoded-password-hash"),
    )
    .await
    .expect("valid pending email registration should be accepted");

    assert!(
        insert_pending_email(
            pool,
            "investor",
            "pending-owner@example.test",
            Some("another-password-hash"),
        )
        .await
        .is_err(),
        "a reserved email must not be registered twice"
    );

    let pending_phone = insert_pending_phone(
        pool,
        "investor",
        "+6281234567890",
        Some("encoded-password-hash"),
    )
    .await
    .expect("valid pending phone registration should be accepted");

    assert!(
        insert_pending_phone(
            pool,
            "business_owner",
            "+6281234567890",
            Some("another-password-hash"),
        )
        .await
        .is_err(),
        "a reserved phone number must not be registered twice"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.users (primary_role, account_status, initial_contact_channel, pending_expires_at) \
             VALUES ('unknown_role', 'pending_verification', 'email', now() + INTERVAL '30 minutes')"
        )
        .execute(pool)
        .await
        .is_err(),
        "unknown roles must be rejected"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.users (primary_role, account_status, initial_contact_channel, pending_expires_at) \
             VALUES ('business_owner', 'unknown_status', 'email', now() + INTERVAL '30 minutes')"
        )
        .execute(pool)
        .await
        .is_err(),
        "unknown account statuses must be rejected"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.users (primary_role, account_status, initial_contact_channel, pending_expires_at) \
             VALUES ('business_owner', 'pending_verification', 'email', now() + INTERVAL '30 minutes')"
        )
        .execute(pool)
        .await
        .is_err(),
        "pending accounts must have their selected initial contact"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.users (primary_role, account_status, email_normalized, initial_contact_channel, pending_expires_at, email_verified_at) \
             VALUES ('business_owner', 'pending_verification', 'verified-too-early@example.test', 'email', now() + INTERVAL '30 minutes', now())"
        )
        .execute(pool)
        .await
        .is_err(),
        "pending accounts cannot claim that the initial contact is already verified"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.users (primary_role, account_status, email_normalized, initial_contact_channel, pending_expires_at, email_verified_at, initial_contact_verified_at) \
             VALUES ('investor', 'pending_verification', 'verified-pending@example.test', 'email', now() + INTERVAL '30 minutes', now(), now())"
        )
        .execute(pool)
        .await
        .is_err(),
        "pending accounts cannot have verification timestamps"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.users (primary_role, account_status, email_normalized, initial_contact_channel, email_verified_at, password_hash) \
             VALUES ('investor', 'active', 'active-unverified@example.test', 'email', NULL, 'encoded-hash')"
        )
        .execute(pool)
        .await
        .is_err(),
        "active accounts require verified contact and registration provenance"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.users (primary_role, account_status, email_normalized, initial_contact_channel, email_verified_at, initial_contact_verified_at) \
             VALUES ('investor', 'active', 'missing-auth-method@example.test', 'email', now(), now())"
        )
        .execute(pool)
        .await
        .is_err(),
        "client accounts must have a local password hash"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.users (primary_role, account_status, phone_e164, initial_contact_channel, pending_expires_at) \
             VALUES ('investor', 'pending_verification', '081234567890', 'phone', now() + INTERVAL '30 minutes')"
        )
        .execute(pool)
        .await
        .is_err(),
        "phone numbers must use E.164 format"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.users (primary_role, account_status, email_normalized, initial_contact_channel, email_verified_at, initial_contact_verified_at, password_hash) \
             VALUES ('investor', 'active', 'orphan-verification@example.test', 'email', now(), now(), '   ')"
        )
        .execute(pool)
        .await
        .is_err(),
        "password hashes cannot be empty"
    );

    let active_id = insert_active_email(pool, "active-contact@example.test").await;
    assert!(
        sqlx::query("UPDATE public.users SET account_status = 'deactivated' WHERE id = $1::uuid")
            .bind(&active_id)
            .execute(pool)
            .await
            .is_ok(),
        "active accounts may be deactivated without releasing their contacts"
    );

    assert!(
        insert_pending_email(
            pool,
            "business_owner",
            "active-contact@example.test",
            Some("another-hash"),
        )
        .await
        .is_err(),
        "deactivated accounts must continue reserving their primary contact"
    );

    let deletion_pending_id =
        insert_active_email(pool, "deletion-pending-contact@example.test").await;
    sqlx::query("UPDATE public.users SET account_status = 'deletion_pending' WHERE id = $1::uuid")
        .bind(&deletion_pending_id)
        .execute(pool)
        .await
        .expect("deletion-pending transition should preserve a verified account");

    assert!(
        insert_pending_email(
            pool,
            "investor",
            "deletion-pending-contact@example.test",
            Some("another-hash"),
        )
        .await
        .is_err(),
        "deletion-pending accounts must continue reserving their primary contact"
    );

    assert!(
        sqlx::query(
            "UPDATE public.users SET account_status = 'deletion_processing' WHERE id = $1::uuid"
        )
        .bind(&deletion_pending_id)
        .execute(pool)
        .await
        .is_err(),
        "permanent deletion cannot begin before the 30-day waiting period"
    );

    sqlx::query("UPDATE public.users SET account_status = 'active' WHERE id = $1::uuid")
        .bind(&deletion_pending_id)
        .execute(pool)
        .await
        .expect("a deletion request may be cancelled before processing");

    let early_expiry_id = insert_pending_email(
        pool,
        "business_owner",
        "not-yet-expired@example.test",
        Some("temporary-hash"),
    )
    .await
    .expect("pending account should be created before expiry");

    assert!(
        sqlx::query(
            "UPDATE public.users \
             SET account_status = 'pending_expired', \
                 email_normalized = NULL, \
                 email_verified_at = NULL, \
                 password_hash = NULL, \
                 pending_expires_at = NULL \
             WHERE id = $1::uuid"
        )
        .bind(&early_expiry_id)
        .execute(pool)
        .await
        .is_err(),
        "pending registration cannot release its contact before the 30-minute deadline"
    );

    let expired_id = insert_expired_pending_email(pool, "released-contact@example.test")
        .await
        .expect("expired pending account should be created for cleanup test");

    assert!(
        sqlx::query(
            "UPDATE public.users \
             SET account_status = 'active', \
                 email_verified_at = now(), \
                 initial_contact_verified_at = now(), \
                 pending_expires_at = NULL \
             WHERE id = $1::uuid"
        )
        .bind(&expired_id)
        .execute(pool)
        .await
        .is_err(),
        "an expired pending registration cannot be activated"
    );

    sqlx::query(
        "UPDATE public.users \
         SET account_status = 'pending_expired', \
             email_normalized = NULL, \
             email_verified_at = NULL, \
             phone_e164 = NULL, \
             phone_verified_at = NULL, \
             initial_contact_verified_at = NULL, \
             password_hash = NULL, \
             pending_expires_at = NULL \
         WHERE id = $1::uuid",
    )
    .bind(&expired_id)
    .execute(pool)
    .await
    .expect("expired pending account should release contact atomically");

    insert_pending_email(
        pool,
        "investor",
        "released-contact@example.test",
        Some("new-password-hash"),
    )
    .await
    .expect("released contact may be reserved by a new registration");

    assert!(
        sqlx::query(
            "UPDATE public.users \
             SET account_status = 'pending_verification', \
                 email_normalized = 'resurrected-contact@example.test', \
                 password_hash = 'new-hash', \
                 pending_expires_at = now() + INTERVAL '30 minutes' \
             WHERE id = $1::uuid"
        )
        .bind(&expired_id)
        .execute(pool)
        .await
        .is_err(),
        "pending_expired accounts cannot be reopened"
    );

    let _ = pending_email;
    let _ = pending_phone;
}

async fn assert_role_and_registration_provenance_are_immutable(pool: &PgPool) {
    let pending_id = insert_pending_email(
        pool,
        "business_owner",
        "immutable-role@example.test",
        Some("encoded-password-hash"),
    )
    .await
    .expect("pending account should be created");

    assert!(
        sqlx::query(
            "UPDATE public.users SET pending_expires_at = clock_timestamp() + INTERVAL '60 minutes' WHERE id = $1::uuid"
        )
        .bind(&pending_id)
        .execute(pool)
        .await
        .is_err(),
        "pending registration expiry must not be extended or reset"
    );

    assert!(
        sqlx::query("UPDATE public.users SET primary_role = 'investor' WHERE id = $1::uuid")
            .bind(&pending_id)
            .execute(pool)
            .await
            .is_err(),
        "primary role must be immutable"
    );

    assert!(
        sqlx::query(
            "UPDATE public.users SET initial_contact_channel = 'phone' WHERE id = $1::uuid"
        )
        .bind(&pending_id)
        .execute(pool)
        .await
        .is_err(),
        "initial contact channel must be immutable"
    );

    let activation_id = insert_pending_email(
        pool,
        "investor",
        "activation-contact@example.test",
        Some("encoded-password-hash"),
    )
    .await
    .expect("pending account should be created before activation");

    assert!(
        sqlx::query(
            "UPDATE public.users SET initial_contact_verified_at = now() WHERE id = $1::uuid"
        )
        .bind(&activation_id)
        .execute(pool)
        .await
        .is_err(),
        "initial contact provenance cannot be set before the account is activated"
    );

    sqlx::query(
        "UPDATE public.users \
         SET account_status = 'active', \
             email_verified_at = now(), \
             initial_contact_verified_at = now(), \
             pending_expires_at = NULL \
         WHERE id = $1::uuid",
    )
    .bind(&activation_id)
    .execute(pool)
    .await
    .expect("verified initial contact should allow atomic activation");

    assert!(
        sqlx::query(
            "UPDATE public.users SET initial_contact_verified_at = now() + INTERVAL '1 second' WHERE id = $1::uuid"
        )
        .bind(&activation_id)
        .execute(pool)
        .await
        .is_err(),
        "initial contact verification provenance must be immutable after activation"
    );

    assert!(
        sqlx::query(
            "UPDATE public.users SET initial_contact_verified_at = NULL WHERE id = $1::uuid"
        )
        .bind(&activation_id)
        .execute(pool)
        .await
        .is_err(),
        "initial contact verification provenance cannot be cleared during normal account use"
    );
}

async fn assert_contact_reservations_and_release(pool: &PgPool) {
    let (first, second) = tokio::join!(
        insert_pending_email(
            pool,
            "business_owner",
            "concurrent-email@example.test",
            Some("hash-one"),
        ),
        insert_pending_email(
            pool,
            "investor",
            "concurrent-email@example.test",
            Some("hash-two"),
        )
    );

    assert_eq!(
        (first.is_ok() as usize) + (second.is_ok() as usize),
        1,
        "only one concurrent registration may reserve the same email"
    );

    let (first, second) = tokio::join!(
        insert_pending_phone(pool, "business_owner", "+6281234567891", Some("hash-one"),),
        insert_pending_phone(pool, "investor", "+6281234567891", Some("hash-two"),)
    );

    assert_eq!(
        (first.is_ok() as usize) + (second.is_ok() as usize),
        1,
        "only one concurrent registration may reserve the same phone number"
    );
}

async fn assert_password_only_authentication_invariants(pool: &PgPool) {
    let user_id: String =
        sqlx::query_scalar("SELECT public.create_client_user($1, $2, $3, $4, $5)::text")
            .bind("business_owner")
            .bind("email")
            .bind("password-only-owner@example.test")
            .bind(Option::<String>::None)
            .bind("encoded-password-hash")
            .fetch_one(pool)
            .await
            .expect("password registration should create a pending client account");

    let account_state: (String, Option<String>) = sqlx::query_as(
        "SELECT account_status, password_hash FROM public.users WHERE id = $1::uuid",
    )
    .bind(&user_id)
    .fetch_one(pool)
    .await
    .expect("created account state should be queryable");
    assert_eq!(account_state.0, "pending_verification");
    assert_eq!(account_state.1.as_deref(), Some("encoded-password-hash"));

    let legacy_identity_table_exists: bool =
        sqlx::query_scalar("SELECT pg_catalog.to_regclass('public.user_identities') IS NOT NULL")
            .fetch_one(pool)
            .await
            .expect("legacy provider identity table state should be queryable");
    assert!(
        !legacy_identity_table_exists,
        "provider identity storage must be absent"
    );

    assert!(
        sqlx::query_scalar::<_, String>(
            "SELECT public.create_client_user($1, $2, $3, $4, $5)::text",
        )
        .bind("investor")
        .bind("email")
        .bind("passwordless-owner@example.test")
        .bind(Option::<String>::None)
        .bind(Option::<String>::None)
        .fetch_one(pool)
        .await
        .is_err(),
        "public registration must reject accounts without a local password hash"
    );

    let signatures: (bool, bool) = sqlx::query_as(
        "SELECT pg_catalog.to_regprocedure('public.create_client_user(text,text,text,text,text)') IS NOT NULL, \
                pg_catalog.to_regprocedure('public.create_client_user(text,text,text,text,text,text)') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .expect("registration function signatures should be queryable");
    assert_eq!(signatures, (true, false));
}

async fn assert_client_creation_function(pool: &PgPool) {
    assert!(
        sqlx::query_scalar::<_, String>(
            "SELECT public.create_client_user($1, $2, $3, $4, $5)::text",
        )
        .bind("admin")
        .bind("email")
        .bind("not-allowed-admin@example.test")
        .bind(Option::<String>::None)
        .bind("encoded-password-hash")
        .fetch_one(pool)
        .await
        .is_err(),
        "public client creation function must reject Admin role"
    );

    assert!(
        sqlx::query_scalar::<_, String>(
            "SELECT public.create_client_user($1, $2, $3, $4, $5)::text",
        )
        .bind("business_owner")
        .bind("email")
        .bind("missing-password@example.test")
        .bind(Option::<String>::None)
        .bind(Option::<String>::None)
        .fetch_one(pool)
        .await
        .is_err(),
        "client account creation requires a local password hash"
    );

    assert!(
        sqlx::query_scalar::<_, String>(
            "SELECT public.create_client_user($1, $2, $3, $4, $5)::text",
        )
        .bind("business_owner")
        .bind("email")
        .bind("empty-password-hash@example.test")
        .bind(Option::<String>::None)
        .bind("   ")
        .fetch_one(pool)
        .await
        .is_err(),
        "client account creation must reject an empty password hash"
    );
}

async fn insert_pending_email(
    pool: &PgPool,
    role: &str,
    email: &str,
    password_hash: Option<&str>,
) -> Result<String, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO public.users ( \
             primary_role, account_status, email_normalized, phone_e164, \
             initial_contact_channel, password_hash, pending_expires_at \
         ) \
         VALUES ($1, 'pending_verification', $2, NULL, 'email', $3, now() + INTERVAL '30 minutes') \
         RETURNING id::text",
    )
    .bind(role)
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await
}

async fn insert_expired_pending_email(pool: &PgPool, email: &str) -> Result<String, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO public.users ( \
             primary_role, account_status, email_normalized, phone_e164, \
             initial_contact_channel, password_hash, pending_expires_at \
         ) \
         VALUES ($1, 'pending_verification', $2, NULL, 'email', $3, now() - INTERVAL '1 second') \
         RETURNING id::text",
    )
    .bind("business_owner")
    .bind(email)
    .bind(Some("expired-password-hash"))
    .fetch_one(pool)
    .await
}

async fn insert_pending_phone(
    pool: &PgPool,
    role: &str,
    phone: &str,
    password_hash: Option<&str>,
) -> Result<String, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO public.users ( \
             primary_role, account_status, email_normalized, phone_e164, \
             initial_contact_channel, password_hash, pending_expires_at \
         ) \
         VALUES ($1, 'pending_verification', NULL, $2, 'phone', $3, now() + INTERVAL '30 minutes') \
         RETURNING id::text",
    )
    .bind(role)
    .bind(phone)
    .bind(password_hash)
    .fetch_one(pool)
    .await
}

async fn insert_active_email(pool: &PgPool, email: &str) -> String {
    sqlx::query_scalar(
        "INSERT INTO public.users ( \
             primary_role, account_status, email_normalized, initial_contact_channel, \
             email_verified_at, initial_contact_verified_at, password_hash \
         ) \
         VALUES ('business_owner', 'active', $1, 'email', now(), now(), 'encoded-password-hash') \
         RETURNING id::text",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("active account with verified initial contact should be accepted")
}

fn assert_pool_closed(pool: &PgPool) {
    assert!(pool.is_closed());
}

async fn assert_verification_schema_invariants(pool: &PgPool) {
    let pending_user_id = insert_pending_email(
        pool,
        "business_owner",
        "verification-schema-owner@example.test",
        Some("encoded-password-hash"),
    )
    .await
    .expect("pending account should be available for registration verification");

    let session_id: String = sqlx::query_scalar(
        "INSERT INTO public.verification_sessions (              user_id, purpose, channel, destination_ciphertext, destination_nonce,              encryption_key_version, destination_hmac, hmac_key_version, expires_at          )          VALUES (              $1::uuid, 'registration', 'email', $2, $3, 'enc-v1', $4, 'hmac-v1',              clock_timestamp() + INTERVAL '20 minutes'          )          RETURNING id::text",
    )
    .bind(&pending_user_id)
    .bind(vec![1_u8])
    .bind(vec![2_u8])
    .bind(vec![3_u8])
    .fetch_one(pool)
    .await
    .expect("valid registration verification session should be accepted");

    assert!(
        sqlx::query(
            "INSERT INTO public.verification_sessions (                  purpose, channel, destination_ciphertext, destination_nonce,                  encryption_key_version, destination_hmac, hmac_key_version, expires_at              )              VALUES (                  'registration', 'email', '\\x01', '\\x02', 'enc-v1', '\\x03', 'hmac-v1',                  clock_timestamp() + INTERVAL '10 minutes'              )"
        )
        .execute(pool)
        .await
        .is_err(),
        "registration verification must require a user"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.verification_sessions (                  user_id, purpose, channel, destination_ciphertext, destination_nonce,                  encryption_key_version, destination_hmac, hmac_key_version, expires_at              )              VALUES (                  $1::uuid, 'registration', 'whatsapp', $2, $3, 'enc-v1', $4, 'hmac-v1',                  clock_timestamp() + INTERVAL '20 minutes'              )"
        )
        .bind(&pending_user_id)
        .bind(vec![1_u8])
        .bind(vec![2_u8])
        .bind(vec![3_u8])
        .execute(pool)
        .await
        .is_err(),
        "registration verification channel must match the selected initial contact"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.verification_sessions (                  user_id, purpose, channel, destination_ciphertext, destination_nonce,                  encryption_key_version, destination_hmac, hmac_key_version, expires_at              )              VALUES (                  $1::uuid, 'registration', 'email', $2, $3, 'enc-v1', $4, 'hmac-v1',                  clock_timestamp() + INTERVAL '40 minutes'              )"
        )
        .bind(&pending_user_id)
        .bind(vec![1_u8])
        .bind(vec![2_u8])
        .bind(vec![3_u8])
        .execute(pool)
        .await
        .is_err(),
        "registration verification cannot outlive the pending account"
    );

    let lock_session_id: String = sqlx::query_scalar(
        "INSERT INTO public.verification_sessions (              user_id, purpose, channel, destination_ciphertext, destination_nonce,              encryption_key_version, destination_hmac, hmac_key_version, expires_at          ) VALUES ($1::uuid, 'registration', 'email', $2, $3, 'enc-v1', $4, 'hmac-v1',              clock_timestamp() + INTERVAL '20 minutes') RETURNING id::text"
    )
    .bind(&pending_user_id)
    .bind(vec![1_u8])
    .bind(vec![2_u8])
    .bind(vec![3_u8])
    .fetch_one(pool)
    .await
    .expect("second registration session should be accepted");

    for count in 1_i16..=4 {
        sqlx::query(
            "UPDATE public.verification_sessions SET failed_attempt_count = $2 WHERE id = $1::uuid",
        )
        .bind(&lock_session_id)
        .bind(count)
        .execute(pool)
        .await
        .expect("failed attempt counter should increment one at a time");
    }

    sqlx::query(
        "UPDATE public.verification_sessions SET failed_attempt_count = 5 WHERE id = $1::uuid",
    )
    .bind(&lock_session_id)
    .execute(pool)
    .await
    .expect("fifth failed attempt should establish a one-hour lock");

    let remaining_lock_seconds: f64 = sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM (locked_until - clock_timestamp()))::double precision          FROM public.verification_sessions WHERE id = $1::uuid"
    )
    .bind(&lock_session_id)
    .fetch_one(pool)
    .await
    .expect("fifth failed attempt should persist a lock deadline");
    assert!(
        remaining_lock_seconds >= 3590.0,
        "fifth failed attempt should lock the session for one hour"
    );

    assert!(
        sqlx::query(
            "UPDATE public.verification_sessions              SET failed_attempt_count = 0, locked_until = NULL WHERE id = $1::uuid"
        )
        .bind(&lock_session_id)
        .execute(pool)
        .await
        .is_err(),
        "session failure counter cannot reset before its one-hour lock expires"
    );

    assert!(
        sqlx::query(
            "UPDATE public.verification_sessions SET status = 'verified', verified_at = clock_timestamp() WHERE id = $1::uuid"
        )
        .bind(&lock_session_id)
        .execute(pool)
        .await
        .is_err(),
        "a locked session cannot be verified through a direct state update"
    );

    let recovery_session_id: String = sqlx::query_scalar(
        "INSERT INTO public.verification_sessions (              purpose, channel, destination_ciphertext, destination_nonce,              encryption_key_version, destination_hmac, hmac_key_version, expires_at          )          VALUES (              'account_recovery', 'email', '\\x01', '\\x02', 'enc-v1', '\\x03', 'hmac-v1',              clock_timestamp() + INTERVAL '20 minutes'          )          RETURNING id::text",
    )
    .fetch_one(pool)
    .await
    .expect("preselection account recovery session may omit its user");

    assert!(
        sqlx::query(
            "INSERT INTO public.verification_sessions (                  user_id, purpose, channel, destination_ciphertext, destination_nonce,                  encryption_key_version, destination_hmac, hmac_key_version, status, verified_at, expires_at              ) VALUES ($1::uuid, 'registration', 'email', $2, $3, 'enc-v1', $4, 'hmac-v1',                  'verified', clock_timestamp(), clock_timestamp() + INTERVAL '10 minutes')"
        )
        .bind(&pending_user_id)
        .bind(vec![1_u8])
        .bind(vec![2_u8])
        .bind(vec![3_u8])
        .execute(pool)
        .await
        .is_err(),
        "verification session must be inserted in pending state"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.verification_challenges (                  verification_session_id, code_hash, hash_key_version, status, consumed_at, expires_at              ) VALUES ($1::uuid, $2, 'otp-hash-v1', 'consumed', clock_timestamp(),                  clock_timestamp() + INTERVAL '5 minutes')"
        )
        .bind(&session_id)
        .bind(vec![7_u8])
        .execute(pool)
        .await
        .is_err(),
        "verification challenge must be inserted in pending state"
    );

    let challenge_id: String = sqlx::query_scalar(
        "INSERT INTO public.verification_challenges (              verification_session_id, code_hash, hash_key_version, expires_at          )          VALUES ($1::uuid, $2, 'otp-hash-v1', clock_timestamp() + INTERVAL '5 minutes')          RETURNING id::text",
    )
    .bind(&session_id)
    .bind(vec![4_u8])
    .fetch_one(pool)
    .await
    .expect("valid challenge should be accepted");

    assert!(
        sqlx::query(
            "INSERT INTO public.verification_challenges (                  verification_session_id, code_hash, hash_key_version, expires_at              )              VALUES ($1::uuid, $2, 'otp-hash-v1', clock_timestamp() + INTERVAL '5 minutes')"
        )
        .bind(&session_id)
        .bind(vec![5_u8])
        .execute(pool)
        .await
        .is_err(),
        "a session may have only one pending challenge"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.verification_challenges (                  verification_session_id, code_hash, hash_key_version, expires_at              )              VALUES ($1::uuid, $2, 'otp-hash-v1', clock_timestamp() + INTERVAL '11 minutes')"
        )
        .bind(&session_id)
        .bind(vec![6_u8])
        .execute(pool)
        .await
        .is_err(),
        "challenge lifetime cannot exceed ten minutes"
    );

    let locked_challenge_id: String = sqlx::query_scalar(
        "INSERT INTO public.verification_challenges (verification_session_id, code_hash, hash_key_version, expires_at) VALUES ($1::uuid, $2, 'otp-hash-v1', clock_timestamp() + INTERVAL '5 minutes') RETURNING id::text"
    )
    .bind(&lock_session_id)
    .bind(vec![8_u8])
    .fetch_one(pool)
    .await
    .expect("challenge may be issued while a session is locked");

    assert!(
        sqlx::query(
            "UPDATE public.verification_challenges SET status = 'consumed', consumed_at = clock_timestamp() WHERE id = $1::uuid"
        )
        .bind(&locked_challenge_id)
        .execute(pool)
        .await
        .is_err(),
        "a challenge cannot be consumed while its parent session is locked"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.otp_verification_attempt_events (                  verification_session_id, verification_challenge_id, destination_hmac,                  hmac_key_version, channel, purpose, outcome              )              VALUES ($1::uuid, $2::uuid, $3, 'hmac-v1', 'email', 'registration', 'failed')"
        )
        .bind(&session_id)
        .bind(&challenge_id)
        .bind(vec![9_u8])
        .execute(pool)
        .await
        .is_err(),
        "attempt event fingerprint must match its session"
    );

    sqlx::query(
        "INSERT INTO public.otp_verification_attempt_events (              verification_session_id, verification_challenge_id, destination_hmac,              hmac_key_version, channel, purpose, outcome          )          VALUES ($1::uuid, $2::uuid, $3, 'hmac-v1', 'email', 'registration', 'failed')",
    )
    .bind(&session_id)
    .bind(&challenge_id)
    .bind(vec![3_u8])
    .execute(pool)
    .await
    .expect("valid failed attempt event should be recorded");

    sqlx::query(
        "UPDATE public.verification_challenges SET status = 'consumed', consumed_at = clock_timestamp() WHERE id = $1::uuid"
    )
    .bind(&challenge_id)
    .execute(pool)
    .await
    .expect("challenge may be consumed while its parent session is eligible");

    assert!(
        sqlx::query(
            "UPDATE public.otp_verification_attempt_events              SET outcome = 'succeeded' WHERE verification_session_id = $1::uuid"
        )
        .bind(&session_id)
        .execute(pool)
        .await
        .is_err(),
        "verification attempt events cannot be rewritten"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.otp_verification_attempt_events (                  verification_session_id, verification_challenge_id, destination_hmac,                  hmac_key_version, channel, purpose, outcome              )              VALUES ($1::uuid, $2::uuid, $3, 'hmac-v1', 'email', 'account_recovery', 'failed')"
        )
        .bind(&recovery_session_id)
        .bind(&challenge_id)
        .bind(vec![3_u8])
        .execute(pool)
        .await
        .is_err(),
        "attempt event must reference a challenge belonging to the same session"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.otp_delivery_events (                  destination_hmac, hmac_key_version, channel, purpose, dispatch_status              ) VALUES ($1, 'hmac-v1', 'email', 'registration', 'accepted')"
        )
        .bind(vec![3_u8])
        .execute(pool)
        .await
        .is_err(),
        "delivery event must be inserted in pending state"
    );

    let delivery_id: String = sqlx::query_scalar(
        "INSERT INTO public.otp_delivery_events (              destination_hmac, hmac_key_version, channel, purpose          ) VALUES ($1, 'hmac-v1', 'email', 'registration') RETURNING id::text",
    )
    .bind(vec![3_u8])
    .fetch_one(pool)
    .await
    .expect("valid delivery event should be recorded");

    sqlx::query(
        "UPDATE public.otp_delivery_events SET dispatch_status = 'accepted' WHERE id = $1::uuid",
    )
    .bind(&delivery_id)
    .execute(pool)
    .await
    .expect("pending delivery event may transition to accepted");

    assert!(
        sqlx::query(
            "UPDATE public.otp_delivery_events SET dispatch_status = 'failed' WHERE id = $1::uuid"
        )
        .bind(&delivery_id)
        .execute(pool)
        .await
        .is_err(),
        "terminal delivery event status cannot be reopened"
    );

    sqlx::query(
        "UPDATE public.verification_sessions          SET status = 'verified', verified_at = clock_timestamp()          WHERE id = $1::uuid",
    )
    .bind(&recovery_session_id)
    .execute(pool)
    .await
    .expect("pending recovery session may become verified");

    let selected_user_id = insert_active_email(pool, "recovery-target@example.test").await;
    sqlx::query("UPDATE public.verification_sessions SET user_id = $2::uuid WHERE id = $1::uuid")
        .bind(&recovery_session_id)
        .bind(&selected_user_id)
        .execute(pool)
        .await
        .expect("verified recovery session may be assigned to one selected account");

    assert!(
        sqlx::query(
            "UPDATE public.verification_sessions              SET user_id = NULL WHERE id = $1::uuid"
        )
        .bind(&recovery_session_id)
        .execute(pool)
        .await
        .is_err(),
        "selected recovery account binding cannot be cleared"
    );
}

async fn assert_verification_database_operations(pool: &PgPool) {
    let user_id: String =
        sqlx::query_scalar("SELECT public.create_client_user($1, $2, $3, $4, $5)::text")
            .bind("business_owner")
            .bind("email")
            .bind("otp-operations-owner@example.test")
            .bind(Option::<String>::None)
            .bind("encoded-password-hash")
            .fetch_one(pool)
            .await
            .expect("pending account should be created for OTP database operations");

    let session_id: String = sqlx::query_scalar(
        "SELECT public.create_registration_verification_session(\
            $1::uuid, $2::uuid, 'email', $3, $4, 'enc-v1', $5, 'hmac-v1', \
            clock_timestamp() + INTERVAL '10 minutes')::text",
    )
    .bind("11111111-1111-4111-8111-111111111111")
    .bind(&user_id)
    .bind(vec![1_u8])
    .bind(vec![2_u8])
    .bind(vec![3_u8; 32])
    .fetch_one(pool)
    .await
    .expect("controlled function should create a registration verification session");
    assert_eq!(session_id, "11111111-1111-4111-8111-111111111111");

    let session_destination: (String, Vec<u8>, Vec<u8>, String, Vec<u8>, String) = sqlx::query_as(
        "SELECT channel, destination_ciphertext, destination_nonce, encryption_key_version, \
                 destination_hmac, hmac_key_version \
          FROM public.get_registration_verification_session($1::uuid)",
    )
    .bind(&session_id)
    .fetch_one(pool)
    .await
    .expect("runtime should read the pending session through a controlled function");
    assert_eq!(session_destination.0, "email");
    assert_eq!(session_destination.1, vec![1_u8]);
    assert_eq!(session_destination.2, vec![2_u8]);
    assert_eq!(session_destination.3, "enc-v1");
    assert_eq!(session_destination.4, vec![3_u8; 32]);
    assert_eq!(session_destination.5, "hmac-v1");

    let (challenge_id, delivery_event_id): (String, String) = sqlx::query_as(
        "SELECT challenge_id::text, delivery_event_id::text \
         FROM public.reserve_otp_challenge($1::uuid, $2::uuid, $3, 'otp-hmac-v1', $4)",
    )
    .bind(&session_id)
    .bind("22222222-2222-4222-8222-222222222222")
    .bind(vec![4_u8; 32])
    .bind(vec![vec![3_u8; 32]])
    .fetch_one(pool)
    .await
    .expect("controlled function should reserve a challenge and delivery event");
    assert_eq!(challenge_id, "22222222-2222-4222-8222-222222222222");

    let pending_challenge: (String, Vec<u8>, String) = sqlx::query_as(
        "SELECT challenge_id::text, code_hash, hash_key_version \
         FROM public.get_pending_otp_challenge($1::uuid)",
    )
    .bind(&session_id)
    .fetch_one(pool)
    .await
    .expect("runtime should read the pending OTP hash through a controlled function");
    assert_eq!(pending_challenge.0, challenge_id);
    assert_eq!(pending_challenge.1, vec![4_u8; 32]);
    assert_eq!(pending_challenge.2, "otp-hmac-v1");

    sqlx::query("SELECT public.record_otp_delivery_result($1::uuid, 'accepted')")
        .bind(&delivery_event_id)
        .execute(pool)
        .await
        .expect("provider acceptance should be recorded");

    assert!(
        sqlx::query(
            "SELECT * FROM public.reserve_otp_challenge($1::uuid, $2::uuid, $3, 'otp-hmac-v1', $4)",
        )
        .bind(&session_id)
        .bind("33333333-3333-4333-8333-333333333333")
        .bind(vec![5_u8; 32])
        .bind(vec![vec![3_u8; 32]])
        .fetch_one(pool)
        .await
        .is_err(),
        "resend must respect the 60-second cooldown"
    );

    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT public.record_otp_verification_attempt($1::uuid, $2::uuid, NULL, $3)",
        )
        .bind(&session_id)
        .bind(&challenge_id)
        .bind(vec![vec![3_u8; 32]])
        .fetch_one(pool)
        .await
        .is_err(),
        "null OTP comparison result must be rejected"
    );

    let verified: bool = sqlx::query_scalar(
        "SELECT public.record_otp_verification_attempt($1::uuid, $2::uuid, TRUE, $3)",
    )
    .bind(&session_id)
    .bind(&challenge_id)
    .bind(vec![vec![3_u8; 32]])
    .fetch_one(pool)
    .await
    .expect("valid OTP result should be committed atomically");
    assert!(verified);

    let account_status: String =
        sqlx::query_scalar("SELECT account_status FROM public.users WHERE id = $1::uuid")
            .bind(&user_id)
            .fetch_one(pool)
            .await
            .expect("account status should be queryable");
    assert_eq!(account_status, "active");

    let session_status: String =
        sqlx::query_scalar("SELECT status FROM public.verification_sessions WHERE id = $1::uuid")
            .bind(&session_id)
            .fetch_one(pool)
            .await
            .expect("verification session status should be queryable");
    assert_eq!(session_status, "verified");

    let challenge_status: String =
        sqlx::query_scalar("SELECT status FROM public.verification_challenges WHERE id = $1::uuid")
            .bind(&challenge_id)
            .fetch_one(pool)
            .await
            .expect("challenge status should be queryable");
    assert_eq!(challenge_status, "consumed");

    let failed_user_id: String =
        sqlx::query_scalar("SELECT public.create_client_user($1, $2, $3, $4, $5)::text")
            .bind("investor")
            .bind("email")
            .bind("otp-failure-owner@example.test")
            .bind(Option::<String>::None)
            .bind("encoded-password-hash")
            .fetch_one(pool)
            .await
            .expect("second pending account should be created");

    let failed_session_id: String = sqlx::query_scalar(
        "SELECT public.create_registration_verification_session(\
            $1::uuid, $2::uuid, 'email', $3, $4, 'enc-v1', $5, 'hmac-v1', \
            clock_timestamp() + INTERVAL '10 minutes')::text",
    )
    .bind("44444444-4444-4444-8444-444444444444")
    .bind(&failed_user_id)
    .bind(vec![6_u8])
    .bind(vec![7_u8])
    .bind(vec![8_u8; 32])
    .fetch_one(pool)
    .await
    .expect("second verification session should be created");

    let failed_challenge_id: String = sqlx::query_scalar(
        "SELECT challenge_id::text FROM public.reserve_otp_challenge($1::uuid, $2::uuid, $3, 'otp-hmac-v1', $4)",
    )
    .bind(&failed_session_id)
    .bind("55555555-5555-4555-8555-555555555555")
    .bind(vec![9_u8; 32])
    .bind(vec![vec![8_u8; 32]])
    .fetch_one(pool)
    .await
    .expect("second challenge should be reserved");

    let accepted: bool = sqlx::query_scalar(
        "SELECT public.record_otp_verification_attempt($1::uuid, $2::uuid, FALSE, $3)",
    )
    .bind(&failed_session_id)
    .bind(&failed_challenge_id)
    .bind(vec![vec![8_u8; 32]])
    .fetch_one(pool)
    .await
    .expect("failed OTP attempt should be recorded");
    assert!(!accepted);

    let failed_attempt_count: i16 = sqlx::query_scalar(
        "SELECT failed_attempt_count FROM public.verification_sessions WHERE id = $1::uuid",
    )
    .bind(&failed_session_id)
    .fetch_one(pool)
    .await
    .expect("failed attempt counter should be queryable");
    assert_eq!(failed_attempt_count, 1);
}

async fn assert_flowkirim_verification_database_operations(pool: &PgPool) {
    let phone = format!("+6281234{:06}", std::process::id() % 1_000_000);
    let user_id: String =
        sqlx::query_scalar("SELECT public.create_client_user($1, $2, $3, $4, $5)::text")
            .bind("investor")
            .bind("phone")
            .bind(Option::<String>::None)
            .bind(&phone)
            .bind("encoded-password-hash")
            .fetch_one(pool)
            .await
            .expect("pending WhatsApp registration account should be created");

    let session_id = "99999999-9999-4999-8999-999999999999";
    let persisted_session_id: String = sqlx::query_scalar(
        "SELECT public.create_registration_verification_session(\
            $1::uuid, $2::uuid, 'whatsapp', $3, $4, 'enc-v1', $5, 'hmac-v1', \
            clock_timestamp() + INTERVAL '10 minutes')::text",
    )
    .bind(session_id)
    .bind(&user_id)
    .bind(vec![1_u8])
    .bind(vec![2_u8])
    .bind(vec![3_u8; 32])
    .fetch_one(pool)
    .await
    .expect("WhatsApp registration verification session should be created");
    assert_eq!(persisted_session_id, session_id);

    let challenge_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    let reserved: (String, String) = sqlx::query_as(
        "SELECT challenge_id::text, delivery_event_id::text \
         FROM public.reserve_flowkirim_otp_challenge($1::uuid, $2::uuid, $3)",
    )
    .bind(session_id)
    .bind(challenge_id)
    .bind(vec![vec![3_u8; 32]])
    .fetch_one(pool)
    .await
    .expect("FlowKirim challenge should reserve delivery quota");
    assert_eq!(reserved.0, challenge_id);

    let payload: (Option<Vec<u8>>, Option<String>, String) = sqlx::query_as(
        "SELECT code_hash, hash_key_version, provider \
         FROM public.verification_challenges WHERE id = $1::uuid",
    )
    .bind(challenge_id)
    .fetch_one(pool)
    .await
    .expect("FlowKirim challenge metadata should be queryable");
    assert!(
        payload.0.is_none(),
        "provider-managed OTP must not store a fake local hash"
    );
    assert!(
        payload.1.is_none(),
        "provider-managed OTP must not store a local key version"
    );
    assert_eq!(payload.2, "flowkirim");

    assert!(
        sqlx::query(
            "UPDATE public.verification_challenges \
             SET status = 'consumed', consumed_at = clock_timestamp() \
             WHERE id = $1::uuid",
        )
        .bind(challenge_id)
        .execute(pool)
        .await
        .is_err(),
        "FlowKirim challenge cannot be consumed before its request ID is bound"
    );

    assert!(
        sqlx::query("SELECT public.bind_flowkirim_otp_request($1::uuid, $2::uuid, '   ')")
            .bind(session_id)
            .bind(challenge_id)
            .execute(pool)
            .await
            .is_err(),
        "blank FlowKirim request IDs must be rejected"
    );

    sqlx::query("SELECT public.bind_flowkirim_otp_request($1::uuid, $2::uuid, 'fk-request-123')")
        .bind(session_id)
        .bind(challenge_id)
        .execute(pool)
        .await
        .expect("FlowKirim request ID should bind once");

    sqlx::query("SELECT public.bind_flowkirim_otp_request($1::uuid, $2::uuid, 'fk-request-123')")
        .bind(session_id)
        .bind(challenge_id)
        .execute(pool)
        .await
        .expect("repeating the same provider binding should be idempotent");

    assert!(
        sqlx::query(
            "SELECT public.bind_flowkirim_otp_request($1::uuid, $2::uuid, 'fk-request-other')"
        )
        .bind(session_id)
        .bind(challenge_id)
        .execute(pool)
        .await
        .is_err(),
        "a bound FlowKirim request ID must be immutable"
    );

    let pending: (String, String) = sqlx::query_as(
        "SELECT challenge_id::text, provider_request_id \
         FROM public.get_pending_flowkirim_otp_challenge($1::uuid)",
    )
    .bind(session_id)
    .fetch_one(pool)
    .await
    .expect("runtime should retrieve the bound FlowKirim request");
    assert_eq!(pending.0, challenge_id);
    assert_eq!(pending.1, "fk-request-123");

    assert!(
        sqlx::query("SELECT * FROM public.get_pending_otp_challenge($1::uuid)")
            .bind(session_id)
            .fetch_one(pool)
            .await
            .is_err(),
        "local OTP lookup must not return provider-managed challenges"
    );

    let verified: bool = sqlx::query_scalar(
        "SELECT public.record_otp_verification_attempt($1::uuid, $2::uuid, TRUE, $3)",
    )
    .bind(session_id)
    .bind(challenge_id)
    .bind(vec![vec![3_u8; 32]])
    .fetch_one(pool)
    .await
    .expect("successful FlowKirim provider result should activate the pending account atomically");
    assert!(verified);

    let account_status: String =
        sqlx::query_scalar("SELECT account_status FROM public.users WHERE id = $1::uuid")
            .bind(&user_id)
            .fetch_one(pool)
            .await
            .expect("FlowKirim-verified account status should be queryable");
    assert_eq!(account_status, "active");

    let recovery_session_id = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
    let recovery_case_id = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";
    let created_recovery: (String, String) = sqlx::query_as(
        "SELECT session_id::text, case_id::text \
         FROM public.create_account_recovery_verification_session(\
            $1::uuid, $2::uuid, 'password_reset', 'whatsapp', $3, $4, 'enc-v1', $5, 'hmac-v1', \
            clock_timestamp() + INTERVAL '10 minutes')",
    )
    .bind(recovery_session_id)
    .bind(recovery_case_id)
    .bind(vec![4_u8])
    .bind(vec![5_u8])
    .bind(vec![4_u8; 32])
    .fetch_one(pool)
    .await
    .expect("WhatsApp account recovery session should be created");
    assert_eq!(created_recovery.0, recovery_session_id);
    assert_eq!(created_recovery.1, recovery_case_id);

    let recovery_challenge_id = "dddddddd-dddd-4ddd-8ddd-dddddddddddd";
    let recovery_reserved: (String, String) = sqlx::query_as(
        "SELECT challenge_id::text, delivery_event_id::text \
         FROM public.reserve_flowkirim_otp_challenge($1::uuid, $2::uuid, $3)",
    )
    .bind(recovery_session_id)
    .bind(recovery_challenge_id)
    .bind(vec![vec![4_u8; 32]])
    .fetch_one(pool)
    .await
    .expect("FlowKirim challenge should reserve WhatsApp account recovery delivery");
    assert_eq!(recovery_reserved.0, recovery_challenge_id);

    sqlx::query(
        "SELECT public.bind_flowkirim_otp_request($1::uuid, $2::uuid, 'fk-recovery-request-001')",
    )
    .bind(recovery_session_id)
    .bind(recovery_challenge_id)
    .execute(pool)
    .await
    .expect("FlowKirim request ID should bind for account recovery");

    let recovery_pending: (String, String) = sqlx::query_as(
        "SELECT challenge_id::text, provider_request_id \
         FROM public.get_pending_flowkirim_otp_challenge($1::uuid)",
    )
    .bind(recovery_session_id)
    .fetch_one(pool)
    .await
    .expect("runtime should retrieve pending FlowKirim account recovery challenge");
    assert_eq!(recovery_pending.0, recovery_challenge_id);
    assert_eq!(recovery_pending.1, "fk-recovery-request-001");

    let recovery_verified: bool = sqlx::query_scalar(
        "SELECT public.record_otp_verification_attempt($1::uuid, $2::uuid, TRUE, $3)",
    )
    .bind(recovery_session_id)
    .bind(recovery_challenge_id)
    .bind(vec![vec![4_u8; 32]])
    .fetch_one(pool)
    .await
    .expect("valid FlowKirim result should verify account recovery contact");
    assert!(recovery_verified);

    let recovery_status: String =
        sqlx::query_scalar("SELECT status FROM public.verification_sessions WHERE id = $1::uuid")
            .bind(recovery_session_id)
            .fetch_one(pool)
            .await
            .expect("recovery verification status should be queryable");
    assert_eq!(recovery_status, "verified");
}

async fn assert_otp_resend_during_verification_lock(pool: &PgPool) {
    let user_id: String =
        sqlx::query_scalar("SELECT public.create_client_user($1, $2, $3, $4, $5)::text")
            .bind("business_owner")
            .bind("email")
            .bind("otp-lock-resend-owner@example.test")
            .bind(Option::<String>::None)
            .bind("encoded-password-hash")
            .fetch_one(pool)
            .await
            .expect("pending account should be created for lock/resend test");

    let session_id: String = sqlx::query_scalar(
        "SELECT public.create_registration_verification_session(\
            $1::uuid, $2::uuid, 'email', $3, $4, 'enc-v1', $5, 'hmac-v1', \
            clock_timestamp() + INTERVAL '9 minutes')::text",
    )
    .bind("66666666-6666-4666-8666-666666666666")
    .bind(&user_id)
    .bind(vec![1_u8])
    .bind(vec![2_u8])
    .bind(vec![10_u8; 32])
    .fetch_one(pool)
    .await
    .expect("verification session should be created for lock/resend test");

    let initial_challenge_id: String = sqlx::query_scalar(
        "INSERT INTO public.verification_challenges (\
            id, verification_session_id, code_hash, hash_key_version, expires_at\
         ) VALUES (\
            '77777777-7777-4777-8777-777777777777'::uuid, $1::uuid, $2, 'otp-hmac-v1', \
            clock_timestamp() + INTERVAL '8 minutes'\
         ) RETURNING id::text",
    )
    .bind(&session_id)
    .bind(vec![11_u8; 32])
    .fetch_one(pool)
    .await
    .expect("initial pending challenge should be created");

    for _ in 0..5 {
        let accepted: bool = sqlx::query_scalar(
            "SELECT public.record_otp_verification_attempt($1::uuid, $2::uuid, FALSE, $3)",
        )
        .bind(&session_id)
        .bind(&initial_challenge_id)
        .bind(vec![vec![10_u8; 32]])
        .fetch_one(pool)
        .await
        .expect("failed OTP attempt should be recorded until the fifth attempt");
        assert!(!accepted);
    }

    let lock_state: (i16, bool) = sqlx::query_as(
        "SELECT failed_attempt_count, locked_until > clock_timestamp() \
         FROM public.verification_sessions WHERE id = $1::uuid",
    )
    .bind(&session_id)
    .fetch_one(pool)
    .await
    .expect("verification lock state should be queryable");
    assert_eq!(lock_state, (5, true));

    let resent_challenge_id = "88888888-8888-4888-8888-888888888888";
    let reserved: (String, String) = sqlx::query_as(
        "SELECT challenge_id::text, delivery_event_id::text \
         FROM public.reserve_otp_challenge($1::uuid, $2::uuid, $3, 'otp-hmac-v1', $4)",
    )
    .bind(&session_id)
    .bind(resent_challenge_id)
    .bind(vec![12_u8; 32])
    .bind(vec![vec![10_u8; 32]])
    .fetch_one(pool)
    .await
    .expect("OTP resend must remain available while verification is locked");
    assert_eq!(reserved.0, resent_challenge_id);

    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT public.record_otp_verification_attempt($1::uuid, $2::uuid, TRUE, $3)",
        )
        .bind(&session_id)
        .bind(resent_challenge_id)
        .bind(vec![vec![10_u8; 32]])
        .fetch_one(pool)
        .await
        .is_err(),
        "a newly issued OTP must not bypass the active verification lock"
    );

    let final_state: (i16, bool, String) = sqlx::query_as(
        "SELECT s.failed_attempt_count, s.locked_until > clock_timestamp(), c.status \
         FROM public.verification_sessions AS s \
         JOIN public.verification_challenges AS c ON c.verification_session_id = s.id \
         WHERE s.id = $1::uuid AND c.id = $2::uuid",
    )
    .bind(&session_id)
    .bind(resent_challenge_id)
    .fetch_one(pool)
    .await
    .expect("resend challenge and lock state should remain queryable");
    assert_eq!(final_state, (5, true, "pending".to_owned()));
}

async fn assert_authentication_session_invariants(pool: &PgPool) {
    let user_id = insert_active_email(pool, "session-schema-owner@example.test").await;
    let pending_user_id = insert_pending_email(
        pool,
        "investor",
        "session-schema-pending@example.test",
        Some("encoded-password-hash"),
    )
    .await
    .expect("pending account should exist for session rejection test");

    let session_id: String = sqlx::query_scalar(
        "INSERT INTO public.sessions (user_id, token_hash, remember_device, user_agent_summary, ip_prefix_hash, ip_prefix_hmac_key_version, expires_at) \
         VALUES ($1::uuid, $2, TRUE, 'Chrome on Android', $3, 'ip-hmac-v1', clock_timestamp() + INTERVAL '365 days') RETURNING id::text",
    )
    .bind(&user_id)
    .bind(vec![1_u8; 32])
    .bind(vec![9_u8; 32])
    .fetch_one(pool)
    .await
    .expect("active account should be able to create a session");

    let session_timestamps: (i64, bool, Option<String>) = sqlx::query_as(
        "SELECT EXTRACT(EPOCH FROM (expires_at - created_at))::bigint, remember_device, user_agent_summary \
         FROM public.sessions WHERE id = $1::uuid",
    )
    .bind(&session_id)
    .fetch_one(pool)
    .await
    .expect("session timestamps should be queryable");
    assert_eq!(session_timestamps.0, 30 * 24 * 60 * 60);
    assert!(session_timestamps.1);
    assert_eq!(session_timestamps.2.as_deref(), Some("Chrome on Android"));

    assert!(
        sqlx::query("INSERT INTO public.sessions (user_id, token_hash) VALUES ($1::uuid, $2)",)
            .bind(&pending_user_id)
            .bind(vec![2_u8; 32])
            .execute(pool)
            .await
            .is_err(),
        "pending accounts cannot create authentication sessions"
    );

    assert!(
        sqlx::query("INSERT INTO public.sessions (user_id, token_hash) VALUES ($1::uuid, $2)",)
            .bind(&user_id)
            .bind(vec![1_u8; 32])
            .execute(pool)
            .await
            .is_err(),
        "session token hashes must be globally unique"
    );

    assert!(
        sqlx::query("INSERT INTO public.sessions (user_id, token_hash) VALUES ($1::uuid, $2)",)
            .bind(&user_id)
            .bind(vec![3_u8; 31])
            .execute(pool)
            .await
            .is_err(),
        "session token hash must be a 32-byte SHA-256 digest"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.sessions (user_id, token_hash, ip_prefix_hmac_key_version) VALUES ($1::uuid, $2, 'ip-hmac-v1')",
        )
        .bind(&user_id)
        .bind(vec![8_u8; 32])
        .execute(pool)
        .await
        .is_err(),
        "network fingerprint key version cannot exist without its HMAC"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.sessions (user_id, token_hash, ip_prefix_hash) VALUES ($1::uuid, $2, $3)",
        )
        .bind(&user_id)
        .bind(vec![10_u8; 32])
        .bind(vec![11_u8; 32])
        .execute(pool)
        .await
        .is_err(),
        "network fingerprint HMAC cannot exist without its key version"
    );

    let rotated_at_before: f64 = sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM token_rotated_at)::double precision FROM public.sessions WHERE id = $1::uuid",
    )
    .bind(&session_id)
    .fetch_one(pool)
    .await
    .expect("token rotation timestamp should exist");

    sqlx::query("UPDATE public.sessions SET token_hash = $2 WHERE id = $1::uuid")
        .bind(&session_id)
        .bind(vec![4_u8; 32])
        .execute(pool)
        .await
        .expect("active session token may be rotated");

    let rotated_at_after: f64 = sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM token_rotated_at)::double precision FROM public.sessions WHERE id = $1::uuid",
    )
    .bind(&session_id)
    .fetch_one(pool)
    .await
    .expect("rotated timestamp should be queryable");
    assert!(rotated_at_after >= rotated_at_before);

    assert!(
        sqlx::query("UPDATE public.sessions SET remember_device = FALSE WHERE id = $1::uuid")
            .bind(&session_id)
            .execute(pool)
            .await
            .is_err(),
        "remember-device choice is immutable for a session"
    );

    assert!(
        sqlx::query("UPDATE public.sessions SET expires_at = clock_timestamp() + INTERVAL '365 days' WHERE id = $1::uuid")
            .bind(&session_id)
            .execute(pool)
            .await
            .is_err(),
        "absolute session expiry is immutable"
    );

    sqlx::query(
        "UPDATE public.sessions SET revoked_at = clock_timestamp(), revocation_reason = 'user_revoked' \
         WHERE id = $1::uuid",
    )
    .bind(&session_id)
    .execute(pool)
    .await
    .expect("user may revoke an individual session");

    let revoked_reason: Option<String> =
        sqlx::query_scalar("SELECT revocation_reason FROM public.sessions WHERE id = $1::uuid")
            .bind(&session_id)
            .fetch_one(pool)
            .await
            .expect("revocation reason should be recorded");
    assert_eq!(revoked_reason.as_deref(), Some("user_revoked"));

    assert!(
        sqlx::query(
            "UPDATE public.sessions SET revoked_at = NULL, revocation_reason = NULL WHERE id = $1::uuid",
        )
        .bind(&session_id)
        .execute(pool)
        .await
        .is_err(),
        "revoked sessions cannot be reactivated"
    );

    assert!(
        sqlx::query("UPDATE public.sessions SET token_hash = $2 WHERE id = $1::uuid",)
            .bind(&session_id)
            .bind(vec![5_u8; 32])
            .execute(pool)
            .await
            .is_err(),
        "revoked sessions cannot rotate tokens"
    );

    let second_session_id: String = sqlx::query_scalar(
        "INSERT INTO public.sessions (user_id, token_hash, remember_device) \
         VALUES ($1::uuid, $2, FALSE) RETURNING id::text",
    )
    .bind(&user_id)
    .bind(vec![6_u8; 32])
    .fetch_one(pool)
    .await
    .expect("a user may have multiple concurrent sessions");

    sqlx::query("UPDATE public.users SET account_status = 'deactivated' WHERE id = $1::uuid")
        .bind(&user_id)
        .execute(pool)
        .await
        .expect("account deactivation should succeed");

    let second_session_revocation: (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT revoked_at::text, revocation_reason FROM public.sessions WHERE id = $1::uuid",
    )
    .bind(&second_session_id)
    .fetch_one(pool)
    .await
    .expect("session should remain available as a revoked record");
    assert!(second_session_revocation.0.is_some());
    assert_eq!(
        second_session_revocation.1.as_deref(),
        Some("account_status_changed")
    );

    assert!(
        sqlx::query("INSERT INTO public.sessions (user_id, token_hash) VALUES ($1::uuid, $2)")
            .bind(&user_id)
            .bind(vec![7_u8; 32])
            .execute(pool)
            .await
            .is_err(),
        "deactivated accounts cannot create new sessions"
    );
}

async fn assert_authentication_database_operations(pool: &PgPool) {
    let user_id = insert_active_email(pool, "password-login-database@example.test").await;

    let credential: (String, String) = sqlx::query_as(
        "SELECT user_id::text, password_hash \
         FROM public.find_active_password_login('password-login-database@example.test', 'business_owner')",
    )
    .fetch_one(pool)
    .await
    .expect("active owner credentials should be returned through the guarded function");
    assert_eq!(credential.0, user_id);

    let wrong_role: Option<String> = sqlx::query_scalar(
        "SELECT user_id::text \
         FROM public.find_active_password_login('password-login-database@example.test', 'investor')",
    )
    .fetch_optional(pool)
    .await
    .expect("role-scoped credential lookup should execute");
    assert!(
        wrong_role.is_none(),
        "owner credentials must not authenticate as investor"
    );

    let token_hash = vec![41_u8; 32];
    let session_id: String =
        sqlx::query_scalar("SELECT public.create_password_login_session($1::uuid, $2)::text")
            .bind(&user_id)
            .bind(&token_hash)
            .fetch_one(pool)
            .await
            .expect("active password account should create a session through the guarded function");
    let session_owner: String =
        sqlx::query_scalar("SELECT user_id::text FROM public.sessions WHERE id = $1::uuid")
            .bind(&session_id)
            .fetch_one(pool)
            .await
            .expect("created session should be persisted");
    assert_eq!(session_owner, user_id);

    let active_session: (String, String, String, i64) = sqlx::query_as(
        "SELECT session_id::text, user_id::text, primary_role, expires_in_seconds \
         FROM public.get_active_password_login_session($1)",
    )
    .bind(&token_hash)
    .fetch_one(pool)
    .await
    .expect("active session should validate and update its idle timestamp");
    assert_eq!(active_session.0, session_id);
    assert_eq!(active_session.1, user_id);
    assert_eq!(active_session.2, "business_owner");
    assert!((29 * 24 * 60 * 60..=30 * 24 * 60 * 60).contains(&active_session.3));

    let rotated_token_hash = vec![42_u8; 32];
    let rotated_session: (String, String, String, i64) = sqlx::query_as(
        "SELECT session_id::text, user_id::text, primary_role, expires_in_seconds \
         FROM public.rotate_active_password_login_session($1, $2)",
    )
    .bind(&token_hash)
    .bind(&rotated_token_hash)
    .fetch_one(pool)
    .await
    .expect("active session token should rotate atomically");
    assert_eq!(rotated_session.0, session_id);
    assert_eq!(rotated_session.1, user_id);
    assert_eq!(rotated_session.2, "business_owner");

    let old_token_session: Option<String> = sqlx::query_scalar(
        "SELECT session_id::text FROM public.get_active_password_login_session($1)",
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .expect("old token lookup should execute");
    assert!(
        old_token_session.is_none(),
        "the old token must stop authenticating immediately after rotation"
    );

    let new_token_session: String = sqlx::query_scalar(
        "SELECT session_id::text FROM public.get_active_password_login_session($1)",
    )
    .bind(&rotated_token_hash)
    .fetch_one(pool)
    .await
    .expect("new token should validate after rotation");
    assert_eq!(new_token_session, session_id);

    let identifier_hash = vec![51_u8; 32];
    let ip_hash = vec![52_u8; 32];
    for attempt in 1..=10 {
        let allowed: bool =
            sqlx::query_scalar("SELECT public.reserve_password_login_attempt($1, $2)")
                .bind(&identifier_hash)
                .bind(&ip_hash)
                .fetch_one(pool)
                .await
                .expect("identifier and IP login rate limit should accept the initial window");
        assert!(
            allowed,
            "attempt {attempt} should be inside the identifier threshold"
        );
    }
    let blocked: bool = sqlx::query_scalar("SELECT public.reserve_password_login_attempt($1, $2)")
        .bind(&identifier_hash)
        .bind(&ip_hash)
        .fetch_one(pool)
        .await
        .expect("rate limit function should return a decision after the threshold");
    assert!(!blocked, "the eleventh identifier attempt must be limited");

    sqlx::query(
        "UPDATE public.password_login_rate_limits SET window_started_at = clock_timestamp() - INTERVAL '20 minutes' WHERE scope = 'identifier' AND key_hash = $1",
    )
    .bind(&identifier_hash)
    .execute(pool)
    .await
    .expect("test should age the rate-limit window without expiring its active block");
    let still_blocked: bool =
        sqlx::query_scalar("SELECT public.reserve_password_login_attempt($1, $2)")
            .bind(&identifier_hash)
            .bind(&ip_hash)
            .fetch_one(pool)
            .await
            .expect("active login block should remain enforceable after the original window");
    assert!(
        !still_blocked,
        "an active block must not reset with its original window"
    );

    sqlx::query(
        "UPDATE public.password_login_rate_limits SET blocked_until = clock_timestamp() - INTERVAL '1 second' WHERE scope = 'identifier' AND key_hash = $1",
    )
    .bind(&identifier_hash)
    .execute(pool)
    .await
    .expect("test should expire the login block");
    let allowed_after_block: bool =
        sqlx::query_scalar("SELECT public.reserve_password_login_attempt($1, $2)")
            .bind(&identifier_hash)
            .bind(&ip_hash)
            .fetch_one(pool)
            .await
            .expect("identifier should be reset after its block expires");
    assert!(allowed_after_block);

    sqlx::query("SELECT public.clear_password_login_identifier_limit($1)")
        .bind(&identifier_hash)
        .execute(pool)
        .await
        .expect("successful login should be able to clear the identifier bucket");
    let allowed_after_clear: bool =
        sqlx::query_scalar("SELECT public.reserve_password_login_attempt($1, $2)")
            .bind(&identifier_hash)
            .bind(&ip_hash)
            .fetch_one(pool)
            .await
            .expect("cleared identifier should be eligible again");
    assert!(allowed_after_clear);

    let ip_only_hash = vec![61_u8; 32];
    for attempt in 1..=30 {
        let identifier_only_hash = vec![attempt as u8; 32];
        let allowed: bool =
            sqlx::query_scalar("SELECT public.reserve_password_login_attempt($1, $2)")
                .bind(&identifier_only_hash)
                .bind(&ip_only_hash)
                .fetch_one(pool)
                .await
                .expect("IP bucket should accept requests through its threshold");
        assert!(
            allowed,
            "IP attempt {attempt} should be inside the threshold"
        );
    }
    let ip_blocked: bool =
        sqlx::query_scalar("SELECT public.reserve_password_login_attempt($1, $2)")
            .bind(vec![99_u8; 32])
            .bind(&ip_only_hash)
            .fetch_one(pool)
            .await
            .expect("IP limit should return a decision after its threshold");
    assert!(!ip_blocked, "the thirty-first IP attempt must be limited");

    let revoked: bool = sqlx::query_scalar("SELECT public.revoke_password_login_session($1)")
        .bind(&rotated_token_hash)
        .fetch_one(pool)
        .await
        .expect("logout function should revoke the session");
    assert!(revoked);

    let revoked_again: bool = sqlx::query_scalar("SELECT public.revoke_password_login_session($1)")
        .bind(&rotated_token_hash)
        .fetch_one(pool)
        .await
        .expect("repeated logout should be idempotent");
    assert!(!revoked_again);

    let reason: String =
        sqlx::query_scalar("SELECT revocation_reason FROM public.sessions WHERE id = $1::uuid")
            .bind(&session_id)
            .fetch_one(pool)
            .await
            .expect("revocation reason should remain auditable");
    assert_eq!(reason, "logout");

    let second_token_hash = vec![43_u8; 32];
    let second_session_id: String =
        sqlx::query_scalar("SELECT public.create_password_login_session($1::uuid, $2)::text")
            .bind(&user_id)
            .bind(&second_token_hash)
            .fetch_one(pool)
            .await
            .expect("active user should be able to create another device session");
    let third_token_hash = vec![44_u8; 32];
    let third_session_id: String =
        sqlx::query_scalar("SELECT public.create_password_login_session($1::uuid, $2)::text")
            .bind(&user_id)
            .bind(&third_token_hash)
            .fetch_one(pool)
            .await
            .expect("active user should be able to create a third device session");

    let invalid_revoke_count: i32 =
        sqlx::query_scalar("SELECT public.revoke_all_password_login_sessions($1)")
            .bind(vec![99_u8; 32])
            .fetch_one(pool)
            .await
            .expect("invalid session token should return a safe no-op result");
    assert_eq!(invalid_revoke_count, 0);

    let first_revoke =
        sqlx::query_scalar::<_, i32>("SELECT public.revoke_all_password_login_sessions($1)")
            .bind(&second_token_hash)
            .fetch_one(pool);
    let second_revoke =
        sqlx::query_scalar::<_, i32>("SELECT public.revoke_all_password_login_sessions($1)")
            .bind(&third_token_hash)
            .fetch_one(pool);
    let (first_revoke, second_revoke) = tokio::join!(first_revoke, second_revoke);
    let revoked_counts = [
        first_revoke.expect("first concurrent logout-all should complete"),
        second_revoke.expect("second concurrent logout-all should complete"),
    ];
    assert_eq!(revoked_counts.iter().sum::<i32>(), 2);
    assert!(
        revoked_counts.contains(&0) && revoked_counts.contains(&2),
        "concurrent logout-all should revoke the account sessions exactly once"
    );

    for session_id in [&second_session_id, &third_session_id] {
        let reason: String =
            sqlx::query_scalar("SELECT revocation_reason FROM public.sessions WHERE id = $1::uuid")
                .bind(session_id)
                .fetch_one(pool)
                .await
                .expect("logout-all revocation reason should be recorded");
        assert_eq!(reason, "logout_all");
    }

    for token_hash in [&second_token_hash, &third_token_hash] {
        let revoked_session: Option<String> = sqlx::query_scalar(
            "SELECT session_id::text FROM public.get_active_password_login_session($1)",
        )
        .bind(token_hash)
        .fetch_optional(pool)
        .await
        .expect("revoked session validation should execute");
        assert!(revoked_session.is_none());
    }
}

async fn assert_account_recovery_schema_invariants(pool: &PgPool) {
    let first_user_id = insert_active_email(pool, "recovery-contact-first@example.test").await;
    let second_user_id = insert_active_email(pool, "recovery-contact-second@example.test").await;
    let reviewer_email = format!(
        "recovery-review-admin-{}-{}@example.test",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos()
    );
    let reviewer_user_id: String = sqlx::query_scalar(
        "INSERT INTO public.users (primary_role, account_status, email_normalized, initial_contact_channel, email_verified_at, initial_contact_verified_at, password_hash) \
         VALUES ('admin', 'active', $1, 'email', clock_timestamp(), clock_timestamp(), '$argon2id$test-review-hash') \
         RETURNING id::text",
    )
    .bind(&reviewer_email)
    .fetch_one(pool)
    .await
    .expect("active Admin reviewer should be created");

    let unrelated_session_hash = vec![62_u8; 32];
    sqlx::query(
        "INSERT INTO public.sessions (user_id, token_hash, expires_at) \
         VALUES ($1::uuid, $2, clock_timestamp() + INTERVAL '30 days')",
    )
    .bind(&second_user_id)
    .bind(&unrelated_session_hash)
    .execute(pool)
    .await
    .expect("a session for a different account should be created");

    let reviewer_session_hash = vec![61_u8; 32];
    let reviewer_session_id: String = sqlx::query_scalar(
        "INSERT INTO public.admin_sessions (user_id, token_hash) \
         VALUES ($1::uuid, $2) RETURNING id::text",
    )
    .bind(&reviewer_user_id)
    .bind(&reviewer_session_hash)
    .fetch_one(pool)
    .await
    .expect("active dedicated Admin session should be created for recovery review");

    let revoked_reviewer_session_hash = vec![63_u8; 32];
    sqlx::query(
        "INSERT INTO public.admin_sessions \
         (user_id, token_hash, revoked_at, revocation_reason) \
         VALUES ($1::uuid, $2, clock_timestamp(), 'logout')",
    )
    .bind(&reviewer_user_id)
    .bind(&revoked_reviewer_session_hash)
    .execute(pool)
    .await
    .expect("revoked Admin session fixture should be created");

    let expired_reviewer_session_hash = vec![64_u8; 32];
    sqlx::query(
        "INSERT INTO public.admin_sessions \
         (user_id, token_hash, created_at, last_seen_at, expires_at) \
         VALUES ($1::uuid, $2, clock_timestamp() - INTERVAL '2 days', \
                 clock_timestamp() - INTERVAL '2 days', clock_timestamp() - INTERVAL '1 day')",
    )
    .bind(&reviewer_user_id)
    .bind(&expired_reviewer_session_hash)
    .execute(pool)
    .await
    .expect("expired Admin session fixture should be created");

    let first_contact_session: String = sqlx::query_scalar(
        "INSERT INTO public.verification_sessions (user_id, purpose, channel, destination_ciphertext, destination_nonce, encryption_key_version, destination_hmac, hmac_key_version, expires_at)  VALUES ($1::uuid, 'recovery_contact_add', 'email', $2, $3, 'enc-v1', $4, 'hmac-v1', clock_timestamp() + INTERVAL '20 minutes') RETURNING id::text",
    )
    .bind(&first_user_id)
    .bind(vec![1_u8])
    .bind(vec![2_u8])
    .bind(vec![31_u8; 32])
    .fetch_one(pool)
    .await
    .expect("account-bound recovery contact verification session should be created");

    assert!(
        sqlx::query(
            "INSERT INTO public.recovery_contacts (user_id, channel, contact_ciphertext, contact_nonce, encryption_key_version, contact_hmac, hmac_key_version, verification_session_id)  VALUES ($1::uuid, 'email', $2, $3, 'enc-v1', $4, 'hmac-v1', $5::uuid)"
        )
        .bind(&first_user_id)
        .bind(vec![3_u8])
        .bind(vec![4_u8])
        .bind(vec![31_u8; 32])
        .bind(&first_contact_session)
        .execute(pool)
        .await
        .is_err(),
        "unverified recovery contact must not be stored"
    );

    sqlx::query(
        "UPDATE public.verification_sessions SET status = 'verified', verified_at = clock_timestamp() WHERE id = $1::uuid",
    )
    .bind(&first_contact_session)
    .execute(pool)
    .await
    .expect("recovery contact verification session should be verified");

    let first_contact_id: String = sqlx::query_scalar(
        "INSERT INTO public.recovery_contacts (user_id, channel, contact_ciphertext, contact_nonce, encryption_key_version, contact_hmac, hmac_key_version, verification_session_id)  VALUES ($1::uuid, 'email', $2, $3, 'enc-v1', $4, 'hmac-v1', $5::uuid) RETURNING id::text",
    )
    .bind(&first_user_id)
    .bind(vec![3_u8])
    .bind(vec![4_u8])
    .bind(vec![31_u8; 32])
    .bind(&first_contact_session)
    .fetch_one(pool)
    .await
    .expect("verified recovery contact should be accepted");

    let verified_at_exists: bool = sqlx::query_scalar(
        "SELECT verified_at IS NOT NULL FROM public.recovery_contacts WHERE id = $1::uuid",
    )
    .bind(&first_contact_id)
    .fetch_one(pool)
    .await
    .expect("recovery contact verification timestamp should exist");
    assert!(verified_at_exists);

    assert!(
        sqlx::query(
            "UPDATE public.recovery_contacts SET contact_ciphertext = $2 WHERE id = $1::uuid"
        )
        .bind(&first_contact_id)
        .bind(vec![9_u8])
        .execute(pool)
        .await
        .is_err(),
        "verified recovery contact payload must be immutable"
    );

    let second_contact_session: String = sqlx::query_scalar(
        "INSERT INTO public.verification_sessions (user_id, purpose, channel, destination_ciphertext, destination_nonce, encryption_key_version, destination_hmac, hmac_key_version, expires_at)  VALUES ($1::uuid, 'recovery_contact_add', 'email', $2, $3, 'enc-v1', $4, 'hmac-v1', clock_timestamp() + INTERVAL '20 minutes') RETURNING id::text",
    )
    .bind(&second_user_id)
    .bind(vec![5_u8])
    .bind(vec![6_u8])
    .bind(vec![31_u8; 32])
    .fetch_one(pool)
    .await
    .expect("second account may verify the same shared contact");

    sqlx::query(
        "UPDATE public.verification_sessions SET status = 'verified', verified_at = clock_timestamp() WHERE id = $1::uuid",
    )
    .bind(&second_contact_session)
    .execute(pool)
    .await
    .expect("second account recovery contact session should be verified");

    sqlx::query(
        "INSERT INTO public.recovery_contacts (user_id, channel, contact_ciphertext, contact_nonce, encryption_key_version, contact_hmac, hmac_key_version, verification_session_id)  VALUES ($1::uuid, 'email', $2, $3, 'enc-v1', $4, 'hmac-v1', $5::uuid)",
    )
    .bind(&second_user_id)
    .bind(vec![7_u8])
    .bind(vec![8_u8])
    .bind(vec![31_u8; 32])
    .bind(&second_contact_session)
    .execute(pool)
    .await
    .expect("same optional recovery contact may belong to another account");

    let duplicate_contact_session: String = sqlx::query_scalar(
        "INSERT INTO public.verification_sessions (user_id, purpose, channel, destination_ciphertext, destination_nonce, encryption_key_version, destination_hmac, hmac_key_version, expires_at)  VALUES ($1::uuid, 'recovery_contact_add', 'email', $2, $3, 'enc-v1', $4, 'hmac-v1', clock_timestamp() + INTERVAL '20 minutes') RETURNING id::text",
    )
    .bind(&first_user_id)
    .bind(vec![13_u8])
    .bind(vec![14_u8])
    .bind(vec![31_u8; 32])
    .fetch_one(pool)
    .await
    .expect("duplicate contact must be independently re-verified before deduplication");

    sqlx::query(
        "UPDATE public.verification_sessions SET status = 'verified', verified_at = clock_timestamp() WHERE id = $1::uuid",
    )
    .bind(&duplicate_contact_session)
    .execute(pool)
    .await
    .expect("duplicate contact verification session should be verified");

    assert!(
        sqlx::query(
            "INSERT INTO public.recovery_contacts (user_id, channel, contact_ciphertext, contact_nonce, encryption_key_version, contact_hmac, hmac_key_version, verification_session_id)  VALUES ($1::uuid, 'email', $2, $3, 'enc-v1', $4, 'hmac-v1', $5::uuid)"
        )
        .bind(&first_user_id)
        .bind(vec![9_u8])
        .bind(vec![10_u8])
        .bind(vec![31_u8; 32])
        .bind(&duplicate_contact_session)
        .execute(pool)
        .await
        .is_err(),
        "one account cannot register the same active recovery contact twice"
    );

    let recovery_session_id: String = sqlx::query_scalar(
        "INSERT INTO public.verification_sessions (purpose, channel, destination_ciphertext, destination_nonce, encryption_key_version, destination_hmac, hmac_key_version, expires_at)  VALUES ('account_recovery', 'email', $1, $2, 'enc-v1', $3, 'hmac-v1', clock_timestamp() + INTERVAL '20 minutes') RETURNING id::text",
    )
    .bind(vec![11_u8])
    .bind(vec![12_u8])
    .bind(vec![31_u8; 32])
    .fetch_one(pool)
    .await
    .expect("preselection account recovery session should be created");

    let recovery_case_id: String = sqlx::query_scalar(
        "INSERT INTO public.account_recovery_cases (verification_session_id, requested_action)  VALUES ($1::uuid, 'password_reset') RETURNING id::text",
    )
    .bind(&recovery_session_id)
    .fetch_one(pool)
    .await
    .expect("recovery case should start without a selected account");

    assert!(
        sqlx::query(
            "INSERT INTO public.account_recovery_cases (verification_session_id, requested_action) VALUES ($1::uuid, 'forgot_email')"
        )
        .bind(&recovery_session_id)
        .execute(pool)
        .await
        .is_err(),
        "one verification session may have only one recovery case"
    );

    assert!(
        sqlx::query(
            "UPDATE public.account_recovery_cases SET status = 'account_selection_required' WHERE id = $1::uuid"
        )
        .bind(&recovery_case_id)
        .execute(pool)
        .await
        .is_err(),
        "recovery case cannot advance before contact verification"
    );

    sqlx::query(
        "UPDATE public.verification_sessions SET status = 'verified', verified_at = clock_timestamp() WHERE id = $1::uuid",
    )
    .bind(&recovery_session_id)
    .execute(pool)
    .await
    .expect("recovery contact verification should complete");

    sqlx::query("UPDATE public.verification_sessions SET user_id = $2::uuid WHERE id = $1::uuid")
        .bind(&recovery_session_id)
        .bind(&first_user_id)
        .execute(pool)
        .await
        .expect("verified recovery session may bind one selected account");

    sqlx::query(
        "UPDATE public.account_recovery_cases SET target_user_id = $2::uuid, status = 'under_review' WHERE id = $1::uuid",
    )
    .bind(&recovery_case_id)
    .bind(&first_user_id)
    .execute(pool)
    .await
    .expect("case may enter review after the verified account is selected");

    assert!(
        sqlx::query(
            "UPDATE public.account_recovery_cases SET target_user_id = $2::uuid WHERE id = $1::uuid"
        )
        .bind(&recovery_case_id)
        .bind(&second_user_id)
        .execute(pool)
        .await
        .is_err(),
        "selected recovery account cannot be changed"
    );

    let unrelated_session_match: bool =
        sqlx::query_scalar("SELECT public.record_account_recovery_session_signal($1::uuid, $2)")
            .bind(&recovery_case_id)
            .bind(&unrelated_session_hash)
            .fetch_one(pool)
            .await
            .expect("session signal check should safely reject a different account session");
    assert!(!unrelated_session_match);

    let review_context: (String, Option<String>, String, Option<String>, bool) = sqlx::query_as(
        "SELECT resource_id::text, target_user_id::text, case_status, \
                    target_account_status, admin_scope_granted \
               FROM public.get_account_recovery_review_context($1::uuid, $2::uuid, $3)",
    )
        .bind(&recovery_case_id)
        .bind(&reviewer_user_id)
        .bind(&reviewer_session_hash)
        .fetch_one(pool)
        .await
        .expect("active Admin session should load minimal recovery review context");
    assert_eq!(review_context.0, recovery_case_id);
    assert_eq!(review_context.1.as_deref(), Some(first_user_id.as_str()));
    assert_eq!(review_context.2, "under_review");
    assert_eq!(review_context.3.as_deref(), Some("active"));
    assert!(review_context.4);

    let revoked_context = sqlx::query(
        "SELECT resource_id FROM public.get_account_recovery_review_context($1::uuid, $2::uuid, $3)",
    )
    .bind(&recovery_case_id)
    .bind(&reviewer_user_id)
    .bind(&revoked_reviewer_session_hash)
    .fetch_optional(pool)
    .await
    .expect("revoked Admin session must not load recovery review context");
    assert!(revoked_context.is_none());

    let mismatched_actor_context = sqlx::query(
        "SELECT resource_id FROM public.get_account_recovery_review_context($1::uuid, $2::uuid, $3)",
    )
    .bind(&recovery_case_id)
    .bind(&first_user_id)
    .bind(&reviewer_session_hash)
    .fetch_optional(pool)
    .await
    .expect("an Admin session hash must not be reusable for another actor");
    assert!(mismatched_actor_context.is_none());

    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT public.review_account_recovery_case($1::uuid, $2::uuid, $3, 'approved', 'ownership_confirmed')",
        )
        .bind(&recovery_case_id)
        .bind(&reviewer_user_id)
        .bind(&revoked_reviewer_session_hash)
        .fetch_one(pool)
        .await
        .is_err(),
        "a revoked Admin session must not approve a recovery case"
    );

    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT public.review_account_recovery_case($1::uuid, $2::uuid, $3, 'approved', 'ownership_confirmed')",
        )
        .bind(&recovery_case_id)
        .bind(&reviewer_user_id)
        .bind(&expired_reviewer_session_hash)
        .fetch_one(pool)
        .await
        .is_err(),
        "an expired Admin session must not approve a recovery case"
    );

    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT public.review_account_recovery_case($1::uuid, $2::uuid, $3, 'approved', 'ownership_confirmed')",
        )
        .bind(&recovery_case_id)
        .bind(&first_user_id)
        .bind(&reviewer_session_hash)
        .fetch_one(pool)
        .await
        .is_err(),
        "a non-Admin account must not approve a recovery case"
    );

    let approved: bool = sqlx::query_scalar(
        "SELECT public.review_account_recovery_case($1::uuid, $2::uuid, $3, 'approved', 'ownership_confirmed')",
    )
    .bind(&recovery_case_id)
    .bind(&reviewer_user_id)
    .bind(&reviewer_session_hash)
    .fetch_one(pool)
    .await
    .expect("active Admin may approve an eligible recovery case");
    assert!(approved);

    let completed_context: (bool,) = sqlx::query_as(
        "SELECT admin_scope_granted \
           FROM public.get_account_recovery_review_context($1::uuid, $2::uuid, $3)",
    )
    .bind(&recovery_case_id)
    .bind(&reviewer_user_id)
    .bind(&reviewer_session_hash)
    .fetch_one(pool)
    .await
    .expect("completed review context should remain readable for authorization");
    assert!(!completed_context.0);

    let review_row: (String, String, String) = sqlx::query_as(
        "SELECT decision, reason_code, reviewer_session_id::text \
           FROM public.account_recovery_review_decisions WHERE recovery_case_id = $1::uuid",
    )
    .bind(&recovery_case_id)
    .fetch_one(pool)
    .await
    .expect("review decision should be persisted");
    assert_eq!(review_row.0, "approved");
    assert_eq!(review_row.1, "ownership_confirmed");
    assert_eq!(review_row.2, reviewer_session_id);

    assert!(
        sqlx::query(
            "UPDATE public.account_recovery_review_decisions SET reason_code = 'policy_violation' WHERE recovery_case_id = $1::uuid",
        )
        .bind(&recovery_case_id)
        .execute(pool)
        .await
        .is_err(),
        "review decisions must be append-only"
    );

    let token_id: String = sqlx::query_scalar(
        "INSERT INTO public.password_reset_tokens (recovery_case_id, user_id, token_hash, expires_at)  VALUES ($1::uuid, $2::uuid, $3, clock_timestamp() + INTERVAL '15 minutes') RETURNING id::text",
    )
    .bind(&recovery_case_id)
    .bind(&first_user_id)
    .bind(vec![41_u8; 32])
    .fetch_one(pool)
    .await
    .expect("approved case may issue an opaque password reset token hash");

    assert!(
        sqlx::query(
            "INSERT INTO public.password_reset_tokens (recovery_case_id, user_id, token_hash, expires_at)  VALUES ($1::uuid, $2::uuid, $3, clock_timestamp() + INTERVAL '15 minutes')"
        )
        .bind(&recovery_case_id)
        .bind(&first_user_id)
        .bind(vec![42_u8; 32])
        .execute(pool)
        .await
        .is_err(),
        "one recovery case cannot have multiple live reset tokens"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.password_reset_tokens (recovery_case_id, user_id, token_hash, expires_at)  VALUES ($1::uuid, $2::uuid, $3, clock_timestamp() + INTERVAL '15 minutes')"
        )
        .bind(&recovery_case_id)
        .bind(&second_user_id)
        .bind(vec![43_u8; 32])
        .execute(pool)
        .await
        .is_err(),
        "reset token must be bound to the selected account"
    );

    sqlx::query(
        "UPDATE public.password_reset_tokens SET consumed_at = clock_timestamp() WHERE id = $1::uuid",
    )
    .bind(&token_id)
    .execute(pool)
    .await
    .expect("unexpired reset token may be consumed once");

    assert!(
        sqlx::query(
            "UPDATE public.password_reset_tokens SET consumed_at = clock_timestamp() WHERE id = $1::uuid"
        )
        .bind(&token_id)
        .execute(pool)
        .await
        .is_err(),
        "consumed reset token cannot be reused"
    );

    let second_token_id: String = sqlx::query_scalar(
        "INSERT INTO public.password_reset_tokens (recovery_case_id, user_id, token_hash, expires_at)  VALUES ($1::uuid, $2::uuid, $3, clock_timestamp() + INTERVAL '15 minutes') RETURNING id::text",
    )
    .bind(&recovery_case_id)
    .bind(&first_user_id)
    .bind(vec![44_u8; 32])
    .fetch_one(pool)
    .await
    .expect("a new token may be issued after the previous token is consumed");

    sqlx::query("UPDATE public.users SET account_status = 'deactivated' WHERE id = $1::uuid")
        .bind(&first_user_id)
        .execute(pool)
        .await
        .expect("account deactivation should revoke outstanding password reset tokens");

    let revoked_token_reason: Option<String> = sqlx::query_scalar(
        "SELECT revocation_reason FROM public.password_reset_tokens WHERE id = $1::uuid",
    )
    .bind(&second_token_id)
    .fetch_one(pool)
    .await
    .expect("revoked token reason should be queryable");
    assert_eq!(
        revoked_token_reason.as_deref(),
        Some("account_status_changed")
    );

    sqlx::query(
        "UPDATE public.account_recovery_cases SET status = 'completed' WHERE id = $1::uuid",
    )
    .bind(&recovery_case_id)
    .execute(pool)
    .await
    .expect("approved recovery case may be completed");

    let rejected_session_id: String = sqlx::query_scalar(
        "INSERT INTO public.verification_sessions (purpose, channel, destination_ciphertext, destination_nonce, encryption_key_version, destination_hmac, hmac_key_version, expires_at) \
         VALUES ('account_recovery', 'email', $1, $2, 'enc-v1', $3, 'hmac-v1', clock_timestamp() + INTERVAL '20 minutes') \
         RETURNING id::text",
    )
    .bind(vec![51_u8])
    .bind(vec![52_u8])
    .bind(vec![53_u8; 32])
    .fetch_one(pool)
    .await
    .expect("second recovery verification session should be created");

    let rejected_case_id: String = sqlx::query_scalar(
        "INSERT INTO public.account_recovery_cases (verification_session_id, requested_action) \
         VALUES ($1::uuid, 'password_reset') RETURNING id::text",
    )
    .bind(&rejected_session_id)
    .fetch_one(pool)
    .await
    .expect("second recovery case should be created");

    sqlx::query(
        "UPDATE public.verification_sessions SET status = 'verified', verified_at = clock_timestamp() WHERE id = $1::uuid",
    )
    .bind(&rejected_session_id)
    .execute(pool)
    .await
    .expect("second recovery session should be verified");

    sqlx::query("UPDATE public.verification_sessions SET user_id = $2::uuid WHERE id = $1::uuid")
        .bind(&rejected_session_id)
        .bind(&second_user_id)
        .execute(pool)
        .await
        .expect("verified recovery session should be bound to the selected account");

    sqlx::query(
        "UPDATE public.account_recovery_cases SET target_user_id = $2::uuid, status = 'under_review' WHERE id = $1::uuid",
    )
    .bind(&rejected_case_id)
    .bind(&second_user_id)
    .execute(pool)
    .await
    .expect("second recovery case should enter review");

    let first_review = sqlx::query_scalar::<_, bool>(
        "SELECT public.review_account_recovery_case($1::uuid, $2::uuid, $3, 'rejected', 'insufficient_evidence')",
    )
    .bind(&rejected_case_id)
    .bind(&reviewer_user_id)
    .bind(&reviewer_session_hash)
    .fetch_one(pool);
    let second_review = sqlx::query_scalar::<_, bool>(
        "SELECT public.review_account_recovery_case($1::uuid, $2::uuid, $3, 'rejected', 'insufficient_evidence')",
    )
    .bind(&rejected_case_id)
    .bind(&reviewer_user_id)
    .bind(&reviewer_session_hash)
    .fetch_one(pool);

    let (first_review, second_review) = tokio::join!(first_review, second_review);
    assert_ne!(
        first_review.is_ok(),
        second_review.is_ok(),
        "concurrent review attempts must produce exactly one decision"
    );
    assert!(
        first_review
            .or(second_review)
            .expect("one review should succeed"),
        "the winning review should be recorded"
    );

    let rejected_status: (String, bool) = sqlx::query_as(
        "SELECT status, resolved_at IS NOT NULL FROM public.account_recovery_cases WHERE id = $1::uuid",
    )
    .bind(&rejected_case_id)
    .fetch_one(pool)
    .await
    .expect("rejected recovery case state should be queryable");
    assert_eq!(rejected_status, ("rejected".to_owned(), true));
}

async fn assert_identity_audit_event_invariants(pool: &PgPool) {
    let subject_user_id = insert_active_email(pool, "identity-audit-subject@example.test").await;

    let event_id: String = sqlx::query_scalar(
        "INSERT INTO public.identity_audit_events (occurred_at, actor_type, actor_id, subject_user_id, action_code, outcome, reason_code, request_id, metadata) \
         VALUES ('2000-01-01 00:00:00+00'::timestamptz, 'user', $1::uuid, $1::uuid, 'account.recovery.requested', 'success', 'contact_verified', gen_random_uuid(), jsonb_build_object('channel', 'email', 'purpose', 'account_recovery', 'risk_level', 'elevated')) \
         RETURNING id::text",
    )
    .bind(&subject_user_id)
    .fetch_one(pool)
    .await
    .expect("valid identity audit event should be accepted");

    let timestamp_is_database_managed: bool = sqlx::query_scalar(
        "SELECT occurred_at > clock_timestamp() - INTERVAL '1 minute' FROM public.identity_audit_events WHERE id = $1::uuid",
    )
    .bind(&event_id)
    .fetch_one(pool)
    .await
    .expect("audit event timestamp should be queryable");
    assert!(timestamp_is_database_managed);

    assert!(
        sqlx::query(
            "INSERT INTO public.identity_audit_events (actor_type, action_code, outcome, metadata) \
             VALUES ('system', 'account.password.changed', 'success', jsonb_build_object('password', 'must-not-be-stored'))",
        )
        .execute(pool)
        .await
        .is_err(),
        "audit metadata must reject sensitive or unapproved keys"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.identity_audit_events (actor_type, action_code, outcome, metadata) \
             VALUES ('system', 'account.recovery.requested', 'success', jsonb_build_object('resource_type', jsonb_build_object('nested', 'value')))",
        )
        .execute(pool)
        .await
        .is_err(),
        "audit metadata must reject nested payloads"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.identity_audit_events (actor_type, action_code, outcome) \
             VALUES ('system', 'Invalid Action', 'success')",
        )
        .execute(pool)
        .await
        .is_err(),
        "audit action codes must use the normalized identifier format"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.identity_audit_events (actor_type, action_code, outcome) \
             VALUES ('system', 'account.recovery.requested', 'pending')",
        )
        .execute(pool)
        .await
        .is_err(),
        "audit outcomes must use the supported result vocabulary"
    );

    assert!(
        sqlx::query(
            "UPDATE public.identity_audit_events SET outcome = 'failure' WHERE id = $1::uuid",
        )
        .bind(&event_id)
        .execute(pool)
        .await
        .is_err(),
        "identity audit events must reject updates"
    );

    assert!(
        sqlx::query("DELETE FROM public.identity_audit_events WHERE id = $1::uuid")
            .bind(&event_id)
            .execute(pool)
            .await
            .is_err(),
        "identity audit events must reject deletes"
    );

    assert!(
        sqlx::query("TRUNCATE TABLE public.identity_audit_events")
            .execute(pool)
            .await
            .is_err(),
        "identity audit events must reject truncation"
    );
}

async fn assert_profile_and_policy_records(pool: &PgPool) {
    let user_id: String = sqlx::query_scalar(
        "SELECT public.create_client_user_with_profile_and_policy(\
            'Nama Tampilan', 'business_owner', 'email', 'profile-policy@example.test', NULL, \
            'encoded-password-hash', 'terms-v1', 'privacy-v1', FALSE, NULL)::text",
    )
    .fetch_one(pool)
    .await
    .expect("registration should persist a public profile and required policy acceptance");

    let display_name: String = sqlx::query_scalar(
        "SELECT display_name FROM public.user_profiles WHERE user_id = $1::uuid",
    )
    .bind(&user_id)
    .fetch_one(pool)
    .await
    .expect("profile display name should be stored");
    assert_eq!(display_name, "Nama Tampilan");

    let policy_records: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.policy_acceptance_records \
         WHERE user_id = $1::uuid AND policy_version IN ('terms-v1', 'privacy-v1')",
    )
    .bind(&user_id)
    .fetch_one(pool)
    .await
    .expect("required policy acceptance records should be queryable");
    assert_eq!(policy_records, 2);

    let marketing_events: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.consent_events WHERE user_id = $1::uuid",
    )
    .bind(&user_id)
    .fetch_one(pool)
    .await
    .expect("marketing consent history should be queryable");
    assert_eq!(marketing_events, 0, "marketing must remain off by default");

    let marketing_user_id: String = sqlx::query_scalar(
        "SELECT public.create_client_user_with_profile_and_policy(\
            'Investor Demo', 'investor', 'email', 'marketing-opt-in@example.test', NULL, \
            'encoded-password-hash', 'terms-v1', 'privacy-v1', TRUE, 'marketing-v1')::text",
    )
    .fetch_one(pool)
    .await
    .expect("explicit marketing opt-in should be recorded");
    let marketing_action: String =
        sqlx::query_scalar("SELECT action FROM public.consent_events WHERE user_id = $1::uuid")
            .bind(&marketing_user_id)
            .fetch_one(pool)
            .await
            .expect("marketing opt-in event should be queryable");
    assert_eq!(marketing_action, "granted");

    assert!(
        sqlx::query("UPDATE public.policy_acceptance_records SET policy_version = 'tampered' WHERE user_id = $1::uuid")
            .bind(&user_id)
            .execute(pool)
            .await
            .is_err(),
        "policy acceptance history must be append-only"
    );
    assert!(
        sqlx::query("DELETE FROM public.consent_events WHERE user_id = $1::uuid")
            .bind(&marketing_user_id)
            .execute(pool)
            .await
            .is_err(),
        "consent history must be append-only"
    );
    assert!(
        raw_sql("TRUNCATE TABLE public.policy_acceptance_records")
            .execute(pool)
            .await
            .is_err(),
        "policy acceptance history must reject TRUNCATE"
    );
    assert!(
        raw_sql("TRUNCATE TABLE public.consent_events")
            .execute(pool)
            .await
            .is_err(),
        "consent history must reject TRUNCATE"
    );
    assert!(
        sqlx::query(
            "SELECT public.create_client_user_with_profile_and_policy(\
            '   ', 'investor', 'email', 'invalid-display-name@example.test', NULL, \
            'encoded-password-hash', 'terms-v1', 'privacy-v1', FALSE, NULL)"
        )
        .execute(pool)
        .await
        .is_err(),
        "blank public display names must be rejected"
    );
    assert!(
        sqlx::query(
            "SELECT public.create_client_user_with_profile_and_policy(\
            'No Policy', 'investor', 'email', 'missing-policy-version@example.test', NULL, \
            'encoded-password-hash', NULL, 'privacy-v1', FALSE, NULL)"
        )
        .execute(pool)
        .await
        .is_err(),
        "required policy versions must be supplied"
    );
}

async fn assert_single_admin_totp_schema(pool: &PgPool) {
    let (is_unique, predicate): (bool, Option<String>) = sqlx::query_as(
        "SELECT i.indisunique, pg_catalog.pg_get_expr(i.indpred, i.indrelid) \
           FROM pg_catalog.pg_index AS i \
          WHERE i.indexrelid = 'public.users_single_admin_account_uidx'::regclass",
    )
    .fetch_one(pool)
    .await
    .expect("single-Admin uniqueness index should exist");
    assert!(is_unique, "Admin account index must be unique");
    assert!(
        predicate
            .as_deref()
            .is_some_and(|value| value.contains("admin")),
        "Admin uniqueness index must apply only to Admin accounts"
    );

    let required_columns: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint \
           FROM information_schema.columns \
          WHERE table_schema = 'public' \
            AND table_name = 'admin_totp_credentials' \
            AND column_name = ANY(ARRAY[ \
                'user_id', 'secret_ciphertext', 'secret_nonce', \
                'encryption_key_version', 'enabled_at', 'last_accepted_step', \
                'created_at', 'updated_at' \
            ]::text[])",
    )
    .fetch_one(pool)
    .await
    .expect("Admin TOTP credential columns should be inspectable");
    assert_eq!(required_columns, 8);

    let required_guards: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint \
           FROM pg_catalog.pg_trigger \
          WHERE tgrelid = 'public.admin_totp_credentials'::regclass \
            AND NOT tgisinternal \
            AND tgname = ANY(ARRAY[ \
                'admin_totp_credentials_10_guard', \
                'admin_totp_credentials_20_no_delete', \
                'admin_totp_credentials_30_no_truncate' \
            ]::text[])",
    )
    .fetch_one(pool)
    .await
    .expect("Admin TOTP credential guards should be inspectable");
    assert_eq!(required_guards, 3);

    let admin_user_id: String = sqlx::query_scalar(
        "SELECT id::text FROM public.users WHERE primary_role = 'admin' AND account_status = 'active'",
    )
    .fetch_one(pool)
    .await
    .expect("recovery review fixture should provision the single active Admin");

    let duplicate_admin = sqlx::query(
        "INSERT INTO public.users (primary_role, account_status, email_normalized, initial_contact_channel, email_verified_at, initial_contact_verified_at, password_hash) \
         VALUES ('admin', 'active', 'second-admin@example.test', 'email', clock_timestamp(), clock_timestamp(), 'test-only-hash')",
    )
    .execute(pool)
    .await;
    assert!(
        duplicate_admin.is_err(),
        "database must reject a second Admin account"
    );

    let operator_id = "00000000-0000-4000-8000-000000000002";
    let initial_totp_secret = [0x5A_u8; 20];
    let initial_encrypted =
        encrypt_admin_totp_secret(&VerificationTestKeys, &admin_user_id, &initial_totp_secret)
            .expect("initial Admin TOTP secret should be encrypted");
    let initial_key_version = key_version_label(initial_encrypted.key_version)
        .expect("initial Admin TOTP key version should be valid");
    let prepared: bool = sqlx::query_scalar(
        "SELECT public.prepare_admin_totp_enrollment($1::uuid, $2, $3, $4, $5::uuid)",
    )
    .bind(&admin_user_id)
    .bind(&initial_encrypted.ciphertext)
    .bind(initial_encrypted.nonce.to_vec())
    .bind(initial_key_version)
    .bind(operator_id)
    .fetch_one(pool)
    .await
    .expect("operator should prepare the initial encrypted TOTP enrollment");
    assert!(prepared);

    let duplicate_preparation = sqlx::query_scalar::<_, bool>(
        "SELECT public.prepare_admin_totp_enrollment($1::uuid, $2, $3, 'v1', $4::uuid)",
    )
    .bind(&admin_user_id)
    .bind(vec![0xA6_u8; 36])
    .bind(vec![0x5B_u8; 12])
    .bind(operator_id)
    .fetch_one(pool)
    .await;
    assert!(
        duplicate_preparation.is_err(),
        "initial Admin TOTP enrollment must not overwrite an existing credential"
    );

    let enrollment_activated: bool =
        sqlx::query_scalar("SELECT public.activate_admin_totp_enrollment($1::uuid, 100, $2::uuid)")
            .bind(&admin_user_id)
            .bind(operator_id)
            .fetch_one(pool)
            .await
            .expect("operator should activate TOTP only after enrollment verification");
    assert!(enrollment_activated);

    sqlx::query(
        "UPDATE public.admin_totp_credentials SET last_accepted_step = 101 WHERE user_id = $1::uuid",
    )
    .bind(&admin_user_id)
    .execute(pool)
    .await
    .expect("TOTP replay counter should advance");

    let replay = sqlx::query(
        "UPDATE public.admin_totp_credentials SET last_accepted_step = 100 WHERE user_id = $1::uuid",
    )
    .bind(&admin_user_id)
    .execute(pool)
    .await;
    assert!(
        replay.is_err(),
        "TOTP replay counter must never move backwards"
    );

    let secret_replacement = sqlx::query(
        "UPDATE public.admin_totp_credentials SET secret_ciphertext = $2 WHERE user_id = $1::uuid",
    )
    .bind(&admin_user_id)
    .bind(vec![0xB6_u8; 36])
    .execute(pool)
    .await;
    assert!(
        secret_replacement.is_err(),
        "enabled TOTP secret cannot be replaced by a direct update"
    );

    let credential_delete =
        sqlx::query("DELETE FROM public.admin_totp_credentials WHERE user_id = $1::uuid")
            .bind(&admin_user_id)
            .execute(pool)
            .await;
    assert!(
        credential_delete.is_err(),
        "Admin TOTP credential cannot be deleted directly"
    );
}

async fn assert_admin_authentication_service(pool: &PgPool) {
    let (admin_id, admin_email): (String, String) = sqlx::query_as(
        "SELECT id::text, email_normalized FROM public.users \
          WHERE primary_role = 'admin' AND account_status = 'active'",
    )
    .fetch_one(pool)
    .await
    .expect("active Admin fixture should be available for service integration");

    let password_hash = hash_password(b"Admin-Service-Test-Password-2026!")
        .expect("Admin integration password should be hashed");
    sqlx::query("UPDATE public.users SET password_hash = $2 WHERE id = $1::uuid")
        .bind(&admin_id)
        .bind(password_hash)
        .execute(pool)
        .await
        .expect("Admin integration password should be stored through test fixture setup");

    let rate_limit_key = AuthenticationRateLimitKey::from_hex(&"cd".repeat(32))
        .expect("test Admin rate-limit key should be valid");
    let service = AdminAuthenticationService::new(pool, &rate_limit_key, &VerificationTestKeys);

    let current_totp_step: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()) / 30)::bigint")
            .fetch_one(pool)
            .await
            .expect("current TOTP step should be available for service integration");
    let service_totp_secret = [0x6B_u8; 20];
    let service_encrypted =
        encrypt_admin_totp_secret(&VerificationTestKeys, &admin_id, &service_totp_secret)
            .expect("service TOTP secret should be encrypted");
    let service_key_version = key_version_label(service_encrypted.key_version)
        .expect("service TOTP key version should be valid");
    let reset_prepared: bool = sqlx::query_scalar(
        "SELECT public.reset_admin_totp_enrollment( \
            $1::uuid, $2, $3, $4, 'authenticator_lost', $5::uuid \
        )",
    )
    .bind(&admin_id)
    .bind(&service_encrypted.ciphertext)
    .bind(service_encrypted.nonce.to_vec())
    .bind(service_key_version)
    .bind("00000000-0000-4000-8000-000000000002")
    .fetch_one(pool)
    .await
    .expect("operator should prepare a fresh TOTP secret for end-to-end service coverage");
    assert!(reset_prepared);
    // Assert operator activation before exercising the Admin login service.
    let reactivated: bool =
        sqlx::query_scalar("SELECT public.activate_admin_totp_enrollment($1::uuid, $2, $3::uuid)")
            .bind(&admin_id)
            .bind(current_totp_step - 1)
            .bind("00000000-0000-4000-8000-000000000002")
            .fetch_one(pool)
            .await
            .expect("operator should activate the fresh TOTP secret before service login");
    assert!(reactivated);

    let invalid_password = service
        .begin_login(
            &admin_email,
            "Wrong-Admin-Password-2026!",
            "192.0.2.44".parse().expect("test IP should be valid"),
        )
        .await;
    assert!(matches!(
        invalid_password,
        Err(usahapintar_api::identity::AdminAuthenticationError::InvalidCredentials)
    ));

    let challenge = service
        .begin_login(
            &admin_email,
            "Admin-Service-Test-Password-2026!",
            "192.0.2.44".parse().expect("test IP should be valid"),
        )
        .await
        .expect("valid Admin password should issue a TOTP challenge");
    assert!(challenge.expires_at_unix > 0);

    let valid_totp_code = usahapintar_api::identity::totp::code_for_step(
        &service_totp_secret,
        current_totp_step as u64,
    )
    .expect("valid TOTP code should be generated for the current time-step");
    let session = service
        .complete_login(challenge.token.as_str(), &valid_totp_code)
        .await
        .expect("valid TOTP should atomically create an isolated Admin session");
    assert_eq!(session.user_id, admin_id);

    let authenticated = service
        .validate_session(session.token.as_str())
        .await
        .expect("new Admin session should validate");
    assert_eq!(authenticated.user_id, admin_id);
    assert!(service
        .logout(session.token.as_str())
        .await
        .expect("Admin logout should revoke the isolated session"));
    assert!(matches!(
        service.validate_session(session.token.as_str()).await,
        Err(usahapintar_api::identity::AdminAuthenticationError::InvalidSession)
    ));

    let failed_totp_challenge = service
        .begin_login(
            &admin_email,
            "Admin-Service-Test-Password-2026!",
            "192.0.2.44".parse().expect("test IP should be valid"),
        )
        .await
        .expect("correct password should issue a second TOTP challenge");
    let invalid_totp_code = usahapintar_api::identity::totp::code_for_step(
        &service_totp_secret,
        current_totp_step as u64 + 5,
    )
    .expect("out-of-window TOTP code should be generated");
    let invalid_totp = service
        .complete_login(failed_totp_challenge.token.as_str(), &invalid_totp_code)
        .await;
    assert!(matches!(
        invalid_totp,
        Err(usahapintar_api::identity::AdminAuthenticationError::InvalidChallenge)
    ));

    let failed_audit_events: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.identity_audit_events \
          WHERE actor_type = 'system' AND subject_user_id IS NULL \
            AND action_code = 'identity.admin.login.failed' \
            AND outcome = 'failure'",
    )
    .fetch_one(pool)
    .await
    .expect("failed Admin login audit events should be inspectable");
    assert_eq!(failed_audit_events, 2);
}

async fn assert_database_principal_bootstrap(pool: &PgPool) {
    let admin_url = std::env::var("CI_TARGET_ADMIN_DATABASE_URL").expect(
        "CI must provide a disposable target-database DBA connection for privilege bootstrap tests",
    );
    let admin_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_url)
        .await
        .expect("disposable DBA connection should succeed");

    raw_sql(include_str!(
        "../database/bootstrap/002_identity_core_grants.sql"
    ))
    .execute(&admin_pool)
    .await
    .expect("guarded Identity Core grants should apply after migrations");

    let app_url = std::env::var("APP_DATABASE_URL")
        .expect("CI must provide a separate least-privilege application connection");
    let app_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&app_url)
        .await
        .expect("application principal connection should succeed");

    let app_role: String = sqlx::query_scalar("SELECT current_user")
        .fetch_one(&app_pool)
        .await
        .expect("application principal should be identifiable");
    assert_eq!(app_role, "usahapintar_app_login");

    let app_flags: (bool, bool, bool, bool, bool) = sqlx::query_as(
        "SELECT rolsuper, rolcreatedb, rolcreaterole, rolbypassrls, rolreplication \
         FROM pg_catalog.pg_roles WHERE rolname = current_user",
    )
    .fetch_one(&app_pool)
    .await
    .expect("application role flags should be queryable");
    assert_eq!(app_flags, (false, false, false, false, false));

    let app_group_member: bool =
        sqlx::query_scalar("SELECT pg_has_role(current_user, 'usahapintar_app', 'member')")
            .fetch_one(&app_pool)
            .await
            .expect("application group membership should be queryable");
    assert!(app_group_member);

    let schema_create: bool =
        sqlx::query_scalar("SELECT has_schema_privilege(current_user, 'public', 'CREATE')")
            .fetch_one(&app_pool)
            .await
            .expect("application schema privilege should be queryable");
    assert!(!schema_create, "application must not create schema objects");

    let direct_user_access: (bool, bool, bool) = sqlx::query_as(
        "SELECT has_table_privilege(current_user, 'public.users', 'SELECT'), \
                has_table_privilege(current_user, 'public.users', 'INSERT'), \
                has_table_privilege(current_user, 'public.users', 'UPDATE')",
    )
    .fetch_one(&app_pool)
    .await
    .expect("direct account table privileges should be queryable");
    assert_eq!(direct_user_access, (false, false, false));

    let direct_sensitive_table_access: bool = sqlx::query_scalar(
        "SELECT EXISTS ( \
             SELECT 1 \
               FROM unnest(ARRAY[ \
                   'verification_sessions', 'verification_challenges', \
                   'otp_verification_attempt_events', 'otp_delivery_events', 'sessions', \
                   'recovery_contacts', 'account_recovery_cases', 'password_reset_tokens', \
                   'account_recovery_case_signals', 'account_recovery_review_decisions', 'admin_totp_credentials', \
                   'admin_login_rate_limits', 'admin_login_challenges', 'admin_sessions', \
                   'identity_abuse_rate_limits', \
                   'user_profiles', 'policy_acceptance_records', 'consent_events' \
               ]::TEXT[]) AS t(table_name) \
              WHERE has_table_privilege(current_user, pg_catalog.format('public.%I', table_name), 'SELECT') \
                 OR has_table_privilege(current_user, pg_catalog.format('public.%I', table_name), 'INSERT') \
                 OR has_table_privilege(current_user, pg_catalog.format('public.%I', table_name), 'UPDATE') \
                 OR has_table_privilege(current_user, pg_catalog.format('public.%I', table_name), 'DELETE') \
                 OR has_table_privilege(current_user, pg_catalog.format('public.%I', table_name), 'TRUNCATE') \
         )",
    )
    .fetch_one(&app_pool)
    .await
    .expect("sensitive table privileges should be queryable");
    assert!(
        !direct_sensitive_table_access,
        "application must not receive direct access to verification, session, or recovery tables"
    );

    let database_privileges: (bool, bool, bool) = sqlx::query_as(
        "SELECT has_database_privilege(current_user, current_database(), 'CONNECT'), \
                has_database_privilege(current_user, current_database(), 'CREATE'), \
                has_database_privilege(current_user, current_database(), 'TEMP')",
    )
    .fetch_one(&app_pool)
    .await
    .expect("application database privileges should be queryable");
    assert_eq!(database_privileges, (true, false, false));

    let audit_access: (bool, bool, bool, bool, bool) = sqlx::query_as(
        "SELECT has_table_privilege(current_user, 'public.identity_audit_events', 'INSERT'), \
                has_table_privilege(current_user, 'public.identity_audit_events', 'SELECT'), \
                has_table_privilege(current_user, 'public.identity_audit_events', 'UPDATE'), \
                has_table_privilege(current_user, 'public.identity_audit_events', 'DELETE'), \
                has_table_privilege(current_user, 'public.identity_audit_events', 'TRUNCATE')",
    )
    .fetch_one(&app_pool)
    .await
    .expect("audit table privileges should be queryable");
    assert_eq!(audit_access, (true, false, false, false, false));

    let function_access: (bool, bool, bool, bool, bool, bool, bool, bool) = sqlx::query_as(
        "SELECT has_function_privilege(current_user, \
                    'public.create_client_user(text,text,text,text,text)', 'EXECUTE'), \
                has_function_privilege(current_user, \
                    'public.is_safe_identity_audit_metadata(jsonb)', 'EXECUTE'), \
                has_function_privilege(current_user, \
                    'public.guard_identity_audit_event()', 'EXECUTE'), \
                has_function_privilege(current_user, \
                    'public.create_registration_verification_session(uuid,uuid,text,bytea,bytea,text,bytea,text,timestamptz)', 'EXECUTE'), \
                has_function_privilege(current_user, \
                    'public.get_registration_verification_session(uuid)', 'EXECUTE'), \
                has_function_privilege(current_user, \
                    'public.get_pending_otp_challenge(uuid)', 'EXECUTE'), \
                has_function_privilege(current_user, \
                    'public.reserve_otp_challenge(uuid,uuid,bytea,text,bytea[])', 'EXECUTE'), \
                has_function_privilege(current_user, \
                    'public.record_otp_delivery_result(uuid,text)', 'EXECUTE')",
    )
    .fetch_one(&app_pool)
    .await
    .expect("core function privileges should be queryable");
    assert_eq!(
        function_access,
        (false, true, false, true, true, true, true, true)
    );

    let recovery_function_access: (bool, bool, bool, bool, bool, bool, bool, bool, bool) =
        sqlx::query_as(
            "SELECT has_function_privilege(current_user, \
                        'public.record_otp_verification_attempt(uuid,uuid,boolean,bytea[])', 'EXECUTE'), \
                    has_function_privilege(current_user, \
                        'public.create_client_user_with_profile_and_policy(text,text,text,text,text,text,text,text,boolean,text)', 'EXECUTE'), \
                    has_function_privilege(current_user, \
                        'public.record_account_recovery_session_signal(uuid,bytea)', 'EXECUTE'), \
                    has_function_privilege(current_user, \
                        'public.review_account_recovery_case(uuid,uuid,bytea,text,text)', 'EXECUTE'), \
                    has_function_privilege(current_user, \
                        'public.get_account_recovery_review_context(uuid,uuid,bytea)', 'EXECUTE'), \
                    has_function_privilege(current_user, \
                        'public.get_approved_account_recovery_delivery_context(uuid)', 'EXECUTE'), \
                    has_function_privilege(current_user, \
                        'public.create_password_reset_token(uuid,bytea,bytea[])', 'EXECUTE'), \
                    has_function_privilege(current_user, \
                        'public.revoke_password_reset_token(bytea)', 'EXECUTE'), \
                    has_function_privilege(current_user, \
                        'public.complete_password_reset(bytea,text)', 'EXECUTE')",
        )
        .fetch_one(&app_pool)
        .await
        .expect("recovery function privileges should be queryable");
    assert_eq!(
        recovery_function_access,
        (true, true, true, true, true, true, true, true, true)
    );

    let admin_function_grants: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint \
           FROM pg_catalog.unnest(ARRAY[ \
               'public.find_active_admin_password_login(text)'::regprocedure, \
               'public.get_admin_login_totp_credentials(bytea)'::regprocedure, \
               'public.reserve_admin_login_attempt(bytea,bytea)'::regprocedure, \
               'public.create_admin_login_challenge(uuid,bytea)'::regprocedure, \
               'public.record_admin_login_challenge_failure(bytea)'::regprocedure, \
               'public.complete_admin_login(bytea,bigint,bytea)'::regprocedure, \
               'public.get_active_admin_session(bytea)'::regprocedure, \
               'public.revoke_admin_session(bytea)'::regprocedure \
           ]::regprocedure[]) AS f(signature) \
          WHERE has_function_privilege(current_user, f.signature, 'EXECUTE')",
    )
    .fetch_one(&app_pool)
    .await
    .expect("Admin authentication function grants should be inspectable");
    assert_eq!(
        admin_function_grants, 8,
        "application should execute only the explicit Admin authentication functions"
    );

    let identity_abuse_function_access: bool = sqlx::query_scalar(
        "SELECT has_function_privilege(current_user, \
                'public.reserve_identity_abuse_attempt(text,bytea,integer,integer)', 'EXECUTE')",
    )
    .fetch_one(&app_pool)
    .await
    .expect("Identity abuse-rate-limit function privilege should be queryable");
    assert!(
        identity_abuse_function_access,
        "application should execute the Identity abuse-rate-limit function"
    );

    let operator_function_access: (bool, bool, bool) = sqlx::query_as(
        "SELECT has_function_privilege(current_user, \
                    'public.prepare_admin_totp_enrollment(uuid,bytea,bytea,text,uuid)', 'EXECUTE'), \
                has_function_privilege(current_user, \
                    'public.activate_admin_totp_enrollment(uuid,bigint,uuid)', 'EXECUTE'), \
                has_function_privilege(current_user, \
                    'public.reset_admin_totp_enrollment(uuid,bytea,bytea,text,text,uuid)', 'EXECUTE')",
    )
    .fetch_one(&app_pool)
    .await
    .expect("operator-only TOTP procedure privileges should be queryable");
    assert_eq!(
        operator_function_access,
        (false, false, false),
        "application principal must not execute operator-only TOTP provisioning or recovery procedures"
    );

    let created_user_id: String =
        sqlx::query_scalar("SELECT public.create_client_user_with_profile_and_policy($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)::text")
            .bind("Pemilik Bootstrap")
            .bind("business_owner")
            .bind("email")
            .bind("principal-bootstrap@example.test")
            .bind(Option::<String>::None)
            .bind("test-only-encoded-password-hash")
            .bind("terms-v1")
            .bind("privacy-v1")
            .bind(false)
            .bind(Option::<String>::None)
            .fetch_one(&app_pool)
            .await
            .expect("application may create a pending account through the guarded function");
    assert!(!created_user_id.is_empty());

    let created_profile: String = sqlx::query_scalar(
        "SELECT display_name FROM public.user_profiles WHERE user_id = $1::uuid",
    )
    .bind(&created_user_id)
    .fetch_one(pool)
    .await
    .expect("migration principal should verify the application-created profile");
    assert_eq!(created_profile, "Pemilik Bootstrap");

    let created_acceptances: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.policy_acceptance_records WHERE user_id = $1::uuid",
    )
    .bind(&created_user_id)
    .fetch_one(pool)
    .await
    .expect("migration principal should verify required policy acceptance");
    assert_eq!(created_acceptances, 2);

    sqlx::query(
        "INSERT INTO public.identity_audit_events (actor_type, action_code, outcome) \
         VALUES ('system', 'database.bootstrap.checked', 'success')",
    )
    .execute(&app_pool)
    .await
    .expect("application may append a minimal audit event");

    let migrator_flags: (bool, bool, bool, bool) = sqlx::query_as(
        "SELECT rolsuper, rolcreatedb, rolcreaterole, rolbypassrls \
         FROM pg_catalog.pg_roles WHERE rolname = 'usahapintar_migrator'",
    )
    .fetch_one(pool)
    .await
    .expect("migration role flags should be queryable");
    assert_eq!(migrator_flags, (false, false, false, false));

    let migrator_can_create: bool = sqlx::query_scalar(
        "SELECT has_schema_privilege('usahapintar_migrator', 'public', 'CREATE')",
    )
    .fetch_one(pool)
    .await
    .expect("migration schema privilege should be queryable");
    assert!(migrator_can_create);

    let database_owner_is_migrator: bool = sqlx::query_scalar(
        "SELECT pg_catalog.pg_get_userbyid(datdba) = 'usahapintar_migrator' \
         FROM pg_catalog.pg_database WHERE datname = pg_catalog.current_database()",
    )
    .fetch_one(pool)
    .await
    .expect("database owner should be queryable");
    assert!(
        !database_owner_is_migrator,
        "migration role must not own the database"
    );

    let public_default_privileges: bool = sqlx::query_scalar(
        "SELECT EXISTS ( \
             SELECT 1 \
               FROM pg_catalog.pg_default_acl AS d \
               CROSS JOIN LATERAL pg_catalog.aclexplode(d.defaclacl) AS acl \
              WHERE d.defaclrole = 'usahapintar_migrator'::pg_catalog.regrole \
                AND d.defaclnamespace IN (0, 'public'::pg_catalog.regnamespace) \
                AND d.defaclobjtype IN ('r', 'S', 'f') \
                AND acl.grantee = 0 \
         )",
    )
    .fetch_one(pool)
    .await
    .expect("migrator default privileges should be queryable");
    assert!(
        !public_default_privileges,
        "new objects created by migrator must not be granted to PUBLIC"
    );

    let unexpected_app_memberships: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint \
         FROM pg_catalog.pg_auth_members \
         WHERE member = 'usahapintar_app_login'::pg_catalog.regrole \
           AND roleid <> 'usahapintar_app'::pg_catalog.regrole",
    )
    .fetch_one(pool)
    .await
    .expect("application role memberships should be queryable");
    assert_eq!(unexpected_app_memberships, 0);

    let backup_access: (bool, bool, bool) = sqlx::query_as(
        "SELECT pg_has_role('usahapintar_backup_login', 'pg_read_all_data', 'member'), \
                has_table_privilege('usahapintar_backup_login', 'public.identity_audit_events', 'SELECT'), \
                has_table_privilege('usahapintar_backup_login', 'public.identity_audit_events', 'INSERT')",
    )
    .fetch_one(pool)
    .await
    .expect("backup principal privileges should be queryable");
    assert_eq!(backup_access, (true, true, false));

    let monitor_access: (bool, bool) = sqlx::query_as(
        "SELECT pg_has_role('usahapintar_monitor_login', 'pg_monitor', 'member'), \
                has_table_privilege('usahapintar_monitor_login', 'public.identity_audit_events', 'SELECT')",
    )
    .fetch_one(pool)
    .await
    .expect("monitoring principal privileges should be queryable");
    assert_eq!(monitor_access, (true, false));

    let audit_reader_access: (bool, bool) = sqlx::query_as(
        "SELECT has_table_privilege('usahapintar_audit_reader', 'public.identity_audit_events', 'SELECT'), \
                has_table_privilege('usahapintar_audit_reader', 'public.identity_audit_events', 'INSERT')",
    )
    .fetch_one(pool)
    .await
    .expect("audit reader privileges should be queryable");
    assert_eq!(audit_reader_access, (true, false));

    assert_verification_orchestration(&app_pool, pool).await;

    app_pool.close().await;
    admin_pool.close().await;
}

async fn assert_verification_orchestration(app_pool: &PgPool, migrator_pool: &PgPool) {
    let keys = VerificationTestKeys;
    let delivery = FakeOtpDelivery {
        messages: Mutex::new(Vec::new()),
        status: DispatchStatus::Accepted,
    };
    let flowkirim = FakeFlowKirim;
    let service = VerificationService::new_with_flowkirim(app_pool, &keys, &delivery, &flowkirim);
    let registration = ClientRegistration {
        display_name: "Pemilik Usaha Demo",
        role: ClientRole::BusinessOwner,
        initial_contact_channel: ContactChannel::Email,
        email_normalized: Some("orchestration@example.test"),
        phone_e164: None,
        password_hash: "test-only-encoded-password-hash",
        terms_of_service_version: "terms-v1",
        privacy_policy_version: "privacy-v1",
        marketing_consent: false,
        marketing_notice_version: None,
    };

    let started = service
        .register(&registration)
        .await
        .expect("registration orchestration should create a pending account and dispatch OTP");
    assert_eq!(started.dispatch_status, DispatchStatus::Accepted);

    let registration_state: (String, i64, i64) = sqlx::query_as(
        "SELECT p.display_name, \
                (SELECT count(*)::bigint FROM public.policy_acceptance_records AS a WHERE a.user_id = p.user_id), \
                (SELECT count(*)::bigint FROM public.consent_events AS c WHERE c.user_id = p.user_id) \
         FROM public.user_profiles AS p WHERE p.user_id = $1::uuid",
    )
    .bind(&started.user_id)
    .fetch_one(migrator_pool)
    .await
    .expect("registration profile and policy state should be persisted");
    assert_eq!(registration_state, ("Pemilik Usaha Demo".to_owned(), 2, 0));

    let code = {
        let messages = delivery
            .messages
            .lock()
            .expect("fake delivery message lock should be available");
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].channel,
            usahapintar_api::identity::DeliveryChannel::Email
        );
        messages[0].code.as_str().to_owned()
    };

    let incorrect_code = if code == "000000" { "000001" } else { "000000" };
    assert!(!service
        .verify(&started.session_id, incorrect_code)
        .await
        .expect("incorrect OTP should be recorded as a failed attempt"));
    assert!(service
        .verify(&started.session_id, &code)
        .await
        .expect("correct OTP should verify and activate the pending account"));

    let account_status: String =
        sqlx::query_scalar("SELECT account_status FROM public.users WHERE id = $1::uuid")
            .bind(&started.user_id)
            .fetch_one(migrator_pool)
            .await
            .expect("test migrator should inspect the activated account");
    assert_eq!(account_status, "active");

    let email_audit_counts: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT \
            count(*) FILTER (WHERE action_code = 'identity.registration.requested'), \
            count(*) FILTER (WHERE action_code = 'identity.verification.delivery_requested'), \
            count(*) FILTER (WHERE action_code = 'identity.verification.attempt_failed'), \
            count(*) FILTER (WHERE action_code = 'identity.registration.verified') \
         FROM public.identity_audit_events WHERE subject_user_id = $1::uuid",
    )
    .bind(&started.user_id)
    .fetch_one(migrator_pool)
    .await
    .expect("migrator should inspect verification audit events");
    assert_eq!(email_audit_counts, (1, 1, 1, 1));

    let phone_registration = ClientRegistration {
        display_name: "Investor Demo",
        role: ClientRole::Investor,
        initial_contact_channel: ContactChannel::Phone,
        email_normalized: None,
        phone_e164: Some("+6281234567892"),
        password_hash: "test-only-encoded-password-hash",
        terms_of_service_version: "terms-v1",
        privacy_policy_version: "privacy-v1",
        marketing_consent: true,
        marketing_notice_version: Some("marketing-v1"),
    };
    let phone_started = service
        .register(&phone_registration)
        .await
        .expect("WhatsApp registration should create a pending account and dispatch OTP");
    assert_eq!(phone_started.dispatch_status, DispatchStatus::Accepted);

    let phone_marketing_events: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.consent_events \
         WHERE user_id = $1::uuid AND purpose = 'marketing' AND action = 'granted'",
    )
    .bind(&phone_started.user_id)
    .fetch_one(migrator_pool)
    .await
    .expect("explicit marketing opt-in should be recorded");
    assert_eq!(phone_marketing_events, 1);

    {
        let messages = delivery
            .messages
            .lock()
            .expect("fake delivery message lock should be available");
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].channel,
            usahapintar_api::identity::DeliveryChannel::Email
        );
    }
    assert!(service
        .verify(&phone_started.session_id, "123456")
        .await
        .expect("FlowKirim should verify the WhatsApp OTP and activate the pending account"));

    let phone_account_status: String =
        sqlx::query_scalar("SELECT account_status FROM public.users WHERE id = $1::uuid")
            .bind(&phone_started.user_id)
            .fetch_one(migrator_pool)
            .await
            .expect("test migrator should inspect the activated phone account");
    assert_eq!(phone_account_status, "active");

    let phone_audit_counts: (i64, i64, i64) = sqlx::query_as(
        "SELECT \
            count(*) FILTER (WHERE action_code = 'identity.registration.requested'), \
            count(*) FILTER (WHERE action_code = 'identity.verification.delivery_requested'), \
            count(*) FILTER (WHERE action_code = 'identity.registration.verified') \
         FROM public.identity_audit_events WHERE subject_user_id = $1::uuid",
    )
    .bind(&phone_started.user_id)
    .fetch_one(migrator_pool)
    .await
    .expect("migrator should inspect WhatsApp verification audit events");
    assert_eq!(phone_audit_counts, (1, 1, 1));

    let accepted_delivery_audits: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.identity_audit_events \
         WHERE action_code = 'identity.verification.delivery_result' \
           AND metadata ->> 'result_code' = 'accepted'",
    )
    .fetch_one(migrator_pool)
    .await
    .expect("migrator should inspect delivery result audit events");

    assert!(accepted_delivery_audits >= 3);

    // The same verified recovery contact may belong to multiple active accounts.
    // Selection must be explicit and must prove ownership inside PostgreSQL.
    let shared_contact_hmac = vec![0x5au8; 32];
    let recovery_contact_session_ids: Vec<String> = sqlx::query_scalar(
        "INSERT INTO public.verification_sessions ( \
             id, user_id, purpose, channel, destination_ciphertext, destination_nonce, \
             encryption_key_version, destination_hmac, hmac_key_version, expires_at \
         ) VALUES \
             (gen_random_uuid(), $1::uuid, 'recovery_contact_add', 'email', decode('01','hex'), decode('02','hex'), 'enc-v1', $3, 'hmac-v1', clock_timestamp() + INTERVAL '10 minutes'), \
             (gen_random_uuid(), $2::uuid, 'recovery_contact_add', 'email', decode('01','hex'), decode('02','hex'), 'enc-v1', $3, 'hmac-v1', clock_timestamp() + INTERVAL '10 minutes') \
         RETURNING id::text",
    )
    .bind(&started.user_id)
    .bind(&phone_started.user_id)
    .bind(&shared_contact_hmac)
    .fetch_all(migrator_pool)
    .await
    .expect("recovery contact verification sessions should be created");
    assert_eq!(recovery_contact_session_ids.len(), 2);

    sqlx::query(
        "UPDATE public.verification_sessions \
            SET status = 'verified', verified_at = clock_timestamp() \
          WHERE id IN ($1::uuid, $2::uuid)",
    )
    .bind(&recovery_contact_session_ids[0])
    .bind(&recovery_contact_session_ids[1])
    .execute(migrator_pool)
    .await
    .expect("recovery contact verification sessions should be verified");

    sqlx::query(
        "INSERT INTO public.recovery_contacts ( \
             id, user_id, channel, contact_ciphertext, contact_nonce, encryption_key_version, \
             contact_hmac, hmac_key_version, verification_session_id, verified_at \
         ) \
         SELECT gen_random_uuid(), user_id, channel, destination_ciphertext, destination_nonce, \
                encryption_key_version, destination_hmac, hmac_key_version, id, verified_at \
           FROM public.verification_sessions \
          WHERE id IN ($1::uuid, $2::uuid) AND status = 'verified'",
    )
    .bind(&recovery_contact_session_ids[0])
    .bind(&recovery_contact_session_ids[1])
    .execute(migrator_pool)
    .await
    .expect("two active accounts may share one verified recovery contact");

    let shared_contact_owners: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT rc.user_id)::bigint \
           FROM public.recovery_contacts AS rc \
           JOIN public.users AS u ON u.id = rc.user_id \
          WHERE rc.channel = 'email' AND rc.contact_hmac = $1 \
            AND rc.revoked_at IS NULL AND u.account_status = 'active'",
    )
    .bind(&shared_contact_hmac)
    .fetch_one(migrator_pool)
    .await
    .expect("shared recovery contact owners should be queryable");
    assert_eq!(shared_contact_owners, 2);

    let (recovery_session_id, recovery_case_id): (String, String) =
        sqlx::query_as("SELECT gen_random_uuid()::text, gen_random_uuid()::text")
            .fetch_one(migrator_pool)
            .await
            .expect("recovery identifiers should be generated");
    let (created_session_id, created_case_id): (String, String) = sqlx::query_as(
        "SELECT session_id::text, case_id::text \
           FROM public.create_account_recovery_verification_session( \
               $1::uuid, $2::uuid, 'password_reset', 'email', decode('01','hex'), \
               decode('02','hex'), 'enc-v1', $3, 'hmac-v1', \
               clock_timestamp() + INTERVAL '10 minutes')",
    )
    .bind(&recovery_session_id)
    .bind(&recovery_case_id)
    .bind(&shared_contact_hmac)
    .fetch_one(app_pool)
    .await
    .expect("application may create an unbound account recovery session");
    assert_eq!(created_session_id, recovery_session_id);
    assert_eq!(created_case_id, recovery_case_id);

    let recovery_destination: (String, Vec<u8>) = sqlx::query_as(
        "SELECT channel, destination_hmac \
           FROM public.get_account_recovery_verification_session($1::uuid)",
    )
    .bind(&recovery_session_id)
    .fetch_one(app_pool)
    .await
    .expect("application may load an eligible account recovery destination");
    assert_eq!(
        recovery_destination,
        ("email".to_owned(), shared_contact_hmac.clone())
    );

    let (recovery_challenge_id, recovery_delivery_event_id): (String, String) = sqlx::query_as(
        "SELECT challenge_id::text, delivery_event_id::text \
                FROM public.reserve_otp_challenge( \
                    $1::uuid, gen_random_uuid(), $2, 'otp-v1', $3)",
    )
    .bind(&recovery_session_id)
    .bind(vec![0x33u8; 32])
    .bind(vec![shared_contact_hmac.clone()])
    .fetch_one(app_pool)
    .await
    .expect("shared OTP reservation must support account recovery purpose");

    let pending_recovery_challenge: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.get_pending_otp_challenge($1::uuid)",
    )
    .bind(&recovery_session_id)
    .fetch_one(app_pool)
    .await
    .expect("generic pending OTP lookup must support recovery");
    assert_eq!(pending_recovery_challenge, 1);

    sqlx::query("SELECT public.record_otp_delivery_result($1::uuid, 'accepted')")
        .bind(&recovery_delivery_event_id)
        .execute(app_pool)
        .await
        .expect("recovery OTP delivery result should be recorded");
    let recovery_verified: bool = sqlx::query_scalar(
        "SELECT public.record_otp_verification_attempt($1::uuid, $2::uuid, TRUE, $3)",
    )
    .bind(&recovery_session_id)
    .bind(&recovery_challenge_id)
    .bind(vec![shared_contact_hmac.clone()])
    .fetch_one(app_pool)
    .await
    .expect("valid recovery OTP should advance the recovery case");
    assert!(recovery_verified);

    let recovery_case_status: String =
        sqlx::query_scalar("SELECT status FROM public.account_recovery_cases WHERE id = $1::uuid")
            .bind(&recovery_case_id)
            .fetch_one(migrator_pool)
            .await
            .expect("migrator should inspect recovery case state");
    assert_eq!(recovery_case_status, "account_selection_required");

    let recovery_context: (String, String) = sqlx::query_as(
        "SELECT session_id::text, channel \
           FROM public.get_account_recovery_selection_context($1::uuid)",
    )
    .bind(&recovery_case_id)
    .fetch_one(app_pool)
    .await
    .expect("application may load the verified recovery selection context");
    assert_eq!(
        recovery_context,
        (recovery_session_id.clone(), "email".to_owned())
    );

    let candidates: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT user_id::text, primary_role, display_name \
           FROM public.list_account_recovery_candidates($1::uuid, $2)",
    )
    .bind(&recovery_case_id)
    .bind(vec![shared_contact_hmac.clone()])
    .fetch_all(app_pool)
    .await
    .expect("verified contact may list all active accounts that share it");
    assert_eq!(candidates.len(), 2);
    assert!(candidates.iter().any(|candidate| {
        candidate.0 == started.user_id
            && candidate.1 == "business_owner"
            && candidate.2 == "Pemilik Usaha Demo"
    }));
    assert!(candidates.iter().any(|candidate| {
        candidate.0 == phone_started.user_id
            && candidate.1 == "investor"
            && candidate.2 == "Investor Demo"
    }));

    let mismatched_candidates = sqlx::query_as::<_, (String, String, String)>(
        "SELECT user_id::text, primary_role, display_name \
           FROM public.list_account_recovery_candidates($1::uuid, $2)",
    )
    .bind(&recovery_case_id)
    .bind(vec![vec![0x7fu8; 32]])
    .fetch_all(app_pool)
    .await;
    assert!(
        mismatched_candidates.is_err(),
        "candidate discovery must reject HMAC aliases that do not match the verified session"
    );

    let unrelated_user_id: String = sqlx::query_scalar("SELECT gen_random_uuid()::text")
        .fetch_one(migrator_pool)
        .await
        .expect("unrelated user id should be generated");
    let unrelated_selection = sqlx::query_scalar::<_, String>(
        "SELECT public.bind_account_recovery_session_to_account($1::uuid, $2::uuid, $3)::text",
    )
    .bind(&recovery_case_id)
    .bind(&unrelated_user_id)
    .bind(vec![shared_contact_hmac.clone()])
    .fetch_one(app_pool)
    .await;
    assert!(
        unrelated_selection.is_err(),
        "account selection must reject an account that does not own the verified contact"
    );

    let selected_user_id: String = sqlx::query_scalar(
        "SELECT public.bind_account_recovery_session_to_account($1::uuid, $2::uuid, $3)::text",
    )
    .bind(&recovery_case_id)
    .bind(&started.user_id)
    .bind(vec![shared_contact_hmac.clone()])
    .fetch_one(app_pool)
    .await
    .expect("a selected account owning the shared verified contact should bind");
    assert_eq!(selected_user_id, started.user_id);

    let selected_case: (String, String) = sqlx::query_as(
        "SELECT status, target_user_id::text \
           FROM public.account_recovery_cases WHERE id = $1::uuid",
    )
    .bind(&recovery_case_id)
    .fetch_one(migrator_pool)
    .await
    .expect("migrator should inspect selected recovery account");
    assert_eq!(
        selected_case,
        ("under_review".to_owned(), started.user_id.clone())
    );
}

async fn assert_admin_totp_operator_workflows(pool: &PgPool) {
    let admin_id: String = sqlx::query_scalar(
        "SELECT id::text FROM public.users WHERE primary_role = 'admin' AND account_status = 'active'",
    )
    .fetch_one(pool)
    .await
    .expect("active Admin should exist for operator workflow tests");

    let operator_id = "00000000-0000-4000-8000-000000000002";
    let pending_challenge_hash = vec![0x71_u8; 32];
    let _: String = sqlx::query_scalar(
        "SELECT challenge_id::text FROM public.create_admin_login_challenge($1::uuid, $2)",
    )
    .bind(&admin_id)
    .bind(&pending_challenge_hash)
    .fetch_one(pool)
    .await
    .expect("enabled Admin should be able to create a challenge before recovery");

    let active_session_hash = vec![0x72_u8; 32];
    sqlx::query("INSERT INTO public.admin_sessions (user_id, token_hash) VALUES ($1::uuid, $2)")
        .bind(&admin_id)
        .bind(&active_session_hash)
        .execute(pool)
        .await
        .expect("operator workflow fixture should create an active Admin session");

    let reset_totp_secret = [0x6B_u8; 20];
    let reset_encrypted =
        encrypt_admin_totp_secret(&VerificationTestKeys, &admin_id, &reset_totp_secret)
            .expect("replacement Admin TOTP secret should be encrypted");
    let reset_key_version = key_version_label(reset_encrypted.key_version)
        .expect("replacement Admin TOTP key version should be valid");
    let reset: bool = sqlx::query_scalar(
        "SELECT public.reset_admin_totp_enrollment($1::uuid, $2, $3, $4, 'authenticator_lost', $5::uuid)",
    )
    .bind(&admin_id)
    .bind(&reset_encrypted.ciphertext)
    .bind(reset_encrypted.nonce.to_vec())
    .bind(&reset_key_version)
    .bind(operator_id)
    .fetch_one(pool)
    .await
    .expect("operator should reset the TOTP credential through the controlled procedure");
    assert!(reset);

    let credential_state: (bool, bool, bool, bool, String) = sqlx::query_as(
        "SELECT enabled_at IS NULL, last_accepted_step IS NULL, \
                secret_ciphertext = $2, secret_nonce = $3, encryption_key_version \
           FROM public.admin_totp_credentials WHERE user_id = $1::uuid",
    )
    .bind(&admin_id)
    .bind(&reset_encrypted.ciphertext)
    .bind(reset_encrypted.nonce.to_vec())
    .fetch_one(pool)
    .await
    .expect("reset TOTP credential should be inspectable by migrator");
    assert_eq!(
        credential_state,
        (true, true, true, true, reset_key_version.clone())
    );

    let challenge_consumed: bool = sqlx::query_scalar(
        "SELECT consumed_at IS NOT NULL FROM public.admin_login_challenges WHERE challenge_hash = $1",
    )
    .bind(&pending_challenge_hash)
    .fetch_one(pool)
    .await
    .expect("operator reset should consume pending Admin challenges");
    assert!(challenge_consumed);

    let session_revocation: Option<String> = sqlx::query_scalar(
        "SELECT revocation_reason FROM public.admin_sessions WHERE token_hash = $1",
    )
    .bind(&active_session_hash)
    .fetch_one(pool)
    .await
    .expect("operator reset should revoke active Admin sessions");
    assert_eq!(session_revocation.as_deref(), Some("operator_action"));

    let reenrolled: bool =
        sqlx::query_scalar("SELECT public.activate_admin_totp_enrollment($1::uuid, 101, $2::uuid)")
            .bind(&admin_id)
            .bind(operator_id)
            .fetch_one(pool)
            .await
            .expect("operator should activate the replacement authenticator after verification");
    assert!(reenrolled);

    let audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.identity_audit_events \
          WHERE actor_type = 'operator' AND actor_id = $1::uuid \
            AND subject_user_id = $2::uuid \
            AND action_code IN (\
                'identity.admin.totp.enrollment.prepared', \
                'identity.admin.totp.enrolled', 'identity.admin.totp.reset'\
            )",
    )
    .bind(operator_id)
    .bind(&admin_id)
    .fetch_one(pool)
    .await
    .expect("operator enrollment and reset audit events should be recorded");
    assert_eq!(audit_count, 4);
}

async fn assert_admin_authentication_database_runtime(pool: &PgPool) {
    let (admin_id, admin_email): (String, String) = sqlx::query_as(
        "SELECT id::text, email_normalized FROM public.users \
          WHERE primary_role = 'admin' AND account_status = 'active'",
    )
    .fetch_one(pool)
    .await
    .expect("single active Admin fixture should exist");

    let found_admin: Option<(String, String)> = sqlx::query_as(
        "SELECT user_id::text, password_hash \
           FROM public.find_active_admin_password_login($1)",
    )
    .bind(&admin_email)
    .fetch_optional(pool)
    .await
    .expect("Admin password lookup should be callable");
    assert!(
        found_admin.is_some_and(|(user_id, password_hash)| {
            user_id == admin_id && !password_hash.is_empty()
        }),
        "Admin lookup should return the active Admin credential only"
    );

    let credential_challenge_hash = vec![0x3A_u8; 32];
    let _: String = sqlx::query_scalar(
        "SELECT challenge_id::text FROM public.create_admin_login_challenge($1::uuid, $2)",
    )
    .bind(&admin_id)
    .bind(&credential_challenge_hash)
    .fetch_one(pool)
    .await
    .expect("active enrolled Admin should receive a challenge for scoped credential lookup");

    let credential_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.get_admin_login_totp_credentials($1)",
    )
    .bind(&credential_challenge_hash)
    .fetch_one(pool)
    .await
    .expect("active challenge should resolve only its Admin TOTP credential");
    assert_eq!(credential_count, 1);

    let identifier_hash = vec![0x31_u8; 32];
    let ip_hash = vec![0x32_u8; 32];
    for _ in 1..=5 {
        let allowed: bool = sqlx::query_scalar("SELECT public.reserve_admin_login_attempt($1, $2)")
            .bind(&identifier_hash)
            .bind(&ip_hash)
            .fetch_one(pool)
            .await
            .expect("Admin login attempt should be reserved");
        assert!(
            allowed,
            "the first five identifier attempts should be allowed"
        );
    }
    let blocked: bool = sqlx::query_scalar("SELECT public.reserve_admin_login_attempt($1, $2)")
        .bind(&identifier_hash)
        .bind(&ip_hash)
        .fetch_one(pool)
        .await
        .expect("Admin rate limit should return a decision");
    assert!(!blocked, "the sixth identifier attempt must be blocked");

    let current_totp_step: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()) / 30)::bigint")
            .fetch_one(pool)
            .await
            .expect("current TOTP time-step should be available");

    let failed_challenge_hash = vec![0x41_u8; 32];
    let _: String = sqlx::query_scalar(
        "SELECT challenge_id::text FROM public.create_admin_login_challenge($1::uuid, $2)",
    )
    .bind(&admin_id)
    .bind(&failed_challenge_hash)
    .fetch_one(pool)
    .await
    .expect("active Admin should receive a short-lived login challenge");
    for attempt in 1..=5 {
        let still_valid: bool =
            sqlx::query_scalar("SELECT public.record_admin_login_challenge_failure($1)")
                .bind(&failed_challenge_hash)
                .fetch_one(pool)
                .await
                .expect("challenge failure should be recorded");
        assert_eq!(still_valid, attempt < 5);
    }
    assert!(
        sqlx::query_scalar::<_, String>("SELECT public.complete_admin_login($1, $2, $3)::text",)
            .bind(&failed_challenge_hash)
            .bind(current_totp_step)
            .bind(vec![0x51_u8; 32])
            .fetch_one(pool)
            .await
            .is_err(),
        "a challenge must be unusable after five failed TOTP attempts"
    );

    let challenge_hash = vec![0x42_u8; 32];
    let (challenge_id, expires_at): (String, i64) = sqlx::query_as(
        "SELECT challenge_id::text, expires_at_unix \
           FROM public.create_admin_login_challenge($1::uuid, $2)",
    )
    .bind(&admin_id)
    .bind(&challenge_hash)
    .fetch_one(pool)
    .await
    .expect("fresh Admin challenge should be created");
    assert!(!challenge_id.is_empty());
    assert!(expires_at > 0);

    let challenge_credential: (String, Vec<u8>, Vec<u8>, String, Option<i64>) = sqlx::query_as(
        "SELECT user_id::text, secret_ciphertext, secret_nonce, \
                encryption_key_version, last_accepted_step \
           FROM public.get_admin_login_totp_credentials($1)",
    )
    .bind(&challenge_hash)
    .fetch_one(pool)
    .await
    .expect("valid Admin challenge should resolve only its active TOTP credential");
    assert_eq!(challenge_credential.0, admin_id);
    assert_eq!(challenge_credential.1.len(), 36);
    assert_eq!(challenge_credential.2.len(), 12);
    assert_eq!(challenge_credential.3, "v1");
    assert_eq!(challenge_credential.4, Some(101));

    let session_hash = vec![0x52_u8; 32];
    let session_id: String =
        sqlx::query_scalar("SELECT public.complete_admin_login($1, $2, $3)::text")
            .bind(&challenge_hash)
            .bind(current_totp_step)
            .bind(&session_hash)
            .fetch_one(pool)
            .await
            .expect("valid TOTP step should atomically consume challenge and create Admin session");
    assert!(!session_id.is_empty());

    let session_owner: Option<(String, String)> = sqlx::query_as(
        "SELECT session_id::text, user_id::text \
           FROM public.get_active_admin_session($1)",
    )
    .bind(&session_hash)
    .fetch_optional(pool)
    .await
    .expect("dedicated Admin session should be retrievable");
    assert_eq!(session_owner, Some((session_id.clone(), admin_id.clone())));

    assert!(
        sqlx::query_scalar::<_, String>("SELECT public.complete_admin_login($1, $2, $3)::text",)
            .bind(&challenge_hash)
            .bind(current_totp_step)
            .bind(vec![0x53_u8; 32])
            .fetch_one(pool)
            .await
            .is_err(),
        "a consumed challenge and TOTP step must not be reusable"
    );

    let replay_challenge_hash = vec![0x43_u8; 32];
    let _: String = sqlx::query_scalar(
        "SELECT challenge_id::text FROM public.create_admin_login_challenge($1::uuid, $2)",
    )
    .bind(&admin_id)
    .bind(&replay_challenge_hash)
    .fetch_one(pool)
    .await
    .expect("new challenge should be available after the prior challenge is consumed");
    assert!(
        sqlx::query_scalar::<_, String>("SELECT public.complete_admin_login($1, $2, $3)::text",)
            .bind(&replay_challenge_hash)
            .bind(current_totp_step)
            .bind(vec![0x54_u8; 32])
            .fetch_one(pool)
            .await
            .is_err(),
        "the same TOTP time-step must be rejected across distinct challenges"
    );

    let future_challenge_hash = vec![0x45_u8; 32];
    let _: String = sqlx::query_scalar(
        "SELECT challenge_id::text FROM public.create_admin_login_challenge($1::uuid, $2)",
    )
    .bind(&admin_id)
    .bind(&future_challenge_hash)
    .fetch_one(pool)
    .await
    .expect("fresh challenge should be created for time-window validation");
    assert!(
        sqlx::query_scalar::<_, String>("SELECT public.complete_admin_login($1, $2, $3)::text",)
            .bind(&future_challenge_hash)
            .bind(current_totp_step + 5)
            .bind(vec![0x55_u8; 32])
            .fetch_one(pool)
            .await
            .is_err(),
        "TOTP steps outside the accepted clock-drift window must be rejected"
    );

    let concurrent_challenge_hash = vec![0x44_u8; 32];
    let _: String = sqlx::query_scalar(
        "SELECT challenge_id::text FROM public.create_admin_login_challenge($1::uuid, $2)",
    )
    .bind(&admin_id)
    .bind(&concurrent_challenge_hash)
    .fetch_one(pool)
    .await
    .expect("concurrency challenge should be created");
    let first =
        sqlx::query_scalar::<_, String>("SELECT public.complete_admin_login($1, $2, $3)::text")
            .bind(&concurrent_challenge_hash)
            .bind(current_totp_step + 1)
            .bind(vec![0x61_u8; 32])
            .fetch_one(pool);
    let second =
        sqlx::query_scalar::<_, String>("SELECT public.complete_admin_login($1, $2, $3)::text")
            .bind(&concurrent_challenge_hash)
            .bind(current_totp_step + 1)
            .bind(vec![0x62_u8; 32])
            .fetch_one(pool);
    let (first_result, second_result) = tokio::join!(first, second);
    assert_ne!(
        first_result.is_ok(),
        second_result.is_ok(),
        "concurrent completion must issue exactly one Admin session"
    );

    let revoked: bool = sqlx::query_scalar("SELECT public.revoke_admin_session($1)")
        .bind(&session_hash)
        .fetch_one(pool)
        .await
        .expect("Admin logout should revoke the dedicated session");
    assert!(revoked);
    let inactive_session: Option<String> =
        sqlx::query_scalar("SELECT session_id::text FROM public.get_active_admin_session($1)")
            .bind(&session_hash)
            .fetch_optional(pool)
            .await
            .expect("revoked Admin session should no longer validate");
    assert!(inactive_session.is_none());

    let active_admin_session_hash: Vec<u8> = sqlx::query_scalar(
        "SELECT token_hash FROM public.admin_sessions \
          WHERE user_id = $1::uuid AND revoked_at IS NULL LIMIT 1",
    )
    .bind(&admin_id)
    .fetch_one(pool)
    .await
    .expect("concurrent login should leave one active Admin session");

    let pending_admin_challenge_hash = vec![0x46_u8; 32];
    let _: String = sqlx::query_scalar(
        "SELECT challenge_id::text FROM public.create_admin_login_challenge($1::uuid, $2)",
    )
    .bind(&admin_id)
    .bind(&pending_admin_challenge_hash)
    .fetch_one(pool)
    .await
    .expect("active Admin should be able to create a challenge before deactivation");

    sqlx::query("UPDATE public.users SET account_status = 'deactivated' WHERE id = $1::uuid")
        .bind(&admin_id)
        .execute(pool)
        .await
        .expect("Admin deactivation should revoke authentication state");

    let deactivation_reason: Option<String> = sqlx::query_scalar(
        "SELECT revocation_reason FROM public.admin_sessions WHERE token_hash = $1",
    )
    .bind(&active_admin_session_hash)
    .fetch_one(pool)
    .await
    .expect("Admin session should be revoked after account deactivation");
    assert_eq!(
        deactivation_reason.as_deref(),
        Some("account_status_changed")
    );

    let deactivated_challenge: bool = sqlx::query_scalar(
        "SELECT consumed_at IS NOT NULL FROM public.admin_login_challenges WHERE challenge_hash = $1",
    )
    .bind(&pending_admin_challenge_hash)
    .fetch_one(pool)
    .await
    .expect("pending challenge should be consumed after account deactivation");
    assert!(deactivated_challenge);

    let deactivated_totp_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.get_admin_login_totp_credentials($1)",
    )
    .bind(vec![0x48_u8; 32])
    .fetch_one(pool)
    .await
    .expect("inactive Admin must not retrieve TOTP credentials without an active challenge");
    assert_eq!(deactivated_totp_count, 0);

    assert!(
        sqlx::query_scalar::<_, String>(
            "SELECT challenge_id::text FROM public.create_admin_login_challenge($1::uuid, $2)",
        )
        .bind(&admin_id)
        .bind(vec![0x47_u8; 32])
        .fetch_one(pool)
        .await
        .is_err(),
        "inactive Admin must not receive a login challenge"
    );

    sqlx::query("UPDATE public.users SET account_status = 'active' WHERE id = $1::uuid")
        .bind(&admin_id)
        .execute(pool)
        .await
        .expect("Admin account may be reactivated after deactivation");

    let resumed_session: Option<String> =
        sqlx::query_scalar("SELECT session_id::text FROM public.get_active_admin_session($1)")
            .bind(&active_admin_session_hash)
            .fetch_optional(pool)
            .await
            .expect("revoked Admin session must remain invalid after account reactivation");
    assert!(
        resumed_session.is_none(),
        "reactivating the account must not revive a revoked Admin session"
    );

    let deactivation_audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.identity_audit_events \
          WHERE actor_type = 'system' AND subject_user_id = $1::uuid \
            AND action_code = 'identity.admin.access.revoked' \
            AND reason_code = 'account_status_changed'",
    )
    .bind(&admin_id)
    .fetch_one(pool)
    .await
    .expect("Admin deactivation audit event should be recorded");
    assert_eq!(deactivation_audit_count, 1);

    let admin_audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.identity_audit_events \
          WHERE actor_type = 'admin' AND actor_id = $1::uuid \
            AND action_code = 'identity.admin.login.succeeded' \
            AND outcome = 'success'",
    )
    .bind(&admin_id)
    .fetch_one(pool)
    .await
    .expect("successful Admin login audit event should be recorded");
    assert!(admin_audit_count >= 2);

    let dedicated_session_count: i64 = sqlx::query_scalar(
        "SELECT count(*)::bigint FROM public.admin_sessions WHERE user_id = $1::uuid",
    )
    .bind(&admin_id)
    .fetch_one(pool)
    .await
    .expect("dedicated Admin session table should be inspectable by migrator");
    assert!(dedicated_session_count >= 2);
}

async fn assert_identity_abuse_rate_limits(pool: &PgPool) {
    let fingerprint = vec![0x5a_u8; 32];
    sqlx::query(
        "DELETE FROM public.identity_abuse_rate_limits \
         WHERE scope = 'registration:create:ip' AND key_hash = $1",
    )
    .bind(&fingerprint)
    .execute(pool)
    .await
    .expect("test rate-limit fingerprint should be clearable");

    for expected in [true, true, false] {
        let allowed: bool = sqlx::query_scalar(
            "SELECT public.reserve_identity_abuse_attempt(\
                'registration:create:ip', $1, 2, 900\
             )",
        )
        .bind(&fingerprint)
        .fetch_one(pool)
        .await
        .expect("Identity rate-limit reservation should succeed");
        assert_eq!(allowed, expected);
    }

    let attempts: i32 = sqlx::query_scalar(
        "SELECT attempt_count FROM public.identity_abuse_rate_limits \
         WHERE scope = 'registration:create:ip' AND key_hash = $1",
    )
    .bind(&fingerprint)
    .fetch_one(pool)
    .await
    .expect("rate-limit counter should be persisted");
    assert_eq!(attempts, 3);

    let invalid_hash = sqlx::query_scalar::<_, bool>(
        "SELECT public.reserve_identity_abuse_attempt(\
            'registration:create:ip', $1, 2, 900\
         )",
    )
    .bind(vec![0x01_u8; 31])
    .fetch_one(pool)
    .await;
    assert!(
        invalid_hash.is_err(),
        "database function must reject fingerprints that are not 32 bytes"
    );

    sqlx::query(
        "DELETE FROM public.identity_abuse_rate_limits \
         WHERE scope = 'registration:create:ip' AND key_hash = $1",
    )
    .bind(&fingerprint)
    .execute(pool)
    .await
    .expect("test rate-limit fingerprint should be removed");
}
