-- Authentication runtime operations for password login and opaque sessions.
-- The application receives only narrowly scoped SECURITY DEFINER operations.

CREATE TABLE public.password_login_rate_limits (
    scope TEXT NOT NULL,
    key_hash BYTEA NOT NULL,
    window_started_at TIMESTAMPTZ NOT NULL,
    attempt_count INTEGER NOT NULL,
    blocked_until TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT password_login_rate_limits_pk PRIMARY KEY (scope, key_hash),
    CONSTRAINT password_login_rate_limits_scope_check
        CHECK (scope IN ('identifier', 'ip')),
    CONSTRAINT password_login_rate_limits_key_hash_check
        CHECK (octet_length(key_hash) = 32),
    CONSTRAINT password_login_rate_limits_attempt_count_check
        CHECK (attempt_count > 0),
    CONSTRAINT password_login_rate_limits_block_check
        CHECK (blocked_until IS NULL OR blocked_until > window_started_at)
);

CREATE INDEX password_login_rate_limits_updated_idx
    ON public.password_login_rate_limits (updated_at);

CREATE FUNCTION public.find_active_password_login(
    p_identifier TEXT,
    p_expected_role TEXT
)
RETURNS TABLE (user_id UUID, password_hash TEXT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    IF p_identifier IS NULL OR length(btrim(p_identifier)) = 0
       OR p_expected_role NOT IN ('business_owner', 'investor') THEN
        RETURN;
    END IF;

    RETURN QUERY
    SELECT u.id, u.password_hash
      FROM public.users AS u
     WHERE u.account_status = 'active'
       AND u.primary_role = p_expected_role
       AND u.password_hash IS NOT NULL
       AND (
           u.email_normalized = p_identifier
           OR u.phone_e164 = p_identifier
       )
     LIMIT 1;
END;
$$;

CREATE FUNCTION public.create_password_login_session(
    p_user_id UUID,
    p_token_hash BYTEA
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_session_id UUID;
BEGIN
    IF p_user_id IS NULL OR p_token_hash IS NULL OR octet_length(p_token_hash) <> 32 THEN
        RAISE EXCEPTION 'password login session input is invalid'
            USING ERRCODE = '22023';
    END IF;

    INSERT INTO public.sessions (user_id, token_hash, remember_device)
    SELECT u.id, p_token_hash, FALSE
      FROM public.users AS u
     WHERE u.id = p_user_id
       AND u.account_status = 'active'
       AND u.password_hash IS NOT NULL
    RETURNING id INTO v_session_id;

    IF v_session_id IS NULL THEN
        RAISE EXCEPTION 'active password account was not found'
            USING ERRCODE = 'P0002';
    END IF;

    RETURN v_session_id;
END;
$$;

CREATE FUNCTION public.revoke_password_login_session(
    p_token_hash BYTEA
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    IF p_token_hash IS NULL OR octet_length(p_token_hash) <> 32 THEN
        RETURN FALSE;
    END IF;

    UPDATE public.sessions
       SET revoked_at = clock_timestamp(),
           revocation_reason = 'logout'
     WHERE token_hash = p_token_hash
       AND revoked_at IS NULL
       AND expires_at > clock_timestamp()
       AND last_seen_at > clock_timestamp() - INTERVAL '7 days';

    RETURN FOUND;
END;
$$;

CREATE FUNCTION public.reserve_password_login_attempt(
    p_identifier_hash BYTEA,
    p_ip_hash BYTEA
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_now TIMESTAMPTZ := clock_timestamp();
    v_identifier_count INTEGER;
    v_identifier_block TIMESTAMPTZ;
    v_ip_count INTEGER;
    v_ip_block TIMESTAMPTZ;
BEGIN
    IF p_identifier_hash IS NULL OR octet_length(p_identifier_hash) <> 32
       OR p_ip_hash IS NULL OR octet_length(p_ip_hash) <> 32 THEN
        RAISE EXCEPTION 'password login rate-limit fingerprint is invalid'
            USING ERRCODE = '22023';
    END IF;

    DELETE FROM public.password_login_rate_limits
     WHERE updated_at < v_now - INTERVAL '1 day';

    INSERT INTO public.password_login_rate_limits (
        scope, key_hash, window_started_at, attempt_count, blocked_until, updated_at
    )
    VALUES ('identifier', p_identifier_hash, v_now, 1, NULL, v_now)
    ON CONFLICT (scope, key_hash) DO UPDATE
       SET window_started_at = CASE
               WHEN password_login_rate_limits.window_started_at <= v_now - INTERVAL '15 minutes'
               THEN v_now ELSE password_login_rate_limits.window_started_at END,
           attempt_count = CASE
               WHEN password_login_rate_limits.window_started_at <= v_now - INTERVAL '15 minutes'
               THEN 1 ELSE password_login_rate_limits.attempt_count + 1 END,
           blocked_until = CASE
               WHEN password_login_rate_limits.window_started_at <= v_now - INTERVAL '15 minutes'
               THEN NULL
               WHEN password_login_rate_limits.blocked_until > v_now
               THEN password_login_rate_limits.blocked_until
               WHEN password_login_rate_limits.attempt_count + 1 > 10
               THEN v_now + INTERVAL '15 minutes'
               ELSE password_login_rate_limits.blocked_until END,
           updated_at = v_now
    RETURNING attempt_count, blocked_until
         INTO v_identifier_count, v_identifier_block;

    INSERT INTO public.password_login_rate_limits (
        scope, key_hash, window_started_at, attempt_count, blocked_until, updated_at
    )
    VALUES ('ip', p_ip_hash, v_now, 1, NULL, v_now)
    ON CONFLICT (scope, key_hash) DO UPDATE
       SET window_started_at = CASE
               WHEN password_login_rate_limits.window_started_at <= v_now - INTERVAL '15 minutes'
               THEN v_now ELSE password_login_rate_limits.window_started_at END,
           attempt_count = CASE
               WHEN password_login_rate_limits.window_started_at <= v_now - INTERVAL '15 minutes'
               THEN 1 ELSE password_login_rate_limits.attempt_count + 1 END,
           blocked_until = CASE
               WHEN password_login_rate_limits.window_started_at <= v_now - INTERVAL '15 minutes'
               THEN NULL
               WHEN password_login_rate_limits.blocked_until > v_now
               THEN password_login_rate_limits.blocked_until
               WHEN password_login_rate_limits.attempt_count + 1 > 30
               THEN v_now + INTERVAL '15 minutes'
               ELSE password_login_rate_limits.blocked_until END,
           updated_at = v_now
    RETURNING attempt_count, blocked_until
         INTO v_ip_count, v_ip_block;

    RETURN v_identifier_count <= 10
       AND (v_identifier_block IS NULL OR v_identifier_block <= v_now)
       AND v_ip_count <= 30
       AND (v_ip_block IS NULL OR v_ip_block <= v_now);
END;
$$;

CREATE FUNCTION public.clear_password_login_identifier_limit(
    p_identifier_hash BYTEA
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    IF p_identifier_hash IS NULL OR octet_length(p_identifier_hash) <> 32 THEN
        RAISE EXCEPTION 'password login identifier fingerprint is invalid'
            USING ERRCODE = '22023';
    END IF;

    DELETE FROM public.password_login_rate_limits
     WHERE scope = 'identifier'
       AND key_hash = p_identifier_hash;
END;
$$;

REVOKE ALL ON TABLE public.password_login_rate_limits FROM PUBLIC;
REVOKE ALL ON FUNCTION public.find_active_password_login(TEXT, TEXT) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.create_password_login_session(UUID, BYTEA) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.revoke_password_login_session(BYTEA) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.reserve_password_login_attempt(BYTEA, BYTEA) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.clear_password_login_identifier_limit(BYTEA) FROM PUBLIC;

COMMENT ON TABLE public.password_login_rate_limits IS
    'Fingerprint HMAC per identifier dan IP untuk membatasi percobaan login tanpa menyimpan kontak atau alamat IP mentah.';
COMMENT ON FUNCTION public.find_active_password_login(TEXT, TEXT) IS
    'Mengambil kredensial akun aktif hanya untuk role login publik yang diminta.';
COMMENT ON FUNCTION public.create_password_login_session(UUID, BYTEA) IS
    'Membuat sesi opaque untuk akun aktif melalui operasi database terkontrol.';
COMMENT ON FUNCTION public.revoke_password_login_session(BYTEA) IS
    'Mencabut sesi aktif berdasarkan hash token pada logout.';
COMMENT ON FUNCTION public.reserve_password_login_attempt(BYTEA, BYTEA) IS
    'Menerapkan batas 10 percobaan per identifier dan 30 per IP dalam jendela 15 menit.';
COMMENT ON FUNCTION public.clear_password_login_identifier_limit(BYTEA) IS
    'Menghapus penghitung identifier setelah autentikasi berhasil.';
