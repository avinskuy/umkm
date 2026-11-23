-- Dedicated Admin authentication persistence and atomic TOTP/session operations.
-- Application access is restricted to SECURITY DEFINER functions granted separately.

CREATE TABLE public.admin_login_rate_limits (
    scope TEXT NOT NULL,
    key_hash BYTEA NOT NULL,
    window_started_at TIMESTAMPTZ NOT NULL,
    attempt_count INTEGER NOT NULL,
    blocked_until TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT admin_login_rate_limits_pk PRIMARY KEY (scope, key_hash),
    CONSTRAINT admin_login_rate_limits_scope_check CHECK (scope IN ('identifier', 'ip')),
    CONSTRAINT admin_login_rate_limits_key_hash_check CHECK (octet_length(key_hash) = 32),
    CONSTRAINT admin_login_rate_limits_attempt_count_check CHECK (attempt_count > 0),
    CONSTRAINT admin_login_rate_limits_block_check
        CHECK (blocked_until IS NULL OR blocked_until > window_started_at)
);

CREATE INDEX admin_login_rate_limits_updated_idx
    ON public.admin_login_rate_limits (updated_at);

CREATE TABLE public.admin_login_challenges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES public.users (id) ON DELETE RESTRICT,
    challenge_hash BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    consumed_at TIMESTAMPTZ,
    CONSTRAINT admin_login_challenges_hash_check CHECK (octet_length(challenge_hash) = 32),
    CONSTRAINT admin_login_challenges_expiry_check CHECK (expires_at > created_at),
    CONSTRAINT admin_login_challenges_attempt_check CHECK (attempt_count BETWEEN 0 AND 5),
    CONSTRAINT admin_login_challenges_consumed_check
        CHECK (consumed_at IS NULL OR consumed_at >= created_at)
);

CREATE INDEX admin_login_challenges_user_created_idx
    ON public.admin_login_challenges (user_id, created_at DESC);

CREATE INDEX admin_login_challenges_expiry_idx
    ON public.admin_login_challenges (expires_at)
    WHERE consumed_at IS NULL;

CREATE TABLE public.admin_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES public.users (id) ON DELETE RESTRICT,
    token_hash BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT (clock_timestamp() + INTERVAL '8 hours'),
    revoked_at TIMESTAMPTZ,
    revocation_reason TEXT,
    CONSTRAINT admin_sessions_token_hash_check CHECK (octet_length(token_hash) = 32),
    CONSTRAINT admin_sessions_expiry_check CHECK (expires_at > created_at),
    CONSTRAINT admin_sessions_activity_check
        CHECK (last_seen_at >= created_at AND last_seen_at <= clock_timestamp() + INTERVAL '1 minute'),
    CONSTRAINT admin_sessions_revocation_check
        CHECK (
            (revoked_at IS NULL AND revocation_reason IS NULL)
            OR
            (revoked_at IS NOT NULL AND revocation_reason IN ('logout', 'account_status_changed', 'security_event', 'operator_action'))
        )
);

CREATE INDEX admin_sessions_user_active_idx
    ON public.admin_sessions (user_id, expires_at DESC)
    WHERE revoked_at IS NULL;

CREATE FUNCTION public.find_active_admin_password_login(p_identifier TEXT)
RETURNS TABLE (user_id UUID, password_hash TEXT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    IF p_identifier IS NULL OR length(btrim(p_identifier)) = 0 THEN
        RETURN;
    END IF;

    RETURN QUERY
    SELECT u.id, u.password_hash
      FROM public.users AS u
     WHERE u.account_status = 'active'
       AND u.primary_role = 'admin'
       AND u.password_hash IS NOT NULL
       AND u.email_normalized = p_identifier
     LIMIT 1;
END;
$$;

CREATE FUNCTION public.get_admin_totp_credentials(p_user_id UUID)
RETURNS TABLE (
    secret_ciphertext BYTEA,
    secret_nonce BYTEA,
    encryption_key_version TEXT,
    last_accepted_step BIGINT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    IF p_user_id IS NULL THEN
        RETURN;
    END IF;

    RETURN QUERY
    SELECT c.secret_ciphertext, c.secret_nonce, c.encryption_key_version, c.last_accepted_step
      FROM public.admin_totp_credentials AS c
      JOIN public.users AS u ON u.id = c.user_id
     WHERE c.user_id = p_user_id
       AND c.enabled_at IS NOT NULL
       AND u.primary_role = 'admin'
       AND u.account_status = 'active';
END;
$$;

CREATE FUNCTION public.reserve_admin_login_attempt(p_identifier_hash BYTEA, p_ip_hash BYTEA)
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
        RAISE EXCEPTION 'Admin login rate-limit fingerprint is invalid'
            USING ERRCODE = '22023';
    END IF;

    DELETE FROM public.admin_login_rate_limits
     WHERE updated_at < v_now - INTERVAL '1 day';

    INSERT INTO public.admin_login_rate_limits (
        scope, key_hash, window_started_at, attempt_count, blocked_until, updated_at
    )
    VALUES ('identifier', p_identifier_hash, v_now, 1, NULL, v_now)
    ON CONFLICT (scope, key_hash) DO UPDATE
       SET window_started_at = CASE
               WHEN admin_login_rate_limits.window_started_at <= v_now - INTERVAL '15 minutes'
                AND (admin_login_rate_limits.blocked_until IS NULL OR admin_login_rate_limits.blocked_until <= v_now)
               THEN v_now ELSE admin_login_rate_limits.window_started_at END,
           attempt_count = CASE
               WHEN admin_login_rate_limits.window_started_at <= v_now - INTERVAL '15 minutes'
                AND (admin_login_rate_limits.blocked_until IS NULL OR admin_login_rate_limits.blocked_until <= v_now)
               THEN 1 ELSE admin_login_rate_limits.attempt_count + 1 END,
           blocked_until = CASE
               WHEN admin_login_rate_limits.window_started_at <= v_now - INTERVAL '15 minutes'
                AND (admin_login_rate_limits.blocked_until IS NULL OR admin_login_rate_limits.blocked_until <= v_now)
               THEN NULL
               WHEN admin_login_rate_limits.blocked_until > v_now
               THEN admin_login_rate_limits.blocked_until
               WHEN admin_login_rate_limits.attempt_count + 1 > 5
               THEN v_now + INTERVAL '15 minutes'
               ELSE admin_login_rate_limits.blocked_until END,
           updated_at = v_now
    RETURNING attempt_count, blocked_until
         INTO v_identifier_count, v_identifier_block;

    INSERT INTO public.admin_login_rate_limits (
        scope, key_hash, window_started_at, attempt_count, blocked_until, updated_at
    )
    VALUES ('ip', p_ip_hash, v_now, 1, NULL, v_now)
    ON CONFLICT (scope, key_hash) DO UPDATE
       SET window_started_at = CASE
               WHEN admin_login_rate_limits.window_started_at <= v_now - INTERVAL '15 minutes'
                AND (admin_login_rate_limits.blocked_until IS NULL OR admin_login_rate_limits.blocked_until <= v_now)
               THEN v_now ELSE admin_login_rate_limits.window_started_at END,
           attempt_count = CASE
               WHEN admin_login_rate_limits.window_started_at <= v_now - INTERVAL '15 minutes'
                AND (admin_login_rate_limits.blocked_until IS NULL OR admin_login_rate_limits.blocked_until <= v_now)
               THEN 1 ELSE admin_login_rate_limits.attempt_count + 1 END,
           blocked_until = CASE
               WHEN admin_login_rate_limits.window_started_at <= v_now - INTERVAL '15 minutes'
                AND (admin_login_rate_limits.blocked_until IS NULL OR admin_login_rate_limits.blocked_until <= v_now)
               THEN NULL
               WHEN admin_login_rate_limits.blocked_until > v_now
               THEN admin_login_rate_limits.blocked_until
               WHEN admin_login_rate_limits.attempt_count + 1 > 20
               THEN v_now + INTERVAL '15 minutes'
               ELSE admin_login_rate_limits.blocked_until END,
           updated_at = v_now
    RETURNING attempt_count, blocked_until
         INTO v_ip_count, v_ip_block;

    RETURN v_identifier_count <= 5
       AND (v_identifier_block IS NULL OR v_identifier_block <= v_now)
       AND v_ip_count <= 20
       AND (v_ip_block IS NULL OR v_ip_block <= v_now);
END;
$$;

CREATE FUNCTION public.create_admin_login_challenge(p_user_id UUID, p_challenge_hash BYTEA)
RETURNS TABLE (challenge_id UUID, expires_at_unix BIGINT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_challenge_id UUID;
    v_expires_at TIMESTAMPTZ;
BEGIN
    IF p_user_id IS NULL OR p_challenge_hash IS NULL OR octet_length(p_challenge_hash) <> 32 THEN
        RAISE EXCEPTION 'Admin login challenge input is invalid'
            USING ERRCODE = '22023';
    END IF;

    PERFORM 1
      FROM public.users AS u
      JOIN public.admin_totp_credentials AS c ON c.user_id = u.id
     WHERE u.id = p_user_id
       AND u.primary_role = 'admin'
       AND u.account_status = 'active'
       AND u.password_hash IS NOT NULL
       AND c.enabled_at IS NOT NULL
     FOR UPDATE OF u;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'active Admin with enrolled TOTP was not found'
            USING ERRCODE = 'P0002';
    END IF;

    UPDATE public.admin_login_challenges
       SET consumed_at = clock_timestamp()
     WHERE user_id = p_user_id
       AND consumed_at IS NULL;

    v_expires_at := clock_timestamp() + INTERVAL '5 minutes';

    INSERT INTO public.admin_login_challenges (user_id, challenge_hash, expires_at)
    VALUES (p_user_id, p_challenge_hash, v_expires_at)
    RETURNING id INTO v_challenge_id;

    RETURN QUERY SELECT v_challenge_id, floor(extract(epoch FROM v_expires_at))::BIGINT;
END;
$$;

CREATE FUNCTION public.record_admin_login_challenge_failure(p_challenge_hash BYTEA)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_attempt_count INTEGER;
BEGIN
    IF p_challenge_hash IS NULL OR octet_length(p_challenge_hash) <> 32 THEN
        RETURN FALSE;
    END IF;

    UPDATE public.admin_login_challenges
       SET attempt_count = attempt_count + 1,
           consumed_at = CASE
               WHEN attempt_count + 1 >= 5 THEN clock_timestamp()
               ELSE consumed_at
           END
     WHERE challenge_hash = p_challenge_hash
       AND consumed_at IS NULL
       AND expires_at > clock_timestamp()
       AND attempt_count < 5
    RETURNING attempt_count INTO v_attempt_count;

    RETURN FOUND AND v_attempt_count < 5;
END;
$$;

CREATE FUNCTION public.complete_admin_login(
    p_challenge_hash BYTEA,
    p_totp_step BIGINT,
    p_session_token_hash BYTEA
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_challenge_id UUID;
    v_user_id UUID;
    v_session_id UUID;
    v_last_step BIGINT;
BEGIN
    IF p_challenge_hash IS NULL OR octet_length(p_challenge_hash) <> 32
       OR p_totp_step IS NULL OR p_totp_step < 0
       OR p_session_token_hash IS NULL OR octet_length(p_session_token_hash) <> 32 THEN
        RAISE EXCEPTION 'Admin login completion input is invalid'
            USING ERRCODE = '22023';
    END IF;

    SELECT c.id, c.user_id
      INTO v_challenge_id, v_user_id
      FROM public.admin_login_challenges AS c
     WHERE c.challenge_hash = p_challenge_hash
       AND c.consumed_at IS NULL
       AND c.expires_at > clock_timestamp()
       AND c.attempt_count < 5
     FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Admin login challenge is invalid or expired'
            USING ERRCODE = '42501';
    END IF;

    PERFORM 1
      FROM public.users AS u
     WHERE u.id = v_user_id
       AND u.primary_role = 'admin'
       AND u.account_status = 'active'
     FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Admin account is not active'
            USING ERRCODE = '42501';
    END IF;

    SELECT c.last_accepted_step
      INTO v_last_step
      FROM public.admin_totp_credentials AS c
     WHERE c.user_id = v_user_id
       AND c.enabled_at IS NOT NULL
     FOR UPDATE;

    IF NOT FOUND OR (v_last_step IS NOT NULL AND p_totp_step <= v_last_step) THEN
        RAISE EXCEPTION 'Admin TOTP step is invalid or already used'
            USING ERRCODE = '42501';
    END IF;

    UPDATE public.admin_totp_credentials
       SET last_accepted_step = p_totp_step
     WHERE user_id = v_user_id;

    UPDATE public.admin_login_challenges
       SET consumed_at = clock_timestamp()
     WHERE id = v_challenge_id;

    INSERT INTO public.admin_sessions (user_id, token_hash)
    VALUES (v_user_id, p_session_token_hash)
    RETURNING id INTO v_session_id;

    INSERT INTO public.identity_audit_events (
        actor_type, actor_id, subject_user_id, action_code, outcome, reason_code, metadata
    )
    VALUES (
        'admin', v_user_id, v_user_id, 'identity.admin.login.succeeded', 'success', 'totp_verified',
        pg_catalog.jsonb_build_object('purpose', 'admin_login', 'session_id', v_session_id::text)
    );

    RETURN v_session_id;
END;
$$;

CREATE FUNCTION public.get_active_admin_session(p_session_token_hash BYTEA)
RETURNS TABLE (session_id UUID, user_id UUID)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    IF p_session_token_hash IS NULL OR octet_length(p_session_token_hash) <> 32 THEN
        RETURN;
    END IF;

    RETURN QUERY
    UPDATE public.admin_sessions AS s
       SET last_seen_at = clock_timestamp()
      FROM public.users AS u
     WHERE s.token_hash = p_session_token_hash
       AND s.user_id = u.id
       AND u.primary_role = 'admin'
       AND u.account_status = 'active'
       AND s.revoked_at IS NULL
       AND s.expires_at > clock_timestamp()
       AND s.last_seen_at > clock_timestamp() - INTERVAL '30 minutes'
    RETURNING s.id, s.user_id;
END;
$$;

CREATE FUNCTION public.revoke_admin_session(p_session_token_hash BYTEA)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    IF p_session_token_hash IS NULL OR octet_length(p_session_token_hash) <> 32 THEN
        RETURN FALSE;
    END IF;

    UPDATE public.admin_sessions
       SET revoked_at = clock_timestamp(), revocation_reason = 'logout'
     WHERE token_hash = p_session_token_hash
       AND revoked_at IS NULL;

    RETURN FOUND;
END;
$$;

REVOKE ALL ON TABLE public.admin_login_rate_limits FROM PUBLIC;
REVOKE ALL ON TABLE public.admin_login_challenges FROM PUBLIC;
REVOKE ALL ON TABLE public.admin_sessions FROM PUBLIC;
REVOKE ALL ON FUNCTION public.find_active_admin_password_login(TEXT) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.get_admin_totp_credentials(UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.reserve_admin_login_attempt(BYTEA, BYTEA) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.create_admin_login_challenge(UUID, BYTEA) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.record_admin_login_challenge_failure(BYTEA) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.complete_admin_login(BYTEA, BIGINT, BYTEA) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.get_active_admin_session(BYTEA) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.revoke_admin_session(BYTEA) FROM PUBLIC;

COMMENT ON TABLE public.admin_login_challenges IS
    'Short-lived, single-use second-factor challenges for the single Admin account.';
COMMENT ON TABLE public.admin_login_rate_limits IS
    'HMAC fingerprints for Admin identifier/IP rate limits; raw identifiers and IP addresses are not stored.';
COMMENT ON TABLE public.admin_sessions IS
    'Dedicated Admin sessions isolated from Business Owner and Investor sessions.';
COMMENT ON FUNCTION public.complete_admin_login(BYTEA, BIGINT, BYTEA) IS
    'Atomically consumes an Admin challenge and TOTP time-step, creates an isolated Admin session, and appends a security audit event.';
