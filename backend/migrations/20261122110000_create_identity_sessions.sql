-- Identity Core slice 0003: opaque authentication sessions.
-- Runtime authentication, cookie handling and grants are intentionally external.

CREATE TABLE public.sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    token_hash BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL,
    token_rotated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    remember_device BOOLEAN NOT NULL DEFAULT FALSE,
    revoked_at TIMESTAMPTZ,
    revocation_reason TEXT,
    user_agent_summary TEXT,
    ip_prefix_hash BYTEA,
    ip_prefix_hmac_key_version TEXT,

    CONSTRAINT sessions_user_fk
        FOREIGN KEY (user_id) REFERENCES public.users (id) ON DELETE RESTRICT,
    CONSTRAINT sessions_token_hash_check
        CHECK (octet_length(token_hash) = 32),
    CONSTRAINT sessions_ip_prefix_hash_check
        CHECK (
            (ip_prefix_hash IS NULL AND ip_prefix_hmac_key_version IS NULL)
            OR (
                ip_prefix_hash IS NOT NULL
                AND octet_length(ip_prefix_hash) = 32
                AND ip_prefix_hmac_key_version IS NOT NULL
                AND length(btrim(ip_prefix_hmac_key_version)) > 0
            )
        ),
    CONSTRAINT sessions_user_agent_summary_check
        CHECK (
            user_agent_summary IS NULL
            OR (
                length(user_agent_summary) BETWEEN 1 AND 256
                AND user_agent_summary = btrim(user_agent_summary)
            )
        ),
    CONSTRAINT sessions_revocation_reason_check
        CHECK (
            revocation_reason IS NULL
            OR revocation_reason IN (
                'logout',
                'user_revoked',
                'logout_all',
                'password_reset',
                'account_status_changed',
                'security_event',
                'admin_action'
            )
        ),
    CONSTRAINT sessions_revocation_pair_check
        CHECK (
            (revoked_at IS NULL AND revocation_reason IS NULL)
            OR (revoked_at IS NOT NULL AND revocation_reason IS NOT NULL)
        ),
    CONSTRAINT sessions_timestamp_order_check
        CHECK (
            created_at <= last_seen_at
            AND created_at <= token_rotated_at
            AND token_rotated_at <= last_seen_at
            AND expires_at > created_at
        ),
    CONSTRAINT sessions_token_hash_uidx UNIQUE (token_hash)
);

CREATE INDEX sessions_user_active_idx
    ON public.sessions (user_id, last_seen_at DESC, expires_at)
    WHERE revoked_at IS NULL;

CREATE INDEX sessions_user_history_idx
    ON public.sessions (user_id, created_at DESC);

CREATE FUNCTION public.guard_session()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
DECLARE
    v_account_status TEXT;
    v_now TIMESTAMPTZ;
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.revoked_at IS NOT NULL OR NEW.revocation_reason IS NOT NULL THEN
            RAISE EXCEPTION 'new session cannot start revoked'
                USING ERRCODE = '23514';
        END IF;

        SELECT account_status
        INTO v_account_status
        FROM public.users
        WHERE id = NEW.user_id
        FOR SHARE;

        IF NOT FOUND OR v_account_status <> 'active' THEN
            RAISE EXCEPTION 'sessions may only be created for active accounts'
                USING ERRCODE = '23514';
        END IF;

        NEW.created_at := clock_timestamp();
        NEW.last_seen_at := NEW.created_at;
        NEW.expires_at := NEW.created_at + INTERVAL '30 days';
        NEW.token_rotated_at := NEW.created_at;
        RETURN NEW;
    END IF;

    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.user_id IS DISTINCT FROM OLD.user_id
       OR NEW.created_at IS DISTINCT FROM OLD.created_at
       OR NEW.expires_at IS DISTINCT FROM OLD.expires_at
       OR NEW.remember_device IS DISTINCT FROM OLD.remember_device
       OR NEW.user_agent_summary IS DISTINCT FROM OLD.user_agent_summary
       OR NEW.ip_prefix_hash IS DISTINCT FROM OLD.ip_prefix_hash
       OR NEW.ip_prefix_hmac_key_version IS DISTINCT FROM OLD.ip_prefix_hmac_key_version THEN
        RAISE EXCEPTION 'session identity, lifetime and device metadata are immutable'
            USING ERRCODE = '23514';
    END IF;

    IF OLD.revoked_at IS NOT NULL THEN
        RAISE EXCEPTION 'revoked session is immutable'
            USING ERRCODE = '23514';
    END IF;

    IF NEW.revoked_at IS DISTINCT FROM OLD.revoked_at THEN
        IF NEW.revoked_at IS NULL OR NEW.revocation_reason IS NULL THEN
            RAISE EXCEPTION 'session revocation cannot be reversed'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.token_hash IS DISTINCT FROM OLD.token_hash
           OR NEW.last_seen_at IS DISTINCT FROM OLD.last_seen_at
           OR NEW.token_rotated_at IS DISTINCT FROM OLD.token_rotated_at THEN
            RAISE EXCEPTION 'revocation cannot be combined with session activity or token rotation'
                USING ERRCODE = '23514';
        END IF;

        NEW.revoked_at := clock_timestamp();
        RETURN NEW;
    END IF;

    IF NEW.revocation_reason IS DISTINCT FROM OLD.revocation_reason THEN
        RAISE EXCEPTION 'session revocation reason is immutable'
            USING ERRCODE = '23514';
    END IF;

    IF NEW.token_hash IS DISTINCT FROM OLD.token_hash
       OR NEW.last_seen_at IS DISTINCT FROM OLD.last_seen_at THEN
        IF OLD.expires_at <= clock_timestamp()
           OR OLD.last_seen_at <= clock_timestamp() - INTERVAL '7 days' THEN
            RAISE EXCEPTION 'expired or idle session cannot be renewed'
                USING ERRCODE = '23514';
        END IF;

        SELECT account_status
        INTO v_account_status
        FROM public.users
        WHERE id = OLD.user_id
        FOR SHARE;

        IF NOT FOUND OR v_account_status <> 'active' THEN
            RAISE EXCEPTION 'inactive account session cannot be renewed'
                USING ERRCODE = '23514';
        END IF;
    END IF;

    IF NEW.token_hash IS DISTINCT FROM OLD.token_hash THEN
        v_now := clock_timestamp();
        NEW.token_rotated_at := v_now;
        NEW.last_seen_at := v_now;
    ELSIF NEW.token_rotated_at IS DISTINCT FROM OLD.token_rotated_at THEN
        RAISE EXCEPTION 'token rotation timestamp is database managed'
            USING ERRCODE = '23514';
    END IF;

    IF NEW.last_seen_at IS DISTINCT FROM OLD.last_seen_at
       AND NEW.token_hash IS NOT DISTINCT FROM OLD.token_hash THEN
        NEW.last_seen_at := clock_timestamp();
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER sessions_10_guard
    BEFORE INSERT OR UPDATE ON public.sessions
    FOR EACH ROW EXECUTE FUNCTION public.guard_session();

CREATE FUNCTION public.revoke_sessions_when_account_inactive()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
BEGIN
    IF NEW.account_status IS DISTINCT FROM OLD.account_status
       AND NEW.account_status <> 'active' THEN
        UPDATE public.sessions
        SET revoked_at = clock_timestamp(),
            revocation_reason = 'account_status_changed'
        WHERE user_id = NEW.id
          AND revoked_at IS NULL;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER users_90_revoke_sessions
    AFTER UPDATE OF account_status ON public.users
    FOR EACH ROW EXECUTE FUNCTION public.revoke_sessions_when_account_inactive();
