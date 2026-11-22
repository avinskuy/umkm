-- Jalankan setelah migration Identity Core 0001–0015 selesai.
-- Fail closed jika SECURITY DEFINER registration function bukan milik migrator non-superuser.

DO $ownership_guard$
DECLARE
    v_function_owner TEXT;
    v_users_owner TEXT;
    v_audit_owner TEXT;
    v_database_owner TEXT;
    v_owner_is_privileged BOOLEAN;
    v_is_security_definer BOOLEAN;
    v_has_safe_search_path BOOLEAN;
BEGIN
    SELECT pg_catalog.pg_get_userbyid(p.proowner),
           p.prosecdef,
           COALESCE(p.proconfig @> ARRAY['search_path=pg_catalog']::TEXT[], FALSE)
      INTO v_function_owner, v_is_security_definer, v_has_safe_search_path
      FROM pg_catalog.pg_proc AS p
     WHERE p.oid = pg_catalog.to_regprocedure(
         'public.create_client_user(text,text,text,text,text)'
     );

    IF v_function_owner IS DISTINCT FROM 'usahapintar_migrator'
       OR v_is_security_definer IS DISTINCT FROM TRUE
       OR v_has_safe_search_path IS DISTINCT FROM TRUE THEN
        RAISE EXCEPTION
            'Refusing application grants: create_client_user must be SECURITY DEFINER owned by usahapintar_migrator';
    END IF;

    SELECT pg_catalog.pg_get_userbyid(c.relowner)
      INTO v_users_owner
      FROM pg_catalog.pg_class AS c
     WHERE c.oid = 'public.users'::pg_catalog.regclass;

    SELECT pg_catalog.pg_get_userbyid(c.relowner)
      INTO v_audit_owner
      FROM pg_catalog.pg_class AS c
     WHERE c.oid = 'public.identity_audit_events'::pg_catalog.regclass;

    IF v_users_owner IS DISTINCT FROM 'usahapintar_migrator'
       OR v_audit_owner IS DISTINCT FROM 'usahapintar_migrator' THEN
        RAISE EXCEPTION
            'Refusing application grants: users and audit tables must be owned by usahapintar_migrator';
    END IF;

    SELECT r.rolsuper OR r.rolbypassrls OR r.rolcreatedb OR r.rolcreaterole
      INTO v_owner_is_privileged
      FROM pg_catalog.pg_roles AS r
     WHERE r.rolname = v_function_owner;

    IF v_owner_is_privileged IS DISTINCT FROM FALSE THEN
        RAISE EXCEPTION
            'Refusing SECURITY DEFINER grant: migration owner has elevated cluster privileges';
    END IF;

    SELECT pg_catalog.pg_get_userbyid(d.datdba)
      INTO v_database_owner
      FROM pg_catalog.pg_database AS d
     WHERE d.datname = pg_catalog.current_database();

    IF v_database_owner IS NOT DISTINCT FROM 'usahapintar_migrator' THEN
        RAISE EXCEPTION
            'Refusing SECURITY DEFINER grant: migrator must not own the database';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_auth_members AS m
          JOIN pg_catalog.pg_roles AS r ON r.oid = m.member
         WHERE r.rolname = 'usahapintar_migrator'
    ) THEN
        RAISE EXCEPTION
            'Refusing SECURITY DEFINER grant: migration owner must not inherit or assume other roles';
    END IF;
END;
$ownership_guard$;

DO $profile_policy_ownership_guard$
DECLARE
    v_owner TEXT;
    v_is_security_definer BOOLEAN;
    v_has_safe_search_path BOOLEAN;
BEGIN
    SELECT pg_catalog.pg_get_userbyid(p.proowner),
           p.prosecdef,
           COALESCE(p.proconfig @> ARRAY['search_path=pg_catalog']::TEXT[], FALSE)
      INTO v_owner, v_is_security_definer, v_has_safe_search_path
      FROM pg_catalog.pg_proc AS p
     WHERE p.oid = pg_catalog.to_regprocedure(
         'public.create_client_user_with_profile_and_policy(text,text,text,text,text,text,text,text,boolean,text)'
     );

    IF v_owner IS DISTINCT FROM 'usahapintar_migrator'
       OR v_is_security_definer IS DISTINCT FROM TRUE
       OR v_has_safe_search_path IS DISTINCT FROM TRUE THEN
        RAISE EXCEPTION
            'Refusing application grants: profile/policy registration function must be SECURITY DEFINER owned by usahapintar_migrator';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM pg_catalog.pg_class AS c
         WHERE c.oid IN (
             'public.user_profiles'::pg_catalog.regclass,
             'public.policy_acceptance_records'::pg_catalog.regclass,
             'public.consent_events'::pg_catalog.regclass
         )
           AND pg_catalog.pg_get_userbyid(c.relowner) <> 'usahapintar_migrator'
    ) THEN
        RAISE EXCEPTION
            'Refusing application grants: profile and policy history tables must be owned by usahapintar_migrator';
    END IF;
END;
$profile_policy_ownership_guard$;

-- Tidak ada akses langsung aplikasi ke tabel akun, kredensial, sesi, atau recovery.
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM usahapintar_app;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM usahapintar_app;
REVOKE EXECUTE ON ALL FUNCTIONS IN SCHEMA public FROM usahapintar_app;

-- Pengecualian sempit: pembuatan akun melalui fungsi terkontrol dan penulisan audit.
REVOKE EXECUTE ON FUNCTION public.create_client_user(TEXT, TEXT, TEXT, TEXT, TEXT)
    FROM usahapintar_app;
GRANT EXECUTE ON FUNCTION public.create_client_user_with_profile_and_policy(
    TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, BOOLEAN, TEXT
) TO usahapintar_app;
GRANT INSERT ON TABLE public.identity_audit_events TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.is_safe_identity_audit_metadata(JSONB)
    TO usahapintar_app;

-- Pembacaan audit dipisahkan dari principal aplikasi. Role ini NOLOGIN dan
-- belum diberikan kepada principal Admin mana pun.
GRANT USAGE ON SCHEMA public TO usahapintar_audit_reader;
GRANT SELECT ON TABLE public.identity_audit_events TO usahapintar_audit_reader;


-- verification function ownership --
DO $verification_ownership_guard$
DECLARE
    v_signature TEXT;
    v_owner TEXT;
    v_is_security_definer BOOLEAN;
    v_has_safe_search_path BOOLEAN;
BEGIN
    FOREACH v_signature IN ARRAY ARRAY[
        'public.create_registration_verification_session(uuid,uuid,text,bytea,bytea,text,bytea,text,timestamp with time zone)',
        'public.get_registration_verification_session(uuid)',
        'public.get_pending_otp_challenge(uuid)',
        'public.reserve_otp_challenge(uuid,uuid,bytea,text,bytea[])',
        'public.record_otp_delivery_result(uuid,text)',
        'public.record_otp_verification_attempt(uuid,uuid,boolean,bytea[])'
    ] LOOP
        SELECT pg_catalog.pg_get_userbyid(p.proowner),
               p.prosecdef,
               COALESCE(p.proconfig @> ARRAY['search_path=pg_catalog']::TEXT[], FALSE)
          INTO v_owner, v_is_security_definer, v_has_safe_search_path
          FROM pg_catalog.pg_proc AS p
         WHERE p.oid = pg_catalog.to_regprocedure(v_signature);

        IF v_owner IS DISTINCT FROM 'usahapintar_migrator'
           OR v_is_security_definer IS DISTINCT FROM TRUE
           OR v_has_safe_search_path IS DISTINCT FROM TRUE THEN
            RAISE EXCEPTION
                'Refusing verification grants: % must be SECURITY DEFINER owned by usahapintar_migrator with safe search_path',
                v_signature;
        END IF;
    END LOOP;
END;
$verification_ownership_guard$;

-- verification function grants --
GRANT EXECUTE ON FUNCTION public.create_registration_verification_session(UUID, UUID, TEXT, BYTEA, BYTEA, TEXT, BYTEA, TEXT, TIMESTAMPTZ)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.get_registration_verification_session(UUID)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.get_pending_otp_challenge(UUID)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.reserve_otp_challenge(UUID, UUID, BYTEA, TEXT, BYTEA[])
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.record_otp_delivery_result(UUID, TEXT)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.record_otp_verification_attempt(UUID, UUID, BOOLEAN, BYTEA[])
    TO usahapintar_app;


-- Password authentication and session operations.
DO $password_authentication_ownership_guard$
DECLARE
    v_signature TEXT;
    v_owner TEXT;
    v_is_security_definer BOOLEAN;
    v_has_safe_search_path BOOLEAN;
BEGIN
    FOREACH v_signature IN ARRAY ARRAY[
        'public.find_active_password_login(text,text)',
        'public.create_password_login_session(uuid,bytea)',
        'public.revoke_password_login_session(bytea)',
        'public.reserve_password_login_attempt(bytea,bytea)',
        'public.clear_password_login_identifier_limit(bytea)'
    ] LOOP
        SELECT pg_catalog.pg_get_userbyid(p.proowner),
               p.prosecdef,
               COALESCE(p.proconfig @> ARRAY['search_path=pg_catalog']::TEXT[], FALSE)
          INTO v_owner, v_is_security_definer, v_has_safe_search_path
          FROM pg_catalog.pg_proc AS p
         WHERE p.oid = pg_catalog.to_regprocedure(v_signature);

        IF v_owner IS DISTINCT FROM 'usahapintar_migrator'
           OR v_is_security_definer IS DISTINCT FROM TRUE
           OR v_has_safe_search_path IS DISTINCT FROM TRUE THEN
            RAISE EXCEPTION
                'Refusing password authentication grants: % must be SECURITY DEFINER owned by usahapintar_migrator with safe search_path',
                v_signature;
        END IF;
    END LOOP;

    IF pg_catalog.pg_get_userbyid(
        (SELECT c.relowner FROM pg_catalog.pg_class AS c
          WHERE c.oid = 'public.password_login_rate_limits'::pg_catalog.regclass)
    ) IS DISTINCT FROM 'usahapintar_migrator' THEN
        RAISE EXCEPTION
            'Refusing password authentication grants: rate-limit table must be owned by usahapintar_migrator';
    END IF;
END;
$password_authentication_ownership_guard$;

REVOKE ALL ON TABLE public.password_login_rate_limits FROM usahapintar_app;
GRANT EXECUTE ON FUNCTION public.find_active_password_login(TEXT, TEXT)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.create_password_login_session(UUID, BYTEA)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.revoke_password_login_session(BYTEA)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.reserve_password_login_attempt(BYTEA, BYTEA)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.clear_password_login_identifier_limit(BYTEA)
    TO usahapintar_app;


-- Session validation, token rotation and logout-all operations.
DO $session_runtime_ownership_guard$
DECLARE
    v_signature TEXT;
    v_owner TEXT;
    v_is_security_definer BOOLEAN;
    v_has_safe_search_path BOOLEAN;
BEGIN
    FOREACH v_signature IN ARRAY ARRAY[
        'public.get_active_password_login_session(bytea)',
        'public.rotate_active_password_login_session(bytea,bytea)',
        'public.revoke_all_password_login_sessions(bytea)'
    ] LOOP
        SELECT pg_catalog.pg_get_userbyid(p.proowner),
               p.prosecdef,
               COALESCE(p.proconfig @> ARRAY['search_path=pg_catalog']::TEXT[], FALSE)
          INTO v_owner, v_is_security_definer, v_has_safe_search_path
          FROM pg_catalog.pg_proc AS p
         WHERE p.oid = pg_catalog.to_regprocedure(v_signature);

        IF v_owner IS DISTINCT FROM 'usahapintar_migrator'
           OR v_is_security_definer IS DISTINCT FROM TRUE
           OR v_has_safe_search_path IS DISTINCT FROM TRUE THEN
            RAISE EXCEPTION
                'Refusing session runtime grants: % must be SECURITY DEFINER owned by usahapintar_migrator with safe search_path',
                v_signature;
        END IF;
    END LOOP;
END;
$session_runtime_ownership_guard$;

GRANT EXECUTE ON FUNCTION public.get_active_password_login_session(BYTEA)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.rotate_active_password_login_session(BYTEA, BYTEA)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.revoke_all_password_login_sessions(BYTEA)
    TO usahapintar_app;

-- Account recovery verification and account selection.
DO $account_recovery_ownership_guard$
DECLARE
    v_signature TEXT;
    v_owner TEXT;
    v_is_security_definer BOOLEAN;
    v_has_safe_search_path BOOLEAN;
BEGIN
    FOREACH v_signature IN ARRAY ARRAY[
        'public.create_account_recovery_verification_session(uuid,uuid,text,text,bytea,bytea,text,bytea,text,timestamp with time zone)',
        'public.get_account_recovery_verification_session(uuid)',
        'public.bind_account_recovery_session_to_account(uuid,uuid,bytea[])'
    ] LOOP
        SELECT pg_catalog.pg_get_userbyid(p.proowner),
               p.prosecdef,
               COALESCE(p.proconfig @> ARRAY['search_path=pg_catalog']::TEXT[], FALSE)
          INTO v_owner, v_is_security_definer, v_has_safe_search_path
          FROM pg_catalog.pg_proc AS p
         WHERE p.oid = pg_catalog.to_regprocedure(v_signature);

        IF v_owner IS DISTINCT FROM 'usahapintar_migrator'
           OR v_is_security_definer IS DISTINCT FROM TRUE
           OR v_has_safe_search_path IS DISTINCT FROM TRUE THEN
            RAISE EXCEPTION
                'Refusing account recovery grants: % must be SECURITY DEFINER owned by usahapintar_migrator with safe search_path',
                v_signature;
        END IF;
    END LOOP;
END;
$account_recovery_ownership_guard$;

GRANT EXECUTE ON FUNCTION public.create_account_recovery_verification_session(UUID, UUID, TEXT, TEXT, BYTEA, BYTEA, TEXT, BYTEA, TEXT, TIMESTAMPTZ)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.get_account_recovery_verification_session(UUID)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.bind_account_recovery_session_to_account(UUID, UUID, BYTEA[])
    TO usahapintar_app;

-- Recovery account candidate discovery.
DO $account_recovery_candidate_ownership_guard$
DECLARE
    v_signature TEXT;
    v_owner TEXT;
    v_is_security_definer BOOLEAN;
    v_has_safe_search_path BOOLEAN;
BEGIN
    FOREACH v_signature IN ARRAY ARRAY[
        'public.get_account_recovery_selection_context(uuid)',
        'public.list_account_recovery_candidates(uuid,bytea[])'
    ] LOOP
        SELECT pg_catalog.pg_get_userbyid(p.proowner),
               p.prosecdef,
               COALESCE(p.proconfig @> ARRAY['search_path=pg_catalog']::TEXT[], FALSE)
          INTO v_owner, v_is_security_definer, v_has_safe_search_path
          FROM pg_catalog.pg_proc AS p
         WHERE p.oid = pg_catalog.to_regprocedure(v_signature);

        IF v_owner IS DISTINCT FROM 'usahapintar_migrator'
           OR v_is_security_definer IS DISTINCT FROM TRUE
           OR v_has_safe_search_path IS DISTINCT FROM TRUE THEN
            RAISE EXCEPTION
                'Refusing recovery candidate grants: % must be SECURITY DEFINER owned by usahapintar_migrator with safe search_path',
                v_signature;
        END IF;
    END LOOP;
END;
$account_recovery_candidate_ownership_guard$;

GRANT EXECUTE ON FUNCTION public.get_account_recovery_selection_context(UUID)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.list_account_recovery_candidates(UUID, BYTEA[])
    TO usahapintar_app;

-- Account recovery review and prior-session signal operations.
DO $account_recovery_review_ownership_guard$
DECLARE
    v_signature TEXT;
    v_owner TEXT;
    v_is_security_definer BOOLEAN;
    v_has_safe_search_path BOOLEAN;
BEGIN
    FOREACH v_signature IN ARRAY ARRAY[
        'public.record_account_recovery_session_signal(uuid,bytea)',
        'public.review_account_recovery_case(uuid,uuid,bytea,text,text)'
    ] LOOP
        SELECT pg_catalog.pg_get_userbyid(p.proowner),
               p.prosecdef,
               COALESCE(p.proconfig @> ARRAY['search_path=pg_catalog']::TEXT[], FALSE)
          INTO v_owner, v_is_security_definer, v_has_safe_search_path
          FROM pg_catalog.pg_proc AS p
         WHERE p.oid = pg_catalog.to_regprocedure(v_signature);

        IF v_owner IS DISTINCT FROM 'usahapintar_migrator'
           OR v_is_security_definer IS DISTINCT FROM TRUE
           OR v_has_safe_search_path IS DISTINCT FROM TRUE THEN
            RAISE EXCEPTION
                'Refusing account recovery review grants: % must be SECURITY DEFINER owned by usahapintar_migrator with safe search_path',
                v_signature;
        END IF;
    END LOOP;
END;
$account_recovery_review_ownership_guard$;

GRANT EXECUTE ON FUNCTION public.record_account_recovery_session_signal(UUID, BYTEA)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.review_account_recovery_case(UUID, UUID, BYTEA, TEXT, TEXT)
    TO usahapintar_app;

-- Password reset issuance and completion operations.
DO $password_reset_ownership_guard$
DECLARE
    v_signature TEXT;
    v_owner TEXT;
    v_is_security_definer BOOLEAN;
    v_has_safe_search_path BOOLEAN;
BEGIN
    FOREACH v_signature IN ARRAY ARRAY[
        'public.get_approved_account_recovery_delivery_context(uuid)',
        'public.create_password_reset_token(uuid,bytea,bytea[])',
        'public.revoke_password_reset_token(bytea)',
        'public.complete_password_reset(bytea,text)'
    ] LOOP
        SELECT pg_catalog.pg_get_userbyid(p.proowner),
               p.prosecdef,
               COALESCE(p.proconfig @> ARRAY['search_path=pg_catalog']::TEXT[], FALSE)
          INTO v_owner, v_is_security_definer, v_has_safe_search_path
          FROM pg_catalog.pg_proc AS p
         WHERE p.oid = pg_catalog.to_regprocedure(v_signature);

        IF v_owner IS DISTINCT FROM 'usahapintar_migrator'
           OR v_is_security_definer IS DISTINCT FROM TRUE
           OR v_has_safe_search_path IS DISTINCT FROM TRUE THEN
            RAISE EXCEPTION
                'Refusing password reset grants: % must be SECURITY DEFINER owned by usahapintar_migrator with safe search_path',
                v_signature;
        END IF;
    END LOOP;
END;
$password_reset_ownership_guard$;

GRANT EXECUTE ON FUNCTION public.get_approved_account_recovery_delivery_context(UUID)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.create_password_reset_token(UUID, BYTEA, BYTEA[])
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.revoke_password_reset_token(BYTEA)
    TO usahapintar_app;
GRANT EXECUTE ON FUNCTION public.complete_password_reset(BYTEA, TEXT)
    TO usahapintar_app;
