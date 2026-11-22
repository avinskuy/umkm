-- Rate limit untuk endpoint registrasi, resend, dan verifikasi sebelum operasi OTP yang lebih mahal.

CREATE TABLE public.registration_http_rate_limits (
    action TEXT NOT NULL,
    scope TEXT NOT NULL,
    key_hash BYTEA NOT NULL,
    window_started_at TIMESTAMPTZ NOT NULL,
    attempt_count INTEGER NOT NULL,
    blocked_until TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),

    CONSTRAINT registration_http_rate_limits_pk PRIMARY KEY (action, scope, key_hash),
    CONSTRAINT registration_http_rate_limits_action_check
        CHECK (action IN ('create', 'resend', 'verify')),
    CONSTRAINT registration_http_rate_limits_scope_check
        CHECK (scope IN ('ip', 'subject')),
    CONSTRAINT registration_http_rate_limits_hash_check
        CHECK (octet_length(key_hash) = 32),
    CONSTRAINT registration_http_rate_limits_count_check
        CHECK (attempt_count >= 1),
    CONSTRAINT registration_http_rate_limits_block_check
        CHECK (blocked_until IS NULL OR blocked_until > window_started_at)
);

CREATE INDEX registration_http_rate_limits_updated_idx
    ON public.registration_http_rate_limits (updated_at);

CREATE FUNCTION public.reserve_registration_http_attempt(
    p_action TEXT,
    p_ip_hash BYTEA,
    p_subject_hash BYTEA
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_now TIMESTAMPTZ := clock_timestamp();
    v_ip_limit INTEGER;
    v_subject_limit INTEGER;
    v_bucket RECORD;
    v_attempt_count INTEGER;
    v_blocked_until TIMESTAMPTZ;
    v_allowed BOOLEAN := TRUE;
BEGIN
    IF p_action NOT IN ('create', 'resend', 'verify')
       OR p_ip_hash IS NULL OR octet_length(p_ip_hash) <> 32 THEN
        RAISE EXCEPTION 'registration rate limit input is invalid'
            USING ERRCODE = '22023';
    END IF;

    CASE p_action
        WHEN 'create' THEN
            v_ip_limit := 5;
            v_subject_limit := NULL;
            IF p_subject_hash IS NOT NULL THEN
                RAISE EXCEPTION 'registration create must not include a subject hash'
                    USING ERRCODE = '22023';
            END IF;
        WHEN 'resend' THEN
            v_ip_limit := 30;
            v_subject_limit := 5;
        WHEN 'verify' THEN
            v_ip_limit := 60;
            v_subject_limit := 10;
    END CASE;

    IF p_action IN ('resend', 'verify')
       AND (p_subject_hash IS NULL OR octet_length(p_subject_hash) <> 32) THEN
        RAISE EXCEPTION 'registration subject hash is required'
            USING ERRCODE = '22023';
    END IF;

    DELETE FROM public.registration_http_rate_limits
     WHERE updated_at < v_now - INTERVAL '1 day';

    FOR v_bucket IN
        SELECT bucket.scope, bucket.key_hash, bucket.attempt_limit
          FROM (
              VALUES
                  ('ip'::TEXT, p_ip_hash, v_ip_limit),
                  ('subject'::TEXT, p_subject_hash, v_subject_limit)
          ) AS bucket(scope, key_hash, attempt_limit)
         WHERE bucket.key_hash IS NOT NULL
         ORDER BY bucket.scope
    LOOP
        PERFORM pg_catalog.pg_advisory_xact_lock(
            pg_catalog.hashtextextended(
                'registration-http:' || p_action || ':' || v_bucket.scope || ':' ||
                pg_catalog.encode(v_bucket.key_hash, 'hex'),
                0
            )
        );

        INSERT INTO public.registration_http_rate_limits (
            action, scope, key_hash, window_started_at, attempt_count, blocked_until
        )
        VALUES (
            p_action, v_bucket.scope, v_bucket.key_hash, v_now, 1, NULL
        )
        ON CONFLICT (action, scope, key_hash) DO UPDATE
        SET window_started_at = CASE
                WHEN public.registration_http_rate_limits.window_started_at <= v_now - INTERVAL '15 minutes'
                 AND (
                     public.registration_http_rate_limits.blocked_until IS NULL
                     OR public.registration_http_rate_limits.blocked_until <= v_now
                 )
                THEN v_now
                ELSE public.registration_http_rate_limits.window_started_at
            END,
            attempt_count = CASE
                WHEN public.registration_http_rate_limits.window_started_at <= v_now - INTERVAL '15 minutes'
                 AND (
                     public.registration_http_rate_limits.blocked_until IS NULL
                     OR public.registration_http_rate_limits.blocked_until <= v_now
                 )
                THEN 1
                ELSE public.registration_http_rate_limits.attempt_count + 1
            END,
            blocked_until = CASE
                WHEN public.registration_http_rate_limits.window_started_at <= v_now - INTERVAL '15 minutes'
                 AND (
                     public.registration_http_rate_limits.blocked_until IS NULL
                     OR public.registration_http_rate_limits.blocked_until <= v_now
                 )
                THEN NULL
                WHEN public.registration_http_rate_limits.blocked_until > v_now
                THEN public.registration_http_rate_limits.blocked_until
                WHEN public.registration_http_rate_limits.attempt_count + 1 > v_bucket.attempt_limit
                THEN v_now + INTERVAL '15 minutes'
                ELSE NULL
            END,
            updated_at = v_now
        RETURNING attempt_count, blocked_until
             INTO v_attempt_count, v_blocked_until;

        IF v_attempt_count > v_bucket.attempt_limit
           OR (v_blocked_until IS NOT NULL AND v_blocked_until > v_now) THEN
            v_allowed := FALSE;
        END IF;
    END LOOP;

    RETURN v_allowed;
END;
$$;

REVOKE ALL ON TABLE public.registration_http_rate_limits FROM PUBLIC;
REVOKE ALL ON FUNCTION public.reserve_registration_http_attempt(TEXT, BYTEA, BYTEA) FROM PUBLIC;

COMMENT ON TABLE public.registration_http_rate_limits IS
    'HMAC fingerprint-only rate-limit buckets for registration HTTP actions; no raw IP address or verification ID is stored.';
COMMENT ON FUNCTION public.reserve_registration_http_attempt(TEXT, BYTEA, BYTEA) IS
    'Reserves one registration HTTP attempt atomically using HMAC fingerprints, per-action IP limits, and per-session subject limits.';
