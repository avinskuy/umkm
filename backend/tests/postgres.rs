use sqlx::PgPool;
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
    assert_provider_identity_invariants(&pool).await;
    assert_client_creation_function(&pool).await;
    assert_verification_schema_invariants(&pool).await;

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
        "client accounts without a password must have a Google identity in the same transaction"
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

async fn assert_provider_identity_invariants(pool: &PgPool) {
    let google_user_id: String =
        sqlx::query_scalar("SELECT public.create_client_user($1, $2, $3, $4, $5, $6)::text")
            .bind("business_owner")
            .bind("email")
            .bind("google-owner@example.test")
            .bind(Option::<String>::None)
            .bind(Option::<String>::None)
            .bind("google-subject-001")
            .fetch_one(pool)
            .await
            .expect("Google-only registration should create user and provider identity atomically");

    let local_user_id: String =
        sqlx::query_scalar("SELECT public.create_client_user($1, $2, $3, $4, $5, $6)::text")
            .bind("investor")
            .bind("email")
            .bind("local-investor@example.test")
            .bind(Option::<String>::None)
            .bind("encoded-password-hash")
            .bind(Option::<String>::None)
            .fetch_one(pool)
            .await
            .expect("local-password registration should create a pending client account");

    assert!(
        sqlx::query(
            "INSERT INTO public.user_identities (user_id, provider, provider_subject) \
             VALUES ($1::uuid, 'google', 'google-subject-001')"
        )
        .bind(&local_user_id)
        .execute(pool)
        .await
        .is_err(),
        "a Google provider subject cannot be linked to two accounts"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.user_identities (user_id, provider, provider_subject) \
             VALUES ($1::uuid, 'unknown-provider', 'provider-subject-002')"
        )
        .bind(&local_user_id)
        .execute(pool)
        .await
        .is_err(),
        "unsupported providers must be rejected"
    );

    assert!(
        sqlx::query(
            "INSERT INTO public.user_identities (user_id, provider, provider_subject) \
             VALUES ('00000000-0000-0000-0000-000000000000', 'google', 'provider-subject-003')"
        )
        .execute(pool)
        .await
        .is_err(),
        "provider identities must reference an existing user"
    );

    assert!(
        sqlx::query(
            "UPDATE public.user_identities SET provider_subject = 'changed-subject' WHERE user_id = $1::uuid"
        )
        .bind(&google_user_id)
        .execute(pool)
        .await
        .is_err(),
        "provider identity binding must be immutable"
    );

    assert!(
        sqlx::query("DELETE FROM public.user_identities WHERE user_id = $1::uuid")
            .bind(&google_user_id)
            .execute(pool)
            .await
            .is_err(),
        "a Google-only client cannot unlink its last authentication method"
    );

    let mut setup_tx = pool
        .begin()
        .await
        .expect("transaction should start for deletion lifecycle fixture");

    let terminal_user_id: String = sqlx::query_scalar(
        "INSERT INTO public.users ( \
             primary_role, account_status, email_normalized, initial_contact_channel, \
             email_verified_at, initial_contact_verified_at, status_changed_at \
         ) \
         VALUES ( \
             'business_owner', 'deletion_pending', 'terminal-google@example.test', 'email', \
             now(), now(), now() - INTERVAL '31 days' \
         ) \
         RETURNING id::text",
    )
    .fetch_one(&mut *setup_tx)
    .await
    .expect("a deletion-pending account with a past waiting period should be created");

    sqlx::query(
        "INSERT INTO public.user_identities (user_id, provider, provider_subject) \
         VALUES ($1::uuid, 'google', 'google-subject-terminal')",
    )
    .bind(&terminal_user_id)
    .execute(&mut *setup_tx)
    .await
    .expect("terminal fixture should retain its Google identity before deletion completes");

    setup_tx
        .commit()
        .await
        .expect("deletion-pending Google-only account should satisfy auth method invariant");

    let mut processing_tx = pool
        .begin()
        .await
        .expect("transaction should start for deletion processing");

    sqlx::query(
        "UPDATE public.users SET account_status = 'deletion_processing' WHERE id = $1::uuid",
    )
    .bind(&terminal_user_id)
    .execute(&mut *processing_tx)
    .await
    .expect("deletion may enter processing after the waiting period");

    processing_tx
        .commit()
        .await
        .expect("deletion processing transition should commit");

    assert!(
        sqlx::query("UPDATE public.users SET account_status = 'active' WHERE id = $1::uuid")
            .bind(&terminal_user_id)
            .execute(pool)
            .await
            .is_err(),
        "deletion processing cannot be cancelled after it starts"
    );

    let mut tx = pool
        .begin()
        .await
        .expect("transaction should start for terminal account cleanup");

    sqlx::query(
        "UPDATE public.users \
         SET account_status = 'closed', \
             email_normalized = NULL, \
             email_verified_at = NULL, \
             phone_e164 = NULL, \
             phone_verified_at = NULL, \
             initial_contact_verified_at = NULL, \
             password_hash = NULL, \
             pending_expires_at = NULL \
         WHERE id = $1::uuid",
    )
    .bind(&terminal_user_id)
    .execute(&mut *tx)
    .await
    .expect("completed deletion may clear personal account fields");

    sqlx::query("DELETE FROM public.user_identities WHERE user_id = $1::uuid")
        .bind(&terminal_user_id)
        .execute(&mut *tx)
        .await
        .expect("completed deletion may release provider identity");

    tx.commit()
        .await
        .expect("terminal account must not retain an external identity");

    assert!(
        sqlx::query(
            "UPDATE public.users \
             SET account_status = 'pending_verification', \
                 email_normalized = 'reopened-terminal@example.test', \
                 password_hash = 'new-hash', \
                 pending_expires_at = now() + INTERVAL '30 minutes' \
             WHERE id = $1::uuid"
        )
        .bind(&terminal_user_id)
        .execute(pool)
        .await
        .is_err(),
        "closed accounts cannot be reopened"
    );
}

async fn assert_client_creation_function(pool: &PgPool) {
    assert!(
        sqlx::query_scalar::<_, String>(
            "SELECT public.create_client_user($1, $2, $3, $4, $5, $6)::text",
        )
        .bind("admin")
        .bind("email")
        .bind("not-allowed-admin@example.test")
        .bind(Option::<String>::None)
        .bind("encoded-password-hash")
        .bind(Option::<String>::None)
        .fetch_one(pool)
        .await
        .is_err(),
        "public client creation function must reject Admin role"
    );

    assert!(
        sqlx::query_scalar::<_, String>(
            "SELECT public.create_client_user($1, $2, $3, $4, $5, $6)::text",
        )
        .bind("business_owner")
        .bind("email")
        .bind("missing-auth-method@example.test")
        .bind(Option::<String>::None)
        .bind(Option::<String>::None)
        .bind(Option::<String>::None)
        .fetch_one(pool)
        .await
        .is_err(),
        "client account creation requires exactly one initial authentication method"
    );

    assert!(
        sqlx::query_scalar::<_, String>(
            "SELECT public.create_client_user($1, $2, $3, $4, $5, $6)::text",
        )
        .bind("business_owner")
        .bind("email")
        .bind("both-auth-methods@example.test")
        .bind(Option::<String>::None)
        .bind("encoded-password-hash")
        .bind("google-subject-both-methods")
        .fetch_one(pool)
        .await
        .is_err(),
        "client account creation cannot silently mix initial authentication methods"
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
