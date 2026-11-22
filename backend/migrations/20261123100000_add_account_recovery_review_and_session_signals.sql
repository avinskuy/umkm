-- Account recovery manual review and prior-session continuity signals.

CREATE TABLE public.account_recovery_case_signals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recovery_case_id UUID NOT NULL,
    session_id UUID NOT NULL,
    signal_code TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),

    CONSTRAINT account_recovery_case_signals_case_fk
        FOREIGN KEY (recovery_case_id)
        REFERENCES public.account_recovery_cases (id) ON DELETE RESTRICT,
    CONSTRAINT account_recovery_case_signals_session_fk
        FOREIGN KEY (session_id)
        REFERENCES public.sessions (id) ON DELETE RESTRICT,
    CONSTRAINT account_recovery_case_signals_code_check
        CHECK (signal_code = 'active_session_match'),
    CONSTRAINT account_recovery_case_signals_case_code_uidx
        UNIQUE (recovery_case_id, signal_code)
);

CREATE INDEX account_recovery_case_signals_session_idx
    ON public.account_recovery_case_signals (session_id, created_at DESC);

CREATE TABLE public.account_recovery_review_decisions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recovery_case_id UUID NOT NULL UNIQUE,
    target_user_id UUID NOT NULL,
    reviewer_user_id UUID NOT NULL,
    decision TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),

    CONSTRAINT account_recovery_review_case_target_fk
        FOREIGN KEY (recovery_case_id, target_user_id)
        REFERENCES public.account_recovery_cases (id, target_user_id) ON DELETE RESTRICT,
    CONSTRAINT account_recovery_review_reviewer_fk
        FOREIGN KEY (reviewer_user_id)
        REFERENCES public.users (id) ON DELETE RESTRICT,
    CONSTRAINT account_recovery_review_decision_check
        CHECK (decision IN ('approved', 'rejected')),
    CONSTRAINT account_recovery_review_reason_check
        CHECK (reason_code IN (
            'ownership_confirmed',
            'insufficient_evidence',
            'suspicious_activity',
            'unsupported_action',
            'policy_violation'
        )),
    CONSTRAINT account_recovery_review_decision_reason_check
        CHECK (
            (decision = 'approved' AND reason_code = 'ownership_confirmed')
            OR
            (decision = 'rejected' AND reason_code <> 'ownership_confirmed')
        ),
    CONSTRAINT account_recovery_review_actor_check
        CHECK (reviewer_user_id <> target_user_id)
);

CREATE INDEX account_recovery_review_reviewer_idx
    ON public.account_recovery_review_decisions (reviewer_user_id, created_at DESC);

CREATE FUNCTION public.guard_account_recovery_append_only()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
BEGIN
    RAISE EXCEPTION 'account recovery review records and signals are append-only'
        USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER account_recovery_case_signals_10_append_only
    BEFORE UPDATE OR DELETE ON public.account_recovery_case_signals
    FOR EACH ROW EXECUTE FUNCTION public.guard_account_recovery_append_only();

CREATE TRIGGER account_recovery_review_decisions_10_append_only
    BEFORE UPDATE OR DELETE ON public.account_recovery_review_decisions
    FOR EACH ROW EXECUTE FUNCTION public.guard_account_recovery_append_only();

CREATE FUNCTION public.record_account_recovery_session_signal(
    p_case_id UUID,
    p_session_token_hash BYTEA
)
RETURNS BOOLEAN
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_target_user_id UUID;
    v_case_created_at TIMESTAMPTZ;
    v_session_id UUID;
BEGIN
    IF p_case_id IS NULL
       OR p_session_token_hash IS NULL
       OR octet_length(p_session_token_hash) <> 32 THEN
        RAISE EXCEPTION 'recovery session signal input is invalid'
            USING ERRCODE = '22023';
    END IF;

    SELECT target_user_id, created_at
      INTO v_target_user_id, v_case_created_at
      FROM public.account_recovery_cases
     WHERE id = p_case_id
       AND status = 'under_review'
       AND target_user_id IS NOT NULL
     FOR UPDATE;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'recovery case is not eligible for session signal evaluation'
            USING ERRCODE = 'P0002';
    END IF;

    SELECT s.id
      INTO v_session_id
      FROM public.sessions AS s
      JOIN public.users AS u ON u.id = s.user_id
     WHERE s.token_hash = p_session_token_hash
       AND s.user_id = v_target_user_id
       AND s.created_at <= v_case_created_at
       AND s.revoked_at IS NULL
       AND s.expires_at > clock_timestamp()
       AND s.last_seen_at > clock_timestamp() - INTERVAL '7 days'
       AND u.account_status = 'active'
     FOR SHARE OF s, u;

    IF NOT FOUND THEN
        RETURN FALSE;
    END IF;

    INSERT INTO public.account_recovery_case_signals (
        recovery_case_id, session_id, signal_code
    )
    VALUES (p_case_id, v_session_id, 'active_session_match')
    ON CONFLICT (recovery_case_id, signal_code) DO NOTHING;

    INSERT INTO public.identity_audit_events (
        actor_type, subject_user_id, action_code, outcome, reason_code, metadata
    )
    VALUES (
        'system',
        v_target_user_id,
        'identity.recovery.session_signal.matched',
        'success',
        'active_session',
        pg_catalog.jsonb_build_object(
            'purpose', 'account_recovery',
            'recovery_case_id', p_case_id::text,
            'session_id', v_session_id::text
        )
    );

    RETURN TRUE;
END;
$$;

CREATE FUNCTION public.review_account_recovery_case(
    p_case_id UUID,
    p_reviewer_user_id UUID,
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
    v_target_status TEXT;
    v_action_code TEXT;
BEGIN
    IF p_case_id IS NULL
       OR p_reviewer_user_id IS NULL
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
     WHERE id = p_reviewer_user_id
     FOR SHARE;

    IF NOT FOUND
       OR v_reviewer_role <> 'admin'
       OR v_reviewer_status <> 'active'
       OR p_reviewer_user_id = v_target_user_id THEN
        RAISE EXCEPTION 'recovery reviewer is not authorized'
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
        recovery_case_id, target_user_id, reviewer_user_id, decision, reason_code
    )
    VALUES (
        p_case_id, v_target_user_id, p_reviewer_user_id, p_decision, p_reason_code
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
            'recovery_case_id', p_case_id::text
        )
    );

    RETURN TRUE;
END;
$$;

REVOKE ALL ON TABLE public.account_recovery_case_signals FROM PUBLIC;
REVOKE ALL ON TABLE public.account_recovery_review_decisions FROM PUBLIC;
REVOKE ALL ON FUNCTION public.guard_account_recovery_append_only() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.record_account_recovery_session_signal(UUID, BYTEA) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.review_account_recovery_case(UUID, UUID, TEXT, TEXT) FROM PUBLIC;

COMMENT ON TABLE public.account_recovery_case_signals IS
    'Append-only supplemental signals for manual account recovery review; an active prior session is not sufficient by itself to approve recovery.';
COMMENT ON TABLE public.account_recovery_review_decisions IS
    'Append-only manual review decisions tied to an active Admin reviewer and the selected recovery target.';
COMMENT ON FUNCTION public.record_account_recovery_session_signal(UUID, BYTEA) IS
    'Records a match only when a valid, non-revoked, non-idle session for the selected account predates the recovery case.';
COMMENT ON FUNCTION public.review_account_recovery_case(UUID, UUID, TEXT, TEXT) IS
    'Applies an Admin review decision atomically, validating active Admin role, selected active target, reason code, state transition, and audit event.';
