-- runtime-provided verification identifiers --

DROP FUNCTION public.create_registration_verification_session(UUID, TEXT, BYTEA, BYTEA, TEXT, BYTEA, TEXT, TIMESTAMPTZ);
DROP FUNCTION public.reserve_otp_challenge(UUID, BYTEA, TEXT, BYTEA[]);

CREATE FUNCTION public.create_registration_verification_session(
    p_session_id UUID,
    p_user_id UUID,
    p_channel TEXT,
    p_destination_ciphertext BYTEA,
    p_destination_nonce BYTEA,
    p_encryption_key_version TEXT,
    p_destination_hmac BYTEA,
    p_hmac_key_version TEXT,
    p_expires_at TIMESTAMPTZ
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_user public.users%ROWTYPE;
BEGIN
    SELECT * INTO v_user
      FROM public.users
     WHERE id = p_user_id
     FOR UPDATE;

    IF NOT FOUND
       OR p_session_id IS NULL
       OR v_user.account_status <> 'pending_verification'
       OR p_channel NOT IN ('email', 'whatsapp')
       OR (p_channel = 'email' AND v_user.initial_contact_channel <> 'email')
       OR (p_channel = 'whatsapp' AND v_user.initial_contact_channel <> 'phone')
       OR p_expires_at <= clock_timestamp()
       OR p_expires_at > v_user.pending_expires_at
       OR p_expires_at > clock_timestamp() + INTERVAL '10 minutes'
       OR p_destination_ciphertext IS NULL OR octet_length(p_destination_ciphertext) = 0
       OR p_destination_nonce IS NULL OR octet_length(p_destination_nonce) = 0
       OR p_destination_hmac IS NULL OR octet_length(p_destination_hmac) = 0
       OR p_encryption_key_version IS NULL OR length(btrim(p_encryption_key_version)) = 0
       OR p_hmac_key_version IS NULL OR length(btrim(p_hmac_key_version)) = 0 THEN
        RAISE EXCEPTION 'registration verification session input is invalid'
            USING ERRCODE = '22023';
    END IF;

    INSERT INTO public.verification_sessions (
        id, user_id, purpose, channel, destination_ciphertext, destination_nonce,
        encryption_key_version, destination_hmac, hmac_key_version, expires_at
    )
    VALUES (
        p_session_id, p_user_id, 'registration', p_channel, p_destination_ciphertext, p_destination_nonce,
        p_encryption_key_version, p_destination_hmac, p_hmac_key_version, p_expires_at
    );

    INSERT INTO public.identity_audit_events (actor_type, action_code, outcome)
    VALUES ('system', 'identity.registration.requested', 'success');

    RETURN p_session_id;
END;
$$;

CREATE FUNCTION public.reserve_otp_challenge(
    p_session_id UUID,
    p_challenge_id UUID,
    p_code_hash BYTEA,
    p_hash_key_version TEXT,
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
       OR (v_session.locked_until IS NOT NULL AND v_session.locked_until > v_now)
       OR p_code_hash IS NULL OR octet_length(p_code_hash) = 0
       OR p_hash_key_version IS NULL OR length(btrim(p_hash_key_version)) = 0
       OR p_destination_hmac_aliases IS NULL
       OR NOT (v_session.destination_hmac = ANY(p_destination_hmac_aliases)) THEN
        RAISE EXCEPTION 'OTP challenge cannot be reserved for this session'
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
        id, verification_session_id, code_hash, hash_key_version, expires_at
    )
    VALUES (
        p_challenge_id, p_session_id, p_code_hash, p_hash_key_version,
        LEAST(v_now + INTERVAL '10 minutes', v_session.expires_at)
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

    UPDATE public.verification_sessions
       SET resend_available_at = v_now + INTERVAL '60 seconds'
     WHERE id = p_session_id;

    RETURN NEXT;
END;
$$;

REVOKE ALL ON FUNCTION public.create_registration_verification_session(UUID, UUID, TEXT, BYTEA, BYTEA, TEXT, BYTEA, TEXT, TIMESTAMPTZ) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.reserve_otp_challenge(UUID, UUID, BYTEA, TEXT, BYTEA[]) FROM PUBLIC;

COMMENT ON FUNCTION public.create_registration_verification_session(UUID, UUID, TEXT, BYTEA, BYTEA, TEXT, BYTEA, TEXT, TIMESTAMPTZ) IS
    'Membuat sesi OTP registrasi dengan UUID dari runtime sebelum enkripsi tujuan.';
COMMENT ON FUNCTION public.reserve_otp_challenge(UUID, UUID, BYTEA, TEXT, BYTEA[]) IS
    'Merotasi challenge dengan UUID dari runtime sebelum keyed OTP digest dihitung.';
