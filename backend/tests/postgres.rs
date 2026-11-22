use sqlx::{postgres::PgPoolOptions, raw_sql, PgPool};
use usahapintar_api::database::{connect, migrations::run, DatabaseConfig};

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
    assert_verification_schema_invariants(&pool).await;
    assert_authentication_session_invariants(&pool).await;
    assert_account_recovery_schema_invariants(&pool).await;
    assert_identity_audit_event_invariants(&pool).await;
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

async fn assert_account_recovery_schema_invariants(pool: &PgPool) {
    let first_user_id = insert_active_email(pool, "recovery-contact-first@example.test").await;
    let second_user_id = insert_active_email(pool, "recovery-contact-second@example.test").await;

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

    sqlx::query("UPDATE public.account_recovery_cases SET status = 'approved' WHERE id = $1::uuid")
        .bind(&recovery_case_id)
        .execute(pool)
        .await
        .expect("reviewed recovery case may be approved");

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
                   'recovery_contacts', 'account_recovery_cases', 'password_reset_tokens' \
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

    let function_access: (bool, bool, bool) = sqlx::query_as(
        "SELECT has_function_privilege(current_user, \
                    'public.create_client_user(text,text,text,text,text)', 'EXECUTE'), \
                has_function_privilege(current_user, \
                    'public.is_safe_identity_audit_metadata(jsonb)', 'EXECUTE'), \
                has_function_privilege(current_user, \
                    'public.guard_identity_audit_event()', 'EXECUTE')",
    )
    .fetch_one(&app_pool)
    .await
    .expect("function privileges should be queryable");
    assert_eq!(function_access, (true, true, false));

    let created_user_id: String =
        sqlx::query_scalar("SELECT public.create_client_user($1, $2, $3, $4, $5)::text")
            .bind("business_owner")
            .bind("email")
            .bind("principal-bootstrap@example.test")
            .bind(Option::<String>::None)
            .bind("test-only-encoded-password-hash")
            .fetch_one(&app_pool)
            .await
            .expect("application may create a pending account through the guarded function");
    assert!(!created_user_id.is_empty());

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

    app_pool.close().await;
    admin_pool.close().await;
}
