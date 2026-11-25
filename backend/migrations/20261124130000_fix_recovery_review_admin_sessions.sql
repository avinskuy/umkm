-- Align manual recovery review decisions with the dedicated Admin session store.
-- Historical migrations remain immutable; this migration repairs the live schema.

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
          FROM public.account_recovery_review_decisions
         LIMIT 1
    ) THEN
        RAISE EXCEPTION
            'cannot migrate recovery reviewer session references automatically; manual reconciliation is required'
            USING ERRCODE = '55000';
    END IF;
END;
$$;

ALTER TABLE public.account_recovery_review_decisions
    DROP CONSTRAINT account_recovery_review_session_fk;

ALTER TABLE public.account_recovery_review_decisions
    ADD CONSTRAINT account_recovery_review_session_fk
        FOREIGN KEY (reviewer_session_id)
        REFERENCES public.admin_sessions (id) ON DELETE RESTRICT;

CREATE OR REPLACE FUNCTION public.review_account_recovery_case(
    p_case_id UUID,
    p_reviewer_user_id UUID,
    p_reviewer_session_token_hash BYTEA,
    p_decision TEXT,
    p_reason_code TEXT
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_target_user_id UUID;
    v_requested_action TEXT;
    v_case_status TEXT;
    v_reviewer_role TEXT;
    v_reviewer_status TEXT;
    v_reviewer_session_id UUID;
    v_target_status TEXT;
    v_action_code TEXT;
BEGIN
    IF p_case_id IS NULL
       OR p_reviewer_user_id IS NULL
       OR p_reviewer_session_token_hash IS NULL
       OR octet_length(p_reviewer_session_token_hash) <> 32
       OR p_decision IS NULL
       OR p_reason_code IS NULL
       OR p_decision NOT IN ('approved', 'rejected')
       OR p_reason_code NOT IN (
           'ownership_confirmed',
           'insufficient_evidence',
           'suspicious_activity',
           'unsupported_action',
           'policy_violation'
       )
       OR (p_decision = 'approved' AND p_reason_code <> 'ownership_confirmed')
       OR (p_decision = 'rejected' AND p_reason_code = 'ownership_confirmed') THEN
        RAISE EXCEPTION 'recovery review input is invalid'
            USING ERRCODE = '22023';
    END IF;

    SELECT target_user_id, requested_action, status
      INTO v_target_user_id, v_requested_action, v_case_status
      FROM public.account_recovery_cases
     WHERE id = p_case_id
     FOR UPDATE;

    IF NOT FOUND
       OR v_case_status <> 'under_review'
       OR v_target_user_id IS NULL THEN
        RAISE EXCEPTION 'recovery case is not eligible for review'
            USING ERRCODE = 'P0002';
    END IF;

    SELECT primary_role, account_status
      INTO v_reviewer_role, v_reviewer_status
      FROM public.users
     WHERE id = p_reviewer_user_id;

    IF NOT FOUND
       OR v_reviewer_role <> 'admin'
       OR v_reviewer_status <> 'active'
       OR p_reviewer_user_id = v_target_user_id THEN
        RAISE EXCEPTION 'recovery reviewer is not authorized'
            USING ERRCODE = '42501';
    END IF;

    SELECT id
      INTO v_reviewer_session_id
      FROM public.admin_sessions
     WHERE token_hash = p_reviewer_session_token_hash
       AND user_id = p_reviewer_user_id
       AND revoked_at IS NULL
       AND expires_at > clock_timestamp()
       AND last_seen_at > clock_timestamp() - INTERVAL '7 days'
     FOR SHARE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'recovery reviewer session is not active'
            USING ERRCODE = '42501';
    END IF;

    SELECT account_status
      INTO v_target_status
      FROM public.users
     WHERE id = v_target_user_id
     FOR SHARE;

    IF NOT FOUND OR v_target_status <> 'active' THEN
        RAISE EXCEPTION 'recovery target account is not active'
            USING ERRCODE = 'P0002';
    END IF;

    IF p_decision = 'approved' AND v_requested_action <> 'password_reset' THEN
        RAISE EXCEPTION 'requested recovery action is not implemented'
            USING ERRCODE = '22023';
    END IF;

    INSERT INTO public.account_recovery_review_decisions (
        recovery_case_id, target_user_id, reviewer_user_id, reviewer_session_id, decision, reason_code
    )
    VALUES (
        p_case_id, v_target_user_id, p_reviewer_user_id, v_reviewer_session_id, p_decision, p_reason_code
    );

    UPDATE public.account_recovery_cases
       SET status = p_decision
     WHERE id = p_case_id;

    v_action_code := CASE
        WHEN p_decision = 'approved' THEN 'identity.recovery.review.approved'
        ELSE 'identity.recovery.review.rejected'
    END;

    INSERT INTO public.identity_audit_events (
        actor_type, actor_id, subject_user_id, action_code, outcome, reason_code, metadata
    )
    VALUES (
        'admin',
        p_reviewer_user_id,
        v_target_user_id,
        v_action_code,
        'success',
        p_reason_code,
        pg_catalog.jsonb_build_object(
            'purpose', 'account_recovery',
            'recovery_case_id', p_case_id::text,
            'session_id', v_reviewer_session_id::text
        )
    );

    RETURN TRUE;
END;
$$;

COMMENT ON CONSTRAINT account_recovery_review_session_fk
    ON public.account_recovery_review_decisions IS
    'Reviewer session must reference the dedicated Admin session store.';
COMMENT ON FUNCTION public.review_account_recovery_case(UUID, UUID, BYTEA, TEXT, TEXT) IS
    'Applies an Admin recovery review decision after validating an active dedicated Admin session, selected active target, decision constraints, state transition, and audit event.';
