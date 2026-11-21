-- Identity Core slice 0004: shared recovery contacts and account recovery records.
-- Runtime cryptography, recovery decisions, password updates and grants remain external.

CREATE TABLE public.recovery_contacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    channel TEXT NOT NULL,
    contact_ciphertext BYTEA NOT NULL,
    contact_nonce BYTEA NOT NULL,
    encryption_key_version TEXT NOT NULL,
    contact_hmac BYTEA NOT NULL,
    hmac_key_version TEXT NOT NULL,
    verification_session_id UUID NOT NULL UNIQUE,
    verified_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    revoked_at TIMESTAMPTZ,

    CONSTRAINT recovery_contacts_user_fk
        FOREIGN KEY (user_id) REFERENCES public.users (id) ON DELETE RESTRICT,
    CONSTRAINT recovery_contacts_verification_session_fk
        FOREIGN KEY (verification_session_id)
        REFERENCES public.verification_sessions (id) ON DELETE RESTRICT,
    CONSTRAINT recovery_contacts_channel_check
        CHECK (channel IN ('email', 'whatsapp')),
    CONSTRAINT recovery_contacts_payload_check
        CHECK (
            octet_length(contact_ciphertext) > 0
            AND octet_length(contact_nonce) > 0
            AND length(btrim(encryption_key_version)) > 0
            AND octet_length(contact_hmac) > 0
            AND length(btrim(hmac_key_version)) > 0
        ),
    CONSTRAINT recovery_contacts_revoked_check
        CHECK (revoked_at IS NULL OR revoked_at >= verified_at)
);

CREATE UNIQUE INDEX recovery_contacts_account_fingerprint_uidx
    ON public.recovery_contacts (user_id, channel, contact_hmac)
    WHERE revoked_at IS NULL;

CREATE INDEX recovery_contacts_verified_account_idx
    ON public.recovery_contacts (user_id, channel, verified_at DESC)
    WHERE revoked_at IS NULL;

CREATE FUNCTION public.guard_recovery_contact()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
DECLARE
    v_session_user_id UUID;
    v_session_purpose TEXT;
    v_session_channel TEXT;
    v_session_status TEXT;
    v_session_verified_at TIMESTAMPTZ;
    v_session_destination_hmac BYTEA;
    v_session_hmac_key_version TEXT;
    v_account_status TEXT;
BEGIN
    IF TG_OP = 'INSERT' THEN
        SELECT user_id, purpose, channel, status, verified_at, destination_hmac, hmac_key_version
        INTO v_session_user_id, v_session_purpose, v_session_channel, v_session_status,
             v_session_verified_at, v_session_destination_hmac, v_session_hmac_key_version
        FROM public.verification_sessions
        WHERE id = NEW.verification_session_id;

        IF NOT FOUND
           OR v_session_purpose <> 'recovery_contact_add'
           OR v_session_status <> 'verified'
           OR v_session_user_id IS DISTINCT FROM NEW.user_id
           OR v_session_channel IS DISTINCT FROM NEW.channel
           OR v_session_destination_hmac IS DISTINCT FROM NEW.contact_hmac
           OR v_session_hmac_key_version IS DISTINCT FROM NEW.hmac_key_version THEN
            RAISE EXCEPTION 'recovery contact requires its matching verified account-bound session'
                USING ERRCODE = '23514';
        END IF;

        SELECT account_status INTO v_account_status
        FROM public.users
        WHERE id = NEW.user_id
        FOR SHARE;

        IF NOT FOUND OR v_account_status <> 'active' THEN
            RAISE EXCEPTION 'recovery contacts may only be added to active accounts'
                USING ERRCODE = '23514';
        END IF;

        NEW.created_at := clock_timestamp();
        NEW.verified_at := v_session_verified_at;
        IF NEW.revoked_at IS NOT NULL THEN
            RAISE EXCEPTION 'new recovery contact cannot start revoked'
                USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;

    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.user_id IS DISTINCT FROM OLD.user_id
       OR NEW.channel IS DISTINCT FROM OLD.channel
       OR NEW.contact_ciphertext IS DISTINCT FROM OLD.contact_ciphertext
       OR NEW.contact_nonce IS DISTINCT FROM OLD.contact_nonce
       OR NEW.encryption_key_version IS DISTINCT FROM OLD.encryption_key_version
       OR NEW.contact_hmac IS DISTINCT FROM OLD.contact_hmac
       OR NEW.hmac_key_version IS DISTINCT FROM OLD.hmac_key_version
       OR NEW.verification_session_id IS DISTINCT FROM OLD.verification_session_id
       OR NEW.verified_at IS DISTINCT FROM OLD.verified_at
       OR NEW.created_at IS DISTINCT FROM OLD.created_at THEN
        RAISE EXCEPTION 'verified recovery contact identity and proof are immutable'
            USING ERRCODE = '23514';
    END IF;

    IF OLD.revoked_at IS NOT NULL THEN
        RAISE EXCEPTION 'revoked recovery contact is immutable'
            USING ERRCODE = '23514';
    END IF;

    IF NEW.revoked_at IS DISTINCT FROM OLD.revoked_at THEN
        IF NEW.revoked_at IS NULL THEN
            RAISE EXCEPTION 'recovery contact revocation cannot be reversed'
                USING ERRCODE = '23514';
        END IF;
        NEW.revoked_at := clock_timestamp();
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER recovery_contacts_10_guard
    BEFORE INSERT OR UPDATE ON public.recovery_contacts
    FOR EACH ROW EXECUTE FUNCTION public.guard_recovery_contact();

CREATE TABLE public.account_recovery_cases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    verification_session_id UUID NOT NULL UNIQUE,
    target_user_id UUID,
    requested_action TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending_verification',
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    resolved_at TIMESTAMPTZ,

    CONSTRAINT account_recovery_cases_session_fk
        FOREIGN KEY (verification_session_id)
        REFERENCES public.verification_sessions (id) ON DELETE RESTRICT,
    CONSTRAINT account_recovery_cases_target_user_fk
        FOREIGN KEY (target_user_id) REFERENCES public.users (id) ON DELETE RESTRICT,
    CONSTRAINT account_recovery_cases_id_target_uidx UNIQUE (id, target_user_id),
    CONSTRAINT account_recovery_cases_action_check
        CHECK (requested_action IN ('password_reset', 'forgot_email', 'account_access')),
    CONSTRAINT account_recovery_cases_status_check
        CHECK (status IN (
            'pending_verification',
            'account_selection_required',
            'under_review',
            'approved',
            'rejected',
            'completed',
            'cancelled',
            'expired'
        )),
    CONSTRAINT account_recovery_cases_resolution_check
        CHECK (
            (status IN ('rejected', 'completed', 'cancelled', 'expired') AND resolved_at IS NOT NULL)
            OR (status NOT IN ('rejected', 'completed', 'cancelled', 'expired') AND resolved_at IS NULL)
        )
);

CREATE INDEX account_recovery_cases_target_status_idx
    ON public.account_recovery_cases (target_user_id, status, created_at DESC)
    WHERE target_user_id IS NOT NULL;

CREATE INDEX account_recovery_cases_pending_idx
    ON public.account_recovery_cases (created_at)
    WHERE status IN ('pending_verification', 'account_selection_required', 'under_review');

CREATE FUNCTION public.guard_account_recovery_case()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
DECLARE
    v_session_purpose TEXT;
    v_session_status TEXT;
    v_session_user_id UUID;
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.status <> 'pending_verification'
           OR NEW.target_user_id IS NOT NULL
           OR NEW.resolved_at IS NOT NULL THEN
            RAISE EXCEPTION 'account recovery case must start pending without a selected account'
                USING ERRCODE = '23514';
        END IF;

        SELECT purpose, status, user_id
        INTO v_session_purpose, v_session_status, v_session_user_id
        FROM public.verification_sessions
        WHERE id = NEW.verification_session_id;

        IF NOT FOUND OR v_session_purpose <> 'account_recovery' OR v_session_user_id IS NOT NULL THEN
            RAISE EXCEPTION 'account recovery case requires an unbound account-recovery verification session'
                USING ERRCODE = '23514';
        END IF;

        NEW.created_at := clock_timestamp();
        NEW.updated_at := NEW.created_at;
        RETURN NEW;
    END IF;

    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.verification_session_id IS DISTINCT FROM OLD.verification_session_id
       OR NEW.requested_action IS DISTINCT FROM OLD.requested_action
       OR NEW.created_at IS DISTINCT FROM OLD.created_at THEN
        RAISE EXCEPTION 'account recovery case identity and requested action are immutable'
            USING ERRCODE = '23514';
    END IF;

    IF OLD.status IN ('rejected', 'completed', 'cancelled', 'expired') THEN
        RAISE EXCEPTION 'terminal account recovery case cannot be changed'
            USING ERRCODE = '23514';
    END IF;

    IF OLD.target_user_id IS NOT NULL AND NEW.target_user_id IS DISTINCT FROM OLD.target_user_id THEN
        RAISE EXCEPTION 'selected recovery account is immutable'
            USING ERRCODE = '23514';
    END IF;

    IF NEW.target_user_id IS NOT NULL AND OLD.target_user_id IS NULL THEN
        SELECT purpose, status, user_id
        INTO v_session_purpose, v_session_status, v_session_user_id
        FROM public.verification_sessions
        WHERE id = OLD.verification_session_id;

        IF v_session_purpose <> 'account_recovery'
           OR v_session_status <> 'verified'
           OR v_session_user_id IS DISTINCT FROM NEW.target_user_id THEN
            RAISE EXCEPTION 'recovery account may only be selected after contact verification and session binding'
                USING ERRCODE = '23514';
        END IF;
    END IF;

    IF NEW.status IS DISTINCT FROM OLD.status THEN
        IF NOT (
            (OLD.status = 'pending_verification' AND NEW.status IN ('account_selection_required', 'under_review', 'rejected', 'cancelled', 'expired'))
            OR (OLD.status = 'account_selection_required' AND NEW.status IN ('under_review', 'rejected', 'cancelled', 'expired'))
            OR (OLD.status = 'under_review' AND NEW.status IN ('approved', 'rejected', 'cancelled', 'expired'))
            OR (OLD.status = 'approved' AND NEW.status IN ('completed', 'cancelled', 'expired'))
        ) THEN
            RAISE EXCEPTION 'invalid account recovery case transition'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.status IN ('rejected', 'completed', 'cancelled', 'expired') THEN
            NEW.resolved_at := clock_timestamp();
        ELSIF NEW.resolved_at IS NOT NULL THEN
            RAISE EXCEPTION 'non-terminal recovery case cannot have a resolution timestamp'
                USING ERRCODE = '23514';
        END IF;
    ELSIF NEW.resolved_at IS DISTINCT FROM OLD.resolved_at THEN
        RAISE EXCEPTION 'recovery case resolution timestamp is database managed'
            USING ERRCODE = '23514';
    END IF;

    NEW.updated_at := clock_timestamp();
    RETURN NEW;
END;
$$;

CREATE TRIGGER account_recovery_cases_10_guard
    BEFORE INSERT OR UPDATE ON public.account_recovery_cases
    FOR EACH ROW EXECUTE FUNCTION public.guard_account_recovery_case();

CREATE TABLE public.password_reset_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recovery_case_id UUID NOT NULL,
    user_id UUID NOT NULL,
    token_hash BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    revocation_reason TEXT,

    CONSTRAINT password_reset_tokens_case_user_fk
        FOREIGN KEY (recovery_case_id, user_id)
        REFERENCES public.account_recovery_cases (id, target_user_id) ON DELETE RESTRICT,
    CONSTRAINT password_reset_tokens_hash_check
        CHECK (octet_length(token_hash) = 32),
    CONSTRAINT password_reset_tokens_hash_uidx UNIQUE (token_hash),
    CONSTRAINT password_reset_tokens_deadline_check
        CHECK (expires_at > created_at),
    CONSTRAINT password_reset_tokens_terminal_check
        CHECK (
            NOT (consumed_at IS NOT NULL AND revoked_at IS NOT NULL)
            AND (revoked_at IS NULL OR revocation_reason IN (
                'superseded',
                'security_event',
                'account_status_changed',
                'admin_action'
            ))
            AND (revocation_reason IS NULL OR revoked_at IS NOT NULL)
        )
);

CREATE UNIQUE INDEX password_reset_tokens_one_live_per_case_uidx
    ON public.password_reset_tokens (recovery_case_id)
    WHERE consumed_at IS NULL AND revoked_at IS NULL;

CREATE INDEX password_reset_tokens_user_created_idx
    ON public.password_reset_tokens (user_id, created_at DESC);

CREATE FUNCTION public.guard_password_reset_token()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
DECLARE
    v_case_status TEXT;
    v_case_target_user_id UUID;
    v_account_status TEXT;
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.consumed_at IS NOT NULL OR NEW.revoked_at IS NOT NULL OR NEW.revocation_reason IS NOT NULL THEN
            RAISE EXCEPTION 'new password reset token must be unused and unrevoked'
                USING ERRCODE = '23514';
        END IF;

        SELECT status, target_user_id
        INTO v_case_status, v_case_target_user_id
        FROM public.account_recovery_cases
        WHERE id = NEW.recovery_case_id;

        IF NOT FOUND OR v_case_status <> 'approved' OR v_case_target_user_id IS DISTINCT FROM NEW.user_id THEN
            RAISE EXCEPTION 'password reset token requires an approved recovery case for the same account'
                USING ERRCODE = '23514';
        END IF;

        SELECT account_status INTO v_account_status
        FROM public.users
        WHERE id = NEW.user_id
        FOR SHARE;

        IF NOT FOUND OR v_account_status <> 'active' THEN
            RAISE EXCEPTION 'password reset token may only be issued for an active account'
                USING ERRCODE = '23514';
        END IF;

        NEW.created_at := clock_timestamp();
        IF NEW.expires_at <= NEW.created_at THEN
            RAISE EXCEPTION 'password reset token deadline must be in the future'
                USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;

    IF NEW.id IS DISTINCT FROM OLD.id
       OR NEW.recovery_case_id IS DISTINCT FROM OLD.recovery_case_id
       OR NEW.user_id IS DISTINCT FROM OLD.user_id
       OR NEW.token_hash IS DISTINCT FROM OLD.token_hash
       OR NEW.created_at IS DISTINCT FROM OLD.created_at
       OR NEW.expires_at IS DISTINCT FROM OLD.expires_at THEN
        RAISE EXCEPTION 'password reset token identity, hash and deadline are immutable'
            USING ERRCODE = '23514';
    END IF;

    IF OLD.consumed_at IS NOT NULL OR OLD.revoked_at IS NOT NULL THEN
        RAISE EXCEPTION 'terminal password reset token cannot be changed'
            USING ERRCODE = '23514';
    END IF;

    IF NEW.consumed_at IS NOT DISTINCT FROM OLD.consumed_at
       AND NEW.revoked_at IS NOT DISTINCT FROM OLD.revoked_at THEN
        RAISE EXCEPTION 'password reset token update must consume or revoke the token'
            USING ERRCODE = '23514';
    END IF;

    IF NEW.consumed_at IS NOT NULL THEN
        IF NEW.revoked_at IS NOT NULL OR NEW.expires_at <= clock_timestamp() THEN
            RAISE EXCEPTION 'expired or revoked password reset token cannot be consumed'
                USING ERRCODE = '23514';
        END IF;
        IF NEW.revocation_reason IS NOT NULL THEN
            RAISE EXCEPTION 'consumed token cannot have a revocation reason'
                USING ERRCODE = '23514';
        END IF;
        NEW.consumed_at := clock_timestamp();
    ELSE
        IF NEW.revoked_at IS NULL OR NEW.revocation_reason IS NULL THEN
            RAISE EXCEPTION 'token revocation requires a reason'
                USING ERRCODE = '23514';
        END IF;
        NEW.revoked_at := clock_timestamp();
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER password_reset_tokens_10_guard
    BEFORE INSERT OR UPDATE ON public.password_reset_tokens
    FOR EACH ROW EXECUTE FUNCTION public.guard_password_reset_token();
