-- Controlled operator-only enrollment and recovery for the single Admin TOTP credential.
-- The application principal receives no EXECUTE grant for these operator procedures.

CREATE OR REPLACE FUNCTION public.guard_admin_totp_credential()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
DECLARE
    v_role TEXT;
    v_status TEXT;
    v_operator_reset BOOLEAN;
BEGIN
    IF TG_OP = 'INSERT' THEN
        SELECT primary_role, account_status
          INTO v_role, v_status
          FROM public.users
         WHERE id = NEW.user_id
         FOR SHARE;

        IF NOT FOUND OR v_role <> 'admin' OR v_status <> 'active' THEN
            RAISE EXCEPTION 'TOTP credentials require the internally provisioned active Admin account'
                USING ERRCODE = '23514';
        END IF;

        NEW.created_at := clock_timestamp();
        NEW.updated_at := NEW.created_at;
        RETURN NEW;
    END IF;

    IF NEW.user_id IS DISTINCT FROM OLD.user_id
       OR NEW.created_at IS DISTINCT FROM OLD.created_at THEN
        RAISE EXCEPTION 'Admin TOTP credential identity is immutable'
            USING ERRCODE = '23514';
    END IF;

    SELECT primary_role, account_status
      INTO v_role, v_status
      FROM public.users
     WHERE id = NEW.user_id
     FOR SHARE;

    IF NOT FOUND OR v_role <> 'admin' OR v_status <> 'active' THEN
        RAISE EXCEPTION 'Admin TOTP credentials require an active Admin account'
            USING ERRCODE = '23514';
    END IF;

    v_operator_reset := current_user = 'usahapintar_migrator'
        AND COALESCE(pg_catalog.current_setting('usahapintar.admin_totp_reset', TRUE) = 'on', FALSE);

    IF NOT v_operator_reset
       AND OLD.enabled_at IS NOT NULL
       AND (
           NEW.enabled_at IS DISTINCT FROM OLD.enabled_at
           OR NEW.secret_ciphertext IS DISTINCT FROM OLD.secret_ciphertext
           OR NEW.secret_nonce IS DISTINCT FROM OLD.secret_nonce
           OR NEW.encryption_key_version IS DISTINCT FROM OLD.encryption_key_version
       ) THEN
        RAISE EXCEPTION 'enabled Admin TOTP secret and enrollment cannot be replaced directly'
            USING ERRCODE = '23514';
    END IF;

    IF NOT v_operator_reset
       AND OLD.last_accepted_step IS NOT NULL
       AND (
           NEW.last_accepted_step IS NULL
           OR NEW.last_accepted_step < OLD.last_accepted_step
       ) THEN
        RAISE EXCEPTION 'Admin TOTP replay counter cannot move backwards or be cleared'
            USING ERRCODE = '23514';
    END IF;

    NEW.updated_at := clock_timestamp();
    RETURN NEW;
END;
$$;

CREATE FUNCTION public.revoke_admin_authentication_when_inactive()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $
BEGIN
    IF OLD.primary_role = 'admin'
       AND OLD.account_status = 'active'
       AND NEW.account_status <> 'active' THEN
        UPDATE public.admin_login_challenges
           SET consumed_at = clock_timestamp()
         WHERE user_id = NEW.id
           AND consumed_at IS NULL;

        UPDATE public.admin_sessions
           SET revoked_at = clock_timestamp(),
               revocation_reason = 'account_status_changed'
         WHERE user_id = NEW.id
           AND revoked_at IS NULL;

        INSERT INTO public.identity_audit_events (
            actor_type, subject_user_id, action_code, outcome, reason_code, metadata
        )
        VALUES (
            'system', NEW.id, 'identity.admin.access.revoked',
            'success', 'account_status_changed',
            pg_catalog.jsonb_build_object(
                'purpose', 'admin_login',
                'role', 'admin',
                'policy_code', 'inactive_admin_account'
            )
        );
    END IF;

    RETURN NEW;
END;
$;

CREATE TRIGGER users_30_revoke_admin_authentication_when_inactive
    AFTER UPDATE OF account_status ON public.users
    FOR EACH ROW
    WHEN (OLD.primary_role = 'admin' AND OLD.account_status IS DISTINCT FROM NEW.account_status)
    EXECUTE FUNCTION public.revoke_admin_authentication_when_inactive();

REVOKE ALL ON FUNCTION public.revoke_admin_authentication_when_inactive() FROM PUBLIC;

CREATE FUNCTION public.activate_admin_totp_enrollment(
    p_user_id UUID,
    p_verified_step BIGINT,
    p_operator_id UUID
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_enabled_user_id UUID;
BEGIN
    IF p_user_id IS NULL OR p_verified_step IS NULL OR p_verified_step < 0
       OR p_operator_id IS NULL THEN
        RAISE EXCEPTION 'Admin TOTP enrollment input is invalid'
            USING ERRCODE = '22023';
    END IF;

    UPDATE public.admin_totp_credentials AS c
       SET enabled_at = clock_timestamp(),
           last_accepted_step = p_verified_step
      FROM public.users AS u
     WHERE c.user_id = p_user_id
       AND u.id = c.user_id
       AND u.primary_role = 'admin'
       AND u.account_status = 'active'
       AND c.enabled_at IS NULL
       AND c.last_accepted_step IS NULL
    RETURNING c.user_id INTO v_enabled_user_id;

    IF v_enabled_user_id IS NULL THEN
        RAISE EXCEPTION 'Admin TOTP enrollment is unavailable or already enabled'
            USING ERRCODE = 'P0002';
    END IF;

    INSERT INTO public.identity_audit_events (
        actor_type, actor_id, subject_user_id, action_code, outcome, reason_code, metadata
    )
    VALUES (
        'operator', p_operator_id, p_user_id, 'identity.admin.totp.enrolled',
        'success', 'operator_enrollment',
        pg_catalog.jsonb_build_object(
            'purpose', 'admin_totp',
            'role', 'admin',
            'policy_code', 'operator_verified_enrollment'
        )
    );

    RETURN TRUE;
END;
$$;

CREATE FUNCTION public.reset_admin_totp_enrollment(
    p_user_id UUID,
    p_secret_ciphertext BYTEA,
    p_secret_nonce BYTEA,
    p_encryption_key_version TEXT,
    p_reason_code TEXT,
    p_operator_id UUID
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_reset_user_id UUID;
BEGIN
    IF p_user_id IS NULL
       OR p_secret_ciphertext IS NULL
       OR octet_length(p_secret_ciphertext) < 32
       OR p_secret_nonce IS NULL
       OR octet_length(p_secret_nonce) <> 12
       OR p_encryption_key_version IS NULL
       OR p_encryption_key_version !~ '^v[1-9][0-9]{0,9}$'
       OR p_reason_code IS NULL
       OR p_reason_code NOT IN ('authenticator_lost', 'suspected_compromise', 'operator_recovery')
       OR p_operator_id IS NULL THEN
        RAISE EXCEPTION 'Admin TOTP reset input is invalid'
            USING ERRCODE = '22023';
    END IF;

    PERFORM pg_catalog.set_config('usahapintar.admin_totp_reset', 'on', TRUE);

    UPDATE public.admin_totp_credentials AS c
       SET secret_ciphertext = p_secret_ciphertext,
           secret_nonce = p_secret_nonce,
           encryption_key_version = p_encryption_key_version,
           enabled_at = NULL,
           last_accepted_step = NULL
      FROM public.users AS u
     WHERE c.user_id = p_user_id
       AND u.id = c.user_id
       AND u.primary_role = 'admin'
       AND u.account_status = 'active'
       AND c.enabled_at IS NOT NULL
    RETURNING c.user_id INTO v_reset_user_id;

    IF v_reset_user_id IS NULL THEN
        RAISE EXCEPTION 'active enrolled Admin TOTP credential was not found'
            USING ERRCODE = 'P0002';
    END IF;

    UPDATE public.admin_login_challenges
       SET consumed_at = clock_timestamp()
     WHERE user_id = p_user_id
       AND consumed_at IS NULL;

    UPDATE public.admin_sessions
       SET revoked_at = clock_timestamp(),
           revocation_reason = 'operator_action'
     WHERE user_id = p_user_id
       AND revoked_at IS NULL;

    INSERT INTO public.identity_audit_events (
        actor_type, actor_id, subject_user_id, action_code, outcome, reason_code, metadata
    )
    VALUES (
        'operator', p_operator_id, p_user_id, 'identity.admin.totp.reset',
        'success', p_reason_code,
        pg_catalog.jsonb_build_object(
            'purpose', 'admin_totp',
            'role', 'admin',
            'policy_code', 'operator_controlled_reenrollment'
        )
    );

    RETURN TRUE;
END;
$$;

REVOKE ALL ON FUNCTION public.activate_admin_totp_enrollment(UUID, BIGINT, UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.reset_admin_totp_enrollment(UUID, BYTEA, BYTEA, TEXT, TEXT, UUID) FROM PUBLIC;

COMMENT ON FUNCTION public.activate_admin_totp_enrollment(UUID, BIGINT, UUID) IS
    'Operator-only activation after an out-of-band enrollment code has been verified; records the first accepted TOTP step and audit event.';
COMMENT ON FUNCTION public.reset_admin_totp_enrollment(UUID, BYTEA, BYTEA, TEXT, TEXT, UUID) IS
    'Operator-only TOTP re-enrollment that replaces encrypted material, resets replay state, consumes pending challenges, revokes Admin sessions, and appends an audit event.';
