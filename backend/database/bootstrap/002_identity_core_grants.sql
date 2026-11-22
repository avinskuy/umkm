-- Jalankan setelah migration Identity Core 0001–0007 selesai.
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

-- Tidak ada akses langsung aplikasi ke tabel akun, kredensial, sesi, atau recovery.
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM usahapintar_app;
REVOKE ALL ON ALL SEQUENCES IN SCHEMA public FROM usahapintar_app;
REVOKE EXECUTE ON ALL FUNCTIONS IN SCHEMA public FROM usahapintar_app;

-- Pengecualian sempit: pembuatan akun melalui fungsi terkontrol dan penulisan audit.
GRANT EXECUTE ON FUNCTION public.create_client_user(TEXT, TEXT, TEXT, TEXT, TEXT)
    TO usahapintar_app;
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
