-- Identity registration/recovery abuse guard. Raw IPs and identifiers are never stored.
CREATE TABLE public.identity_abuse_rate_limits (
    scope TEXT NOT NULL,
    key_hash BYTEA NOT NULL,
    window_started_at TIMESTAMPTZ NOT NULL,
    attempt_count INTEGER NOT NULL CHECK (attempt_count > 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (scope, key_hash),
    CHECK (octet_length(key_hash) = 32)
 );

CREATE INDEX identity_abuse_rate_limits_updated_at_idx
    ON public.identity_abuse_rate_limits (updated_at);

CREATE FUNCTION public.reserve_identity_abuse_attempt(
    p_scope TEXT,
    p_key_hash BYTEA,
    p_limit INTEGER,
    p_window_seconds INTEGER
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_attempt_count INTEGER;
    v_now TIMESTAMPTZ := clock_timestamp();
BEGIN
    IF p_scope IS NULL OR p_scope NOT IN (
        'registration:create:ip',
        'registration:resend:ip', 'registration:resend:subject',
        'registration:verify:ip', 'registration:verify:subject',
        'recovery:start:ip',
        'recovery:resend:ip', 'recovery:resend:subject',
        'recovery:verify:ip', 'recovery:verify:subject',
        'recovery:candidates:ip', 'recovery:candidates:subject',
        'recovery:select:ip', 'recovery:select:subject',
        'recovery:password_reset_complete:ip',
        'recovery:password_reset_complete:subject'
    )
       OR p_key_hash IS NULL OR octet_length(p_key_hash) <> 32
       OR p_limit IS NULL OR p_limit < 1 OR p_limit > 1000
       OR p_window_seconds IS NULL OR p_window_seconds < 1 OR p_window_seconds > 86400 THEN
        RAISE EXCEPTION 'Identity abuse limit input is invalid'
            USING ERRCODE = '22023';
    END IF;

    INSERT INTO public.identity_abuse_rate_limits AS current_limit (
        scope, key_hash, window_started_at, attempt_count, updated_at
    )
    VALUES (p_scope, p_key_hash, v_now, 1, v_now)
    ON CONFLICT (scope, key_hash) DO UPDATE
       SET window_started_at = CASE
               WHEN current_limit.window_started_at
                    <= v_now - pg_catalog.make_interval(secs => p_window_seconds)
               THEN v_now
               ELSE public.identity_abuse_rate_limits.window_started_at
           END,
           attempt_count = CASE
               WHEN public.identity_abuse_rate_limits.window_started_at
                    <= v_now - pg_catalog.make_interval(secs => p_window_seconds)
               THEN 1
               ELSE current_limit.attempt_count + 1
           END,
           updated_at = v_now
    RETURNING attempt_count INTO v_attempt_count;

    DELETE FROM public.identity_abuse_rate_limits
     WHERE updated_at < v_now - INTERVAL '1 day';

    RETURN v_attempt_count <= p_limit;
END;
$$;

REVOKE ALL ON TABLE public.identity_abuse_rate_limits FROM PUBLIC;
REVOKE ALL ON FUNCTION public.reserve_identity_abuse_attempt(TEXT, BYTEA, INTEGER, INTEGER) FROM PUBLIC;

COMMENT ON TABLE public.identity_abuse_rate_limits IS
    'Identity registration/recovery rate limits using purpose-separated HMAC fingerprints; raw IPs and identifiers are not stored.';
COMMENT ON FUNCTION public.reserve_identity_abuse_attempt(TEXT, BYTEA, INTEGER, INTEGER) IS
    'Atomically reserves one rate-limit attempt for a fixed Identity scope; callers must pass a keyed 32-byte fingerprint.';
