-- Account recovery password reset token issuance and atomic password replacement.

ALTER TABLE public.password_reset_tokens
    DROP CONSTRAINT password_reset_tokens_terminal_check;

ALTER TABLE public.password_reset_tokens
    ADD CONSTRAINT password_reset_tokens_terminal_check
    CHECK (
        NOT (consumed_at IS NOT NULL AND revoked_at IS NOT NULL)
        AND (
            revoked_at IS NULL
            OR (
                revocation_reason IS NOT NULL
                AND revocation_reason IN (
                    'superseded',
                    'security_event',
                    'account_status_changed',
                    'admin_action',
                    'expired',
                    'delivery_failed'
                )
            )
        )
        AND (revocation_reason IS NULL OR revoked_at IS NOT NULL)
    );

CREATE FUNCTION public.get_approved_account_recovery_delivery_context(
    p_case_id UUID
)
RETURNS TABLE (
    session_id UUID,
    channel TEXT,
    destination_ciphertext BYTEA,
    destination_nonce BYTEA,
    encryption_key_version TEXT,
    destination_hmac BYTEA,
    hmac_key_version TEXT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_target_user_id UUID;
    v_verification_session_id UUID;
    v_channel TEXT;
    v_destination_ciphertext BYTEA;
    v_destination_nonce BYTEA;
    v_encryption_key_version TEXT;
    v_destination_hmac BYTEA;
    v_hmac_key_version TEXT;
BEGIN
    IF p_case_id IS NULL THEN
        RAISE EXCEPTION 'recovery case identifier is required'
            USING ERRCODE = '22023';
    END IF;

    SELECT c.target_user_id, c.verification_session_id
      INTO v_target_user_id, v_verification_session_id
      FROM public.account_recovery_cases AS c
      JOIN public.users AS u ON u.id = c.target_user_id
     WHERE c.id = p_case_id
       AND c.status = 'approved'
       AND c.requested_action = 'password_reset'
       AND c.target_user_id IS NOT NULL
       AND u.account_status = 'active';

    IF NOT FOUND THEN
        RAISE EXCEPTION 'approved password recovery case is unavailable'
            USING ERRCODE = 'P0002';
    END IF;

    SELECT s.channel,
           s.destination_ciphertext,
           s.destination_nonce,
           s.encryption_key_version,
           s.destination_hmac,
           s.hmac_key_version
      INTO v_channel,
           v_destination_ciphertext,
           v_destination_nonce,
           v_encryption_key_version,
           v_destination_hmac,
           v_hmac_key_version
      FROM public.verification_sessions AS s
     WHERE s.id = v_verification_session_id
       AND s.purpose = 'account_recovery'
       AND s.status = 'verified'
       AND s.user_id = v_target_user_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'verified recovery contact context is unavailable'
            USING ERRCODE = 'P0002';
    END IF;

    RETURN QUERY
    SELECT v_verification_session_id,
           v_channel,
           v_destination_ciphertext,
           v_destination_nonce,
           v_encryption_key_version,
           v_destination_hmac,
           v_hmac_key_version;
END;
$$;

CREATE FUNCTION public.create_password_reset_token(
    p_case_id UUID,
    p_token_hash BYTEA,
    p_destination_hmac_aliases BYTEA[]
)
RETURNS TABLE (expires_at_unix BIGINT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_target_user_id UUID;
    v_verification_session_id UUID;
    v_case_status TEXT;
    v_requested_action TEXT;
    v_account_status TEXT;
    v_channel TEXT;
    v_recent_issuances BIGINT;
    v_now TIMESTAMPTZ;
    v_expires_at TIMESTAMPTZ;
BEGIN
    IF p_case_id IS NULL
       OR p_token_hash IS NULL
       OR octet_length(p_token_hash) <> 32
       OR p_destination_hmac_aliases IS NULL
       OR cardinality(p_destination_hmac_aliases) < 1
       OR cardinality(p_destination_hmac_aliases) > 8
       OR EXISTS (
            SELECT 1
              FROM unnest(p_destination_hmac_aliases) AS aliases(digest)
             WHERE octet_length(aliases.digest) <> 32
       ) THEN
        RAISE EXCEPTION 'password reset token issuance input is invalid'
            USING ERRCODE = '22023';
    END IF;

    SELECT target_user_id, verification_session_id, status, requested_action
      INTO v_target_user_id, v_verification_session_id, v_case_status, v_requested_action
      FROM public.account_recovery_cases
     WHERE id = p_case_id
     FOR UPDATE;

    IF NOT FOUND
       OR v_case_status <> 'approved'
       OR v_requested_action <> 'password_reset'
       OR v_target_user_id IS NULL THEN
        RAISE EXCEPTION 'recovery case is not approved for password reset'
            USING ERRCODE = 'P0002';
    END IF;

    SELECT account_status
      INTO v_account_status
      FROM public.users
     WHERE id = v_target_user_id
     FOR UPDATE;

    IF NOT FOUND OR v_account_status <> 'active' THEN
        RAISE EXCEPTION 'password reset target account is not active'
            USING ERRCODE = 'P0002';
    END IF;

    SELECT channel
      INTO v_channel
      FROM public.verification_sessions
     WHERE id = v_verification_session_id
       AND purpose = 'account_recovery'
       AND status = 'verified'
       AND user_id = v_target_user_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'verified recovery contact context is unavailable'
            USING ERRCODE = 'P0002';
    END IF;

    PERFORM 1
      FROM public.recovery_contacts
     WHERE user_id = v_target_user_id
       AND channel = v_channel
       AND contact_hmac = ANY (p_destination_hmac_aliases)
       AND revoked_at IS NULL
     FOR SHARE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'verified recovery contact is no longer associated with the account'
            USING ERRCODE = 'P0002';
    END IF;

    UPDATE public.password_reset_tokens
       SET revoked_at = clock_timestamp(),
           revocation_reason = 'expired'
     WHERE user_id = v_target_user_id
       AND consumed_at IS NULL
       AND revoked_at IS NULL
       AND expires_at <= clock_timestamp();

    SELECT count(*)
      INTO v_recent_issuances
      FROM public.password_reset_tokens
     WHERE user_id = v_target_user_id
       AND created_at > clock_timestamp() - INTERVAL '1 hour';

    IF v_recent_issuances >= 3 THEN
        RAISE EXCEPTION 'password reset token issuance limit reached'
            USING ERRCODE = 'P0001';
    END IF;

    IF EXISTS (
        SELECT 1
          FROM public.password_reset_tokens
         WHERE user_id = v_target_user_id
           AND consumed_at IS NULL
           AND revoked_at IS NULL
           AND expires_at > clock_timestamp()
    ) THEN
        RAISE EXCEPTION 'an active password reset token already exists'
            USING ERRCODE = 'P0001';
    END IF;

    v_now := clock_timestamp();
    v_expires_at := v_now + INTERVAL '15 minutes';

    INSERT INTO public.password_reset_tokens (
        recovery_case_id, user_id, token_hash, expires_at
    )
    VALUES (
        p_case_id, v_target_user_id, p_token_hash, v_expires_at
    );

    INSERT INTO public.identity_audit_events (
        actor_type, subject_user_id, action_code, outcome, reason_code, metadata
    )
    VALUES (
        'system',
        v_target_user_id,
        'identity.recovery.password_reset_token.issued',
        'success',
        'account_recovery',
        pg_catalog.jsonb_build_object(
            'purpose', 'account_recovery',
            'recovery_case_id', p_case_id::text,
            'channel', v_channel
        )
    );

    RETURN QUERY SELECT floor(extract(epoch FROM v_expires_at))::BIGINT;
END;
$$;

CREATE FUNCTION public.revoke_password_reset_token(
    p_token_hash BYTEA
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_user_id UUID;
    v_case_id UUID;
BEGIN
    IF p_token_hash IS NULL OR octet_length(p_token_hash) <> 32 THEN
        RAISE EXCEPTION 'password reset token hash is invalid'
            USING ERRCODE = '22023';
    END IF;

    UPDATE public.password_reset_tokens
       SET revoked_at = clock_timestamp(),
           revocation_reason = 'delivery_failed'
     WHERE token_hash = p_token_hash
       AND consumed_at IS NULL
       AND revoked_at IS NULL
    RETURNING user_id, recovery_case_id
         INTO v_user_id, v_case_id;

    IF NOT FOUND THEN
        RETURN FALSE;
    END IF;

    INSERT INTO public.identity_audit_events (
        actor_type, subject_user_id, action_code, outcome, reason_code, metadata
    )
    VALUES (
        'system',
        v_user_id,
        'identity.recovery.password_reset_delivery.failed',
        'failure',
        'delivery_failed',
        pg_catalog.jsonb_build_object(
            'purpose', 'account_recovery',
            'recovery_case_id', v_case_id::text
        )
    );

    RETURN TRUE;
END;
$$;

CREATE FUNCTION public.complete_password_reset(
    p_token_hash BYTEA,
    p_password_hash TEXT
)
RETURNS TABLE (revoked_session_count INTEGER)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_case_id UUID;
    v_user_id UUID;
    v_case_target_user_id UUID;
    v_case_status TEXT;
    v_requested_action TEXT;
    v_account_status TEXT;
    v_token_id UUID;
    v_revoked_session_count INTEGER;
    v_count_bucket TEXT;
BEGIN
    IF p_token_hash IS NULL
       OR octet_length(p_token_hash) <> 32
       OR p_password_hash IS NULL
       OR length(p_password_hash) > 512
       OR p_password_hash <> btrim(p_password_hash)
       OR p_password_hash !~ '^\$argon2id\$v=19\$m=19456,t=2,p=1\$[A-Za-z0-9+/=]+\$[A-Za-z0-9+/=]+$' THEN
        RAISE EXCEPTION 'password reset input is invalid'
            USING ERRCODE = '22023';
    END IF;

    SELECT recovery_case_id, user_id
      INTO v_case_id, v_user_id
      FROM public.password_reset_tokens
     WHERE token_hash = p_token_hash;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'password reset token is invalid or expired'
            USING ERRCODE = 'P0002';
    END IF;

    SELECT target_user_id, status, requested_action
      INTO v_case_target_user_id, v_case_status, v_requested_action
      FROM public.account_recovery_cases
     WHERE id = v_case_id
     FOR UPDATE;

    IF NOT FOUND
       OR v_case_status <> 'approved'
       OR v_requested_action <> 'password_reset'
       OR v_case_target_user_id IS DISTINCT FROM v_user_id THEN
        RAISE EXCEPTION 'password reset token is invalid or expired'
            USING ERRCODE = 'P0002';
    END IF;

    SELECT account_status
      INTO v_account_status
      FROM public.users
     WHERE id = v_user_id
     FOR UPDATE;

    IF NOT FOUND OR v_account_status <> 'active' THEN
        RAISE EXCEPTION 'password reset token is invalid or expired'
            USING ERRCODE = 'P0002';
    END IF;

    SELECT id
      INTO v_token_id
      FROM public.password_reset_tokens
     WHERE token_hash = p_token_hash
       AND recovery_case_id = v_case_id
       AND user_id = v_user_id
       AND consumed_at IS NULL
       AND revoked_at IS NULL
       AND expires_at > clock_timestamp()
     FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'password reset token is invalid or expired'
            USING ERRCODE = 'P0002';
    END IF;

    UPDATE public.password_reset_tokens
       SET consumed_at = clock_timestamp()
     WHERE id = v_token_id;

    UPDATE public.users
       SET password_hash = p_password_hash
     WHERE id = v_user_id;

    UPDATE public.sessions
       SET revoked_at = clock_timestamp(),
           revocation_reason = 'password_reset'
     WHERE user_id = v_user_id
       AND revoked_at IS NULL;

    GET DIAGNOSTICS v_revoked_session_count = ROW_COUNT;

    UPDATE public.password_reset_tokens
       SET revoked_at = clock_timestamp(),
           revocation_reason = 'security_event'
     WHERE user_id = v_user_id
       AND id <> v_token_id
       AND consumed_at IS NULL
       AND revoked_at IS NULL;

    UPDATE public.account_recovery_cases
       SET status = 'completed'
     WHERE id = v_case_id;

    v_count_bucket := CASE
        WHEN v_revoked_session_count = 0 THEN '0'
        WHEN v_revoked_session_count = 1 THEN '1'
        WHEN v_revoked_session_count <= 5 THEN '2-5'
        ELSE '6+'
    END;

    INSERT INTO public.identity_audit_events (
        actor_type, actor_id, subject_user_id, action_code, outcome, reason_code, metadata
    )
    VALUES (
        'user',
        v_user_id,
        v_user_id,
        'identity.password.reset.completed',
        'success',
        'account_recovery',
        pg_catalog.jsonb_build_object(
            'purpose', 'account_recovery',
            'recovery_case_id', v_case_id::text,
            'count_bucket', v_count_bucket
        )
    );

    RETURN QUERY SELECT v_revoked_session_count;
END;
$$;

REVOKE ALL ON FUNCTION public.get_approved_account_recovery_delivery_context(UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.create_password_reset_token(UUID, BYTEA, BYTEA[]) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.revoke_password_reset_token(BYTEA) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.complete_password_reset(BYTEA, TEXT) FROM PUBLIC;

COMMENT ON FUNCTION public.get_approved_account_recovery_delivery_context(UUID) IS
    'Returns encrypted verified contact data only for an approved password-reset recovery case and active target account.';
COMMENT ON FUNCTION public.create_password_reset_token(UUID, BYTEA, BYTEA[]) IS
    'Issues one short-lived token for an approved recovery case after rechecking the active recovery contact and account-level issuance limits.';
COMMENT ON FUNCTION public.revoke_password_reset_token(BYTEA) IS
    'Revokes an undeliverable reset token and records a security audit event.';
COMMENT ON FUNCTION public.complete_password_reset(BYTEA, TEXT) IS
    'Atomically consumes a reset token, replaces the Argon2id password hash, revokes all sessions, completes recovery, and appends an audit event.';
