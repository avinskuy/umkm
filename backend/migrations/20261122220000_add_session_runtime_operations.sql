-- Identity Core session runtime: validate/touch, rotate and revoke all active sessions.
-- The application principal receives only these narrowly scoped operations.

CREATE FUNCTION public.get_active_password_login_session(
    p_token_hash BYTEA
)
RETURNS TABLE (
    session_id UUID,
    user_id UUID,
    primary_role TEXT,
    expires_in_seconds BIGINT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_session_id UUID;
    v_user_id UUID;
    v_primary_role TEXT;
    v_account_status TEXT;
    v_expires_at TIMESTAMPTZ;
    v_last_seen_at TIMESTAMPTZ;
BEGIN
    IF p_token_hash IS NULL OR octet_length(p_token_hash) <> 32 THEN
        RETURN;
    END IF;

    SELECT s.user_id INTO v_user_id
      FROM public.sessions AS s
     WHERE s.token_hash = p_token_hash
       AND s.revoked_at IS NULL;

    IF NOT FOUND THEN
        RETURN;
    END IF;

    SELECT u.primary_role, u.account_status
      INTO v_primary_role, v_account_status
      FROM public.users AS u
     WHERE u.id = v_user_id
     FOR SHARE;

    IF NOT FOUND OR v_account_status <> 'active' THEN
        RETURN;
    END IF;

    SELECT s.id, s.expires_at, s.last_seen_at
      INTO v_session_id, v_expires_at, v_last_seen_at
      FROM public.sessions AS s
     WHERE s.token_hash = p_token_hash
       AND s.user_id = v_user_id
       AND s.revoked_at IS NULL
       AND s.expires_at > clock_timestamp()
       AND s.last_seen_at > clock_timestamp() - INTERVAL '7 days'
     FOR UPDATE;

    IF NOT FOUND
       OR v_expires_at <= clock_timestamp()
       OR v_last_seen_at <= clock_timestamp() - INTERVAL '7 days' THEN
        RETURN;
    END IF;

    UPDATE public.sessions
       SET last_seen_at = clock_timestamp()
     WHERE id = v_session_id;

    session_id := v_session_id;
    user_id := v_user_id;
    primary_role := v_primary_role;
    expires_in_seconds := GREATEST(
        1,
        FLOOR(EXTRACT(EPOCH FROM (v_expires_at - clock_timestamp())))::BIGINT
    );
    RETURN NEXT;
END;
$$;

CREATE FUNCTION public.rotate_active_password_login_session(
    p_token_hash BYTEA,
    p_new_token_hash BYTEA
)
RETURNS TABLE (
    session_id UUID,
    user_id UUID,
    primary_role TEXT,
    expires_in_seconds BIGINT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_session_id UUID;
    v_user_id UUID;
    v_primary_role TEXT;
    v_account_status TEXT;
    v_expires_at TIMESTAMPTZ;
    v_last_seen_at TIMESTAMPTZ;
BEGIN
    IF p_token_hash IS NULL OR octet_length(p_token_hash) <> 32
       OR p_new_token_hash IS NULL OR octet_length(p_new_token_hash) <> 32
       OR p_token_hash = p_new_token_hash THEN
        RETURN;
    END IF;

    SELECT s.user_id INTO v_user_id
      FROM public.sessions AS s
     WHERE s.token_hash = p_token_hash
       AND s.revoked_at IS NULL;

    IF NOT FOUND THEN
        RETURN;
    END IF;

    SELECT u.primary_role, u.account_status
      INTO v_primary_role, v_account_status
      FROM public.users AS u
     WHERE u.id = v_user_id
     FOR SHARE;

    IF NOT FOUND OR v_account_status <> 'active' THEN
        RETURN;
    END IF;

    SELECT s.id, s.expires_at, s.last_seen_at
      INTO v_session_id, v_expires_at, v_last_seen_at
      FROM public.sessions AS s
     WHERE s.token_hash = p_token_hash
       AND s.user_id = v_user_id
       AND s.revoked_at IS NULL
       AND s.expires_at > clock_timestamp()
       AND s.last_seen_at > clock_timestamp() - INTERVAL '7 days'
     FOR UPDATE;

    IF NOT FOUND
       OR v_expires_at <= clock_timestamp()
       OR v_last_seen_at <= clock_timestamp() - INTERVAL '7 days' THEN
        RETURN;
    END IF;

    UPDATE public.sessions
       SET token_hash = p_new_token_hash
     WHERE id = v_session_id;

    session_id := v_session_id;
    user_id := v_user_id;
    primary_role := v_primary_role;
    expires_in_seconds := GREATEST(
        1,
        FLOOR(EXTRACT(EPOCH FROM (v_expires_at - clock_timestamp())))::BIGINT
    );
    RETURN NEXT;
END;
$$;

CREATE FUNCTION public.revoke_all_password_login_sessions(
    p_current_token_hash BYTEA
)
RETURNS INTEGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_revoked_count INTEGER;
    v_user_id UUID;
BEGIN
    IF p_current_token_hash IS NULL OR octet_length(p_current_token_hash) <> 32 THEN
        RETURN 0;
    END IF;

    SELECT user_id INTO v_user_id
      FROM public.sessions
     WHERE token_hash = p_current_token_hash
       AND revoked_at IS NULL
       AND expires_at > clock_timestamp()
       AND last_seen_at > clock_timestamp() - INTERVAL '7 days';

    IF NOT FOUND THEN
        RETURN 0;
    END IF;

    -- Lock every active session in a stable order to avoid deadlocks when
    -- two devices request logout-all concurrently.
    PERFORM s.id
      FROM public.sessions AS s
     WHERE s.user_id = v_user_id
       AND s.revoked_at IS NULL
     ORDER BY s.id
     FOR UPDATE;

    IF NOT EXISTS (
        SELECT 1
          FROM public.sessions
         WHERE token_hash = p_current_token_hash
           AND user_id = v_user_id
           AND revoked_at IS NULL
           AND expires_at > clock_timestamp()
           AND last_seen_at > clock_timestamp() - INTERVAL '7 days'
    ) THEN
        RETURN 0;
    END IF;

    UPDATE public.sessions
       SET revoked_at = clock_timestamp(),
           revocation_reason = 'logout_all'
     WHERE user_id = v_user_id
       AND revoked_at IS NULL;

    GET DIAGNOSTICS v_revoked_count = ROW_COUNT;
    RETURN v_revoked_count;
END;
$$;

REVOKE ALL ON FUNCTION public.get_active_password_login_session(BYTEA) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.rotate_active_password_login_session(BYTEA, BYTEA) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.revoke_all_password_login_sessions(BYTEA) FROM PUBLIC;

COMMENT ON FUNCTION public.get_active_password_login_session(BYTEA) IS
    'Memvalidasi sesi opaque yang belum dicabut, belum melewati expiry absolut 30 hari atau idle 7 hari, memastikan akun aktif, dan memperbarui last_seen_at secara atomik.';
COMMENT ON FUNCTION public.rotate_active_password_login_session(BYTEA, BYTEA) IS
    'Merotasi hash token sesi aktif secara atomik tanpa memperpanjang expiry absolut; token lama langsung tidak berlaku.';
COMMENT ON FUNCTION public.revoke_all_password_login_sessions(BYTEA) IS
    'Mencabut seluruh sesi aktif milik pemegang token sesi aktif yang diberikan dengan reason logout_all.';
