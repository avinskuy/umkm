-- FlowKirim-managed OTP support for registration WhatsApp challenges.
-- Local OTP challenges remain keyed-hash based; FlowKirim challenges never store a fabricated OTP hash.

ALTER TABLE public.verification_challenges
    ALTER COLUMN code_hash DROP NOT NULL,
    ALTER COLUMN hash_key_version DROP NOT NULL,
    ADD COLUMN provider TEXT NOT NULL DEFAULT 'local',
    ADD COLUMN provider_request_id TEXT;

ALTER TABLE public.verification_challenges
    DROP CONSTRAINT verification_challenges_code_hash_check,
    ADD CONSTRAINT verification_challenges_provider_check
        CHECK (provider IN ('local', 'flowkirim')),
    ADD CONSTRAINT verification_challenges_provider_payload_check
        CHECK (
            (provider = 'local'
                AND code_hash IS NOT NULL
                AND octet_length(code_hash) > 0
                AND hash_key_version IS NOT NULL
                AND length(btrim(hash_key_version)) > 0
                AND provider_request_id IS NULL)
            OR
            (provider = 'flowkirim'
                AND code_hash IS NULL
                AND hash_key_version IS NULL
                AND (provider_request_id IS NULL
                    OR (length(btrim(provider_request_id)) BETWEEN 1 AND 256)))
        );

CREATE OR REPLACE FUNCTION public.guard_verification_challenge()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
DECLARE
    v_session_status TEXT;
    v_session_channel TEXT;
    v_session_expires_at TIMESTAMPTZ;
    v_session_locked_until TIMESTAMPTZ;
    v_session_failed_attempt_count SMALLINT;
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.status <> 'pending'
           OR NEW.consumed_at IS NOT NULL
           OR NEW.expired_at IS NOT NULL
           OR NEW.superseded_at IS NOT NULL
           OR NEW.cancelled_at IS NOT NULL THEN
            RAISE EXCEPTION 'verification challenge must start in pending state'
                USING ERRCODE = '23514';
        END IF;

        NEW.created_at := clock_timestamp();
        IF NEW.expires_at <= NEW.created_at THEN
            RAISE EXCEPTION 'verification challenge deadline must be in the future'
                USING ERRCODE = '23514';
        END IF;

        SELECT status, channel, expires_at
          INTO v_session_status, v_session_channel, v_session_expires_at
          FROM public.verification_sessions
         WHERE id = NEW.verification_session_id;

        IF NOT FOUND OR v_session_status <> 'pending' OR NEW.expires_at > v_session_expires_at THEN
            RAISE EXCEPTION 'challenge requires a pending session and cannot outlive it'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.provider = 'flowkirim'
           AND (NEW.provider_request_id IS NOT NULL
                OR v_session_channel <> 'whatsapp'
                OR NOT EXISTS (
                    SELECT 1 FROM public.verification_sessions AS s
                     WHERE s.id = NEW.verification_session_id
                       AND s.purpose = 'registration'
                )) THEN
            RAISE EXCEPTION 'FlowKirim challenge is only valid for WhatsApp registration'
                USING ERRCODE = '23514';
        END IF;
    END IF;

    IF TG_OP = 'UPDATE' THEN
        IF NEW.id IS DISTINCT FROM OLD.id
           OR NEW.verification_session_id IS DISTINCT FROM OLD.verification_session_id
           OR NEW.code_hash IS DISTINCT FROM OLD.code_hash
           OR NEW.hash_key_version IS DISTINCT FROM OLD.hash_key_version
           OR NEW.provider IS DISTINCT FROM OLD.provider
           OR NEW.created_at IS DISTINCT FROM OLD.created_at
           OR NEW.expires_at IS DISTINCT FROM OLD.expires_at THEN
            RAISE EXCEPTION 'verification challenge identity and verification material are immutable'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.provider_request_id IS DISTINCT FROM OLD.provider_request_id THEN
            IF OLD.status <> 'pending'
               OR NEW.status <> 'pending'
               OR OLD.provider <> 'flowkirim'
               OR OLD.provider_request_id IS NOT NULL
               OR NEW.provider_request_id IS NULL
               OR length(btrim(NEW.provider_request_id)) NOT BETWEEN 1 AND 256
               OR NEW.consumed_at IS DISTINCT FROM OLD.consumed_at
               OR NEW.expired_at IS DISTINCT FROM OLD.expired_at
               OR NEW.superseded_at IS DISTINCT FROM OLD.superseded_at
               OR NEW.cancelled_at IS DISTINCT FROM OLD.cancelled_at THEN
                RAISE EXCEPTION 'FlowKirim request identifier can only be bound once to a pending challenge'
                    USING ERRCODE = '23514';
            END IF;
            RETURN NEW;
        END IF;

        IF OLD.status <> 'pending' THEN
            RAISE EXCEPTION 'terminal verification challenge cannot be changed'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.status NOT IN ('consumed', 'expired', 'superseded', 'cancelled') THEN
            RAISE EXCEPTION 'invalid verification challenge transition'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.status = 'consumed' THEN
            IF OLD.expires_at <= clock_timestamp() THEN
                RAISE EXCEPTION 'expired verification challenge cannot be consumed'
                    USING ERRCODE = '23514';
            END IF;

            SELECT status, expires_at, locked_until, failed_attempt_count
              INTO v_session_status, v_session_expires_at, v_session_locked_until, v_session_failed_attempt_count
              FROM public.verification_sessions
             WHERE id = OLD.verification_session_id;

            IF NOT FOUND
               OR v_session_status <> 'pending'
               OR v_session_expires_at <= clock_timestamp()
               OR v_session_failed_attempt_count = 5
               OR (v_session_locked_until IS NOT NULL AND v_session_locked_until > clock_timestamp()) THEN
                RAISE EXCEPTION 'challenge can only be consumed by an eligible verification session'
                    USING ERRCODE = '23514';
            END IF;

            IF OLD.provider = 'flowkirim' AND OLD.provider_request_id IS NULL THEN
                RAISE EXCEPTION 'FlowKirim challenge cannot be consumed before provider request binding'
                    USING ERRCODE = '23514';
            END IF;
        END IF;

        IF NEW.status = 'expired' AND OLD.expires_at > clock_timestamp() THEN
            RAISE EXCEPTION 'verification challenge cannot expire before its deadline'
                USING ERRCODE = '23514';
        END IF;
    END IF;

    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION public.get_pending_otp_challenge(p_session_id UUID)
RETURNS TABLE (challenge_id UUID, code_hash BYTEA, hash_key_version TEXT, expires_at TIMESTAMPTZ)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    RETURN QUERY
    SELECT c.id, c.code_hash, c.hash_key_version, c.expires_at
      FROM public.verification_challenges AS c
      JOIN public.verification_sessions AS s ON s.id = c.verification_session_id
     WHERE c.verification_session_id = p_session_id
       AND c.provider = 'local'
       AND c.status = 'pending'
       AND c.expires_at > clock_timestamp()
       AND s.purpose IN ('registration', 'account_recovery')
       AND s.status = 'pending'
       AND s.expires_at > clock_timestamp()
     ORDER BY c.created_at DESC
     LIMIT 1;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'pending local OTP challenge was not found'
            USING ERRCODE = 'P0002';
    END IF;
END;
$$;

CREATE FUNCTION public.reserve_flowkirim_otp_challenge(
    p_session_id UUID,
    p_challenge_id UUID,
    p_destination_hmac_aliases BYTEA[]
)
RETURNS TABLE (challenge_id UUID, delivery_event_id UUID)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_session public.verification_sessions%ROWTYPE;
    v_now TIMESTAMPTZ := clock_timestamp();
    v_send_count BIGINT;
    v_last_sent_at TIMESTAMPTZ;
    v_alias BYTEA;
BEGIN
    SELECT * INTO v_session
      FROM public.verification_sessions
     WHERE id = p_session_id
     FOR UPDATE;

    IF NOT FOUND
       OR p_challenge_id IS NULL
       OR v_session.status <> 'pending'
       OR v_session.expires_at <= v_now
       OR v_session.purpose <> 'registration'
       OR v_session.channel <> 'whatsapp'
       OR p_destination_hmac_aliases IS NULL
       OR NOT (v_session.destination_hmac = ANY(p_destination_hmac_aliases)) THEN
        RAISE EXCEPTION 'FlowKirim challenge cannot be reserved for this session'
            USING ERRCODE = '22023';
    END IF;

    FOR v_alias IN
        SELECT DISTINCT alias
          FROM pg_catalog.unnest(p_destination_hmac_aliases) AS aliases(alias)
         ORDER BY alias
    LOOP
        PERFORM pg_catalog.pg_advisory_xact_lock(
            pg_catalog.hashtextextended(
                pg_catalog.encode(v_alias, 'hex') || ':' || v_session.channel,
                0
            )
        );
    END LOOP;

    SELECT count(*), max(created_at)
      INTO v_send_count, v_last_sent_at
      FROM public.otp_delivery_events
     WHERE destination_hmac = ANY(p_destination_hmac_aliases)
       AND channel = v_session.channel
       AND created_at >= v_now - INTERVAL '60 minutes';

    IF v_send_count >= 5
       OR (v_last_sent_at IS NOT NULL AND v_last_sent_at > v_now - INTERVAL '60 seconds') THEN
        RAISE EXCEPTION 'OTP delivery rate limit exceeded'
            USING ERRCODE = 'P0001';
    END IF;

    UPDATE public.verification_challenges
       SET status = 'superseded', superseded_at = v_now
     WHERE verification_session_id = p_session_id
       AND status = 'pending';

    INSERT INTO public.verification_challenges (
        id, verification_session_id, code_hash, hash_key_version,
        provider, expires_at
    )
    VALUES (
        p_challenge_id, p_session_id, NULL, NULL,
        'flowkirim', LEAST(v_now + INTERVAL '10 minutes', v_session.expires_at)
    );

    challenge_id := p_challenge_id;

    INSERT INTO public.otp_delivery_events (
        destination_hmac, hmac_key_version, channel, purpose
    )
    VALUES (
        v_session.destination_hmac, v_session.hmac_key_version,
        v_session.channel, v_session.purpose
    )
    RETURNING id INTO delivery_event_id;

    INSERT INTO public.identity_audit_events (
        actor_type, subject_user_id, action_code, outcome, metadata
    )
    VALUES (
        'system', v_session.user_id, 'identity.verification.delivery_requested', 'success',
        pg_catalog.jsonb_build_object(
            'channel', v_session.channel,
            'purpose', v_session.purpose,
            'provider', 'flowkirim',
            'session_id', p_session_id::text,
            'challenge_id', p_challenge_id::text
        )
    );

    UPDATE public.verification_sessions
       SET resend_available_at = v_now + INTERVAL '60 seconds'
     WHERE id = p_session_id;

    RETURN NEXT;
END;
$$;

CREATE FUNCTION public.bind_flowkirim_otp_request(
    p_session_id UUID,
    p_challenge_id UUID,
    p_provider_request_id TEXT
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_existing_request_id TEXT;
BEGIN
    IF p_provider_request_id IS NULL
       OR length(btrim(p_provider_request_id)) NOT BETWEEN 1 AND 256 THEN
        RAISE EXCEPTION 'FlowKirim request identifier is invalid'
            USING ERRCODE = '22023';
    END IF;

    SELECT provider_request_id
      INTO v_existing_request_id
      FROM public.verification_challenges
     WHERE id = p_challenge_id
       AND verification_session_id = p_session_id
       AND provider = 'flowkirim'
       AND status = 'pending'
       AND expires_at > clock_timestamp()
     FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'pending FlowKirim challenge was not found'
            USING ERRCODE = 'P0002';
    END IF;

    IF v_existing_request_id = p_provider_request_id THEN
        RETURN;
    END IF;

    IF v_existing_request_id IS NOT NULL THEN
        RAISE EXCEPTION 'FlowKirim request identifier is already bound'
            USING ERRCODE = '23514';
    END IF;

    UPDATE public.verification_challenges
       SET provider_request_id = p_provider_request_id
     WHERE id = p_challenge_id
       AND verification_session_id = p_session_id
       AND provider = 'flowkirim'
       AND status = 'pending'
       AND provider_request_id IS NULL;
END;
$$;

CREATE FUNCTION public.get_pending_flowkirim_otp_challenge(p_session_id UUID)
RETURNS TABLE (challenge_id UUID, provider_request_id TEXT, expires_at TIMESTAMPTZ)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    RETURN QUERY
    SELECT c.id, c.provider_request_id, c.expires_at
      FROM public.verification_challenges AS c
      JOIN public.verification_sessions AS s ON s.id = c.verification_session_id
     WHERE c.verification_session_id = p_session_id
       AND c.provider = 'flowkirim'
       AND c.provider_request_id IS NOT NULL
       AND c.status = 'pending'
       AND c.expires_at > clock_timestamp()
       AND s.purpose = 'registration'
       AND s.channel = 'whatsapp'
       AND s.status = 'pending'
       AND s.expires_at > clock_timestamp()
     ORDER BY c.created_at DESC
     LIMIT 1;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'pending FlowKirim challenge was not found'
            USING ERRCODE = 'P0002';
    END IF;
END;
$$;

REVOKE ALL ON FUNCTION public.reserve_flowkirim_otp_challenge(UUID, UUID, BYTEA[]) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.bind_flowkirim_otp_request(UUID, UUID, TEXT) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.get_pending_flowkirim_otp_challenge(UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.get_pending_otp_challenge(UUID) FROM PUBLIC;

COMMENT ON COLUMN public.verification_challenges.provider IS
    'OTP authority: local for backend-generated codes, flowkirim for provider-managed WhatsApp registration codes.';
COMMENT ON COLUMN public.verification_challenges.provider_request_id IS
    'Opaque FlowKirim request identifier; bound once after idempotent provider start and never exposed to clients.';
