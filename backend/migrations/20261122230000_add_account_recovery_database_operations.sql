
-- Account recovery verification and account-selection database operations.
-- OTP throttling and challenge mechanics reuse the existing verification tables/functions.

CREATE INDEX verification_sessions_recovery_contact_created_idx
    ON public.verification_sessions (channel, destination_hmac, created_at DESC)
    WHERE purpose = 'account_recovery';

CREATE FUNCTION public.create_account_recovery_verification_session(
    p_session_id UUID,
    p_case_id UUID,
    p_requested_action TEXT,
    p_channel TEXT,
    p_destination_ciphertext BYTEA,
    p_destination_nonce BYTEA,
    p_encryption_key_version TEXT,
    p_destination_hmac BYTEA,
    p_hmac_key_version TEXT,
    p_expires_at TIMESTAMPTZ
)
RETURNS TABLE (session_id UUID, case_id UUID)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_now TIMESTAMPTZ := clock_timestamp();
    v_recent_requests BIGINT;
BEGIN
    IF p_session_id IS NULL OR p_case_id IS NULL
       OR p_requested_action NOT IN ('password_reset', 'forgot_email', 'account_access')
       OR p_channel NOT IN ('email', 'whatsapp')
       OR p_expires_at <= v_now
       OR p_expires_at > v_now + INTERVAL '10 minutes'
       OR p_destination_ciphertext IS NULL OR octet_length(p_destination_ciphertext) = 0
       OR p_destination_nonce IS NULL OR octet_length(p_destination_nonce) = 0
       OR p_destination_hmac IS NULL OR octet_length(p_destination_hmac) <> 32
       OR p_encryption_key_version IS NULL OR length(btrim(p_encryption_key_version)) = 0
       OR p_hmac_key_version IS NULL OR length(btrim(p_hmac_key_version)) = 0 THEN
        RAISE EXCEPTION 'account recovery verification input is invalid'
            USING ERRCODE = '22023';
    END IF;

    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(
            pg_catalog.encode(p_destination_hmac, 'hex') || ':' || p_channel,
            0
        )
    );

    SELECT count(*) INTO v_recent_requests
      FROM public.verification_sessions
     WHERE purpose = 'account_recovery'
       AND channel = p_channel
       AND destination_hmac = p_destination_hmac
       AND created_at >= v_now - INTERVAL '60 minutes';

    IF v_recent_requests >= 5 THEN
        RAISE EXCEPTION 'account recovery request rate limit exceeded'
            USING ERRCODE = 'P0001';
    END IF;

    INSERT INTO public.verification_sessions (
        id, user_id, purpose, channel, destination_ciphertext, destination_nonce,
        encryption_key_version, destination_hmac, hmac_key_version, expires_at
    )
    VALUES (
        p_session_id, NULL, 'account_recovery', p_channel, p_destination_ciphertext,
        p_destination_nonce, p_encryption_key_version, p_destination_hmac,
        p_hmac_key_version, p_expires_at
    );

    INSERT INTO public.account_recovery_cases (
        id, verification_session_id, target_user_id, requested_action, status
    )
    VALUES (
        p_case_id, p_session_id, NULL, p_requested_action, 'pending_verification'
    );

    INSERT INTO public.identity_audit_events (
        actor_type, action_code, outcome, metadata
    )
    VALUES (
        'system', 'identity.recovery.requested', 'success',
        pg_catalog.jsonb_build_object(
            'channel', p_channel,
            'purpose', 'account_recovery',
            'session_id', p_session_id::text,
            'recovery_case_id', p_case_id::text,
        )
    );

    session_id := p_session_id;
    case_id := p_case_id;
    RETURN NEXT;
END;
$$;

CREATE FUNCTION public.get_account_recovery_verification_session(p_session_id UUID)
RETURNS TABLE (
    channel TEXT,
    destination_ciphertext BYTEA,
    destination_nonce BYTEA,
    encryption_key_version TEXT,
    destination_hmac BYTEA,
    hmac_key_version TEXT,
    expires_at TIMESTAMPTZ
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    RETURN QUERY
    SELECT s.channel, s.destination_ciphertext, s.destination_nonce,
           s.encryption_key_version, s.destination_hmac, s.hmac_key_version,
           s.expires_at
      FROM public.verification_sessions AS s
     WHERE s.id = p_session_id
       AND s.purpose = 'account_recovery'
       AND s.status = 'pending'
       AND s.expires_at > clock_timestamp();

    IF NOT FOUND THEN
        RAISE EXCEPTION 'pending account recovery verification session was not found'
            USING ERRCODE = 'P0002';
    END IF;
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
      JOIN public.verification_sessions AS s
        ON s.id = c.verification_session_id
     WHERE c.verification_session_id = p_session_id
       AND c.status = 'pending'
       AND c.expires_at > clock_timestamp()
       AND s.purpose IN ('registration', 'account_recovery')
       AND s.status = 'pending'
       AND s.expires_at > clock_timestamp()
     ORDER BY c.created_at DESC
     LIMIT 1;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'pending OTP challenge was not found'
            USING ERRCODE = 'P0002';
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION public.reserve_otp_challenge(
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
       OR v_session.purpose NOT IN ('registration', 'account_recovery')
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

    INSERT INTO public.identity_audit_events (
        actor_type, subject_user_id, action_code, outcome, metadata
    )
    VALUES (
        'system', v_session.user_id, 'identity.verification.delivery_requested', 'success',
        pg_catalog.jsonb_build_object(
            'channel', v_session.channel,
            'purpose', v_session.purpose,
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

CREATE OR REPLACE FUNCTION public.record_otp_verification_attempt(
    p_session_id UUID,
    p_challenge_id UUID,
    p_code_matches BOOLEAN,
    p_destination_hmac_aliases BYTEA[]
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_session public.verification_sessions%ROWTYPE;
    v_challenge public.verification_challenges%ROWTYPE;
    v_now TIMESTAMPTZ := clock_timestamp();
    v_contact_failures BIGINT;
    v_alias BYTEA;
BEGIN
    SELECT * INTO v_session
      FROM public.verification_sessions
     WHERE id = p_session_id
     FOR UPDATE;

    IF NOT FOUND OR v_session.status <> 'pending'
       OR v_session.purpose NOT IN ('registration', 'account_recovery')
       OR v_session.expires_at <= v_now
       OR (v_session.locked_until IS NOT NULL AND v_session.locked_until > v_now)
       OR p_code_matches IS NULL
       OR p_destination_hmac_aliases IS NULL
       OR NOT (v_session.destination_hmac = ANY(p_destination_hmac_aliases)) THEN
        RAISE EXCEPTION 'OTP verification session is not eligible'
            USING ERRCODE = 'P0001';
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

    IF v_session.failed_attempt_count = 5
       AND v_session.locked_until <= v_now THEN
        UPDATE public.verification_sessions
           SET failed_attempt_count = 0, locked_until = NULL
         WHERE id = p_session_id;
    END IF;

    SELECT * INTO v_challenge
      FROM public.verification_challenges
     WHERE id = p_challenge_id
       AND verification_session_id = p_session_id
     FOR UPDATE;

    IF NOT FOUND OR v_challenge.status <> 'pending'
       OR v_challenge.expires_at <= v_now THEN
        RAISE EXCEPTION 'OTP challenge is not eligible'
            USING ERRCODE = 'P0001';
    END IF;

    SELECT count(*) INTO v_contact_failures
      FROM public.otp_verification_attempt_events
     WHERE destination_hmac = ANY(p_destination_hmac_aliases)
       AND channel = v_session.channel
       AND outcome = 'failed'
       AND attempted_at >= v_now - INTERVAL '60 minutes';

    IF v_contact_failures >= 5 THEN
        RAISE EXCEPTION 'OTP contact verification rate limit exceeded'
            USING ERRCODE = 'P0001';
    END IF;

    INSERT INTO public.otp_verification_attempt_events (
        verification_session_id, verification_challenge_id,
        destination_hmac, hmac_key_version, channel, purpose, outcome
    )
    VALUES (
        p_session_id, p_challenge_id,
        v_session.destination_hmac, v_session.hmac_key_version,
        v_session.channel, v_session.purpose,
        CASE WHEN p_code_matches THEN 'succeeded' ELSE 'failed' END
    );

    IF NOT p_code_matches THEN
        UPDATE public.verification_sessions
           SET failed_attempt_count = failed_attempt_count + 1
         WHERE id = p_session_id;

        INSERT INTO public.identity_audit_events (
            actor_type, subject_user_id, action_code, outcome, reason_code, metadata
        )
        VALUES (
            'system', v_session.user_id, 'identity.verification.attempt_failed',
            'failure', 'invalid_code',
            pg_catalog.jsonb_build_object(
                'channel', v_session.channel,
                'purpose', v_session.purpose,
                'session_id', p_session_id::text,
                'challenge_id', p_challenge_id::text
            )
        );
        RETURN FALSE;
    END IF;

    UPDATE public.verification_challenges
       SET status = 'consumed', consumed_at = v_now
     WHERE id = p_challenge_id;

    UPDATE public.verification_sessions
       SET status = 'verified', verified_at = v_now
     WHERE id = p_session_id;

    IF v_session.purpose = 'registration' THEN
        UPDATE public.users
           SET account_status = 'active',
               email_verified_at = CASE WHEN v_session.channel = 'email' THEN v_now ELSE email_verified_at END,
               phone_verified_at = CASE WHEN v_session.channel = 'whatsapp' THEN v_now ELSE phone_verified_at END,
               initial_contact_verified_at = v_now,
               pending_expires_at = NULL,
               status_changed_at = v_now,
               updated_at = v_now
         WHERE id = v_session.user_id
           AND account_status = 'pending_verification'
           AND pending_expires_at > v_now;

        IF NOT FOUND THEN
            RAISE EXCEPTION 'pending account cannot be activated'
                USING ERRCODE = 'P0001';
        END IF;

        INSERT INTO public.identity_audit_events (
            actor_type, subject_user_id, action_code, outcome, metadata
        )
        VALUES (
            'system', v_session.user_id, 'identity.registration.verified', 'success',
            pg_catalog.jsonb_build_object(
                'channel', v_session.channel,
                'purpose', v_session.purpose,
                'session_id', p_session_id::text,
                'challenge_id', p_challenge_id::text
            )
        );
    ELSE
        UPDATE public.account_recovery_cases
           SET status = 'account_selection_required'
         WHERE verification_session_id = p_session_id
           AND status = 'pending_verification';
        IF NOT FOUND THEN
            RAISE EXCEPTION 'account recovery case is not eligible after contact verification'
                USING ERRCODE = 'P0001';
        END IF;
        INSERT INTO public.identity_audit_events (
            actor_type, action_code, outcome, metadata
        )
        VALUES (
            'system', 'identity.recovery.contact_verified', 'success',
            pg_catalog.jsonb_build_object(
                'channel', v_session.channel,
                'purpose', v_session.purpose,
                'session_id', p_session_id::text,
                'challenge_id', p_challenge_id::text
            )
        );
    END IF;

    RETURN TRUE;
END;
$$;

CREATE FUNCTION public.bind_account_recovery_session_to_account(
    p_case_id UUID,
    p_target_user_id UUID,
    p_destination_hmac_aliases BYTEA[]
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_case public.account_recovery_cases%ROWTYPE;
    v_session public.verification_sessions%ROWTYPE;
BEGIN
    IF p_case_id IS NULL OR p_target_user_id IS NULL
       OR p_destination_hmac_aliases IS NULL
       OR cardinality(p_destination_hmac_aliases) = 0
       OR EXISTS (
           SELECT 1 FROM pg_catalog.unnest(p_destination_hmac_aliases) AS h(value)
           WHERE h.value IS NULL OR octet_length(h.value) <> 32
       ) THEN
        RAISE EXCEPTION 'account recovery selection is not eligible'
            USING ERRCODE = '22023';
    END IF;

    SELECT * INTO v_case
      FROM public.account_recovery_cases
     WHERE id = p_case_id
     FOR UPDATE;

    IF NOT FOUND OR v_case.status <> 'account_selection_required'
       OR v_case.target_user_id IS NOT NULL THEN
        RAISE EXCEPTION 'account recovery selection is not eligible'
            USING ERRCODE = 'P0001';
    END IF;

    SELECT * INTO v_session
      FROM public.verification_sessions
     WHERE id = v_case.verification_session_id
     FOR UPDATE;

    IF NOT FOUND OR v_session.purpose <> 'account_recovery'
       OR v_session.status <> 'verified'
       OR v_session.user_id IS NOT NULL
       OR v_session.expires_at <= clock_timestamp()
       OR NOT EXISTS (
           SELECT 1
             FROM public.recovery_contacts AS rc
             JOIN public.users AS u ON u.id = rc.user_id
            WHERE rc.user_id = p_target_user_id
              AND rc.channel = v_session.channel
              AND rc.contact_hmac = ANY(p_destination_hmac_aliases)
              AND rc.revoked_at IS NULL
              AND u.account_status = 'active'
            FOR SHARE OF rc, u
       ) THEN
        RAISE EXCEPTION 'account recovery selection is not eligible'
            USING ERRCODE = 'P0001';
    END IF;

    UPDATE public.verification_sessions
       SET user_id = p_target_user_id
     WHERE id = v_session.id
       AND user_id IS NULL
       AND status = 'verified';

    IF NOT FOUND THEN
        RAISE EXCEPTION 'account recovery selection is not eligible'
            USING ERRCODE = 'P0001';
    END IF;

    UPDATE public.account_recovery_cases
       SET target_user_id = p_target_user_id,
           status = 'under_review'
     WHERE id = p_case_id
       AND target_user_id IS NULL
       AND status = 'account_selection_required';

    IF NOT FOUND THEN
        RAISE EXCEPTION 'account recovery selection is not eligible'
            USING ERRCODE = 'P0001';
    END IF;

    INSERT INTO public.identity_audit_events (
        actor_type, subject_user_id, action_code, outcome, metadata
    )
    VALUES (
        'system', p_target_user_id, 'identity.recovery.account_selected', 'success',
        pg_catalog.jsonb_build_object(
            'recovery_case_id', p_case_id::text,
            'channel', v_session.channel,
        )
    );

    RETURN p_target_user_id;
END;
$$;

REVOKE ALL ON FUNCTION public.create_account_recovery_verification_session(UUID, UUID, TEXT, TEXT, BYTEA, BYTEA, TEXT, BYTEA, TEXT, TIMESTAMPTZ) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.get_account_recovery_verification_session(UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.get_pending_otp_challenge(UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.reserve_otp_challenge(UUID, UUID, BYTEA, TEXT, BYTEA[]) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.record_otp_verification_attempt(UUID, UUID, BOOLEAN, BYTEA[]) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.bind_account_recovery_session_to_account(UUID, UUID, BYTEA[]) FROM PUBLIC;

COMMENT ON FUNCTION public.create_account_recovery_verification_session(UUID, UUID, TEXT, TEXT, BYTEA, BYTEA, TEXT, BYTEA, TEXT, TIMESTAMPTZ) IS
    'Creates an unbound account-recovery verification session and pending case, limited to five requests per contact per hour.';
COMMENT ON FUNCTION public.bind_account_recovery_session_to_account(UUID, UUID, BYTEA[]) IS
    'Atomically binds a verified recovery session only to an active account owning a matching verified recovery contact.';
