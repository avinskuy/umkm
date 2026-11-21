-- Identity Core slice 0005: append-only identity and security audit events.
-- Runtime event emission, grants, retention and external forwarding remain separate.

CREATE FUNCTION public.is_safe_identity_audit_metadata(p_metadata JSONB)
RETURNS BOOLEAN
LANGUAGE plpgsql
IMMUTABLE
STRICT
SET search_path = pg_catalog
AS $$
DECLARE
    v_key TEXT;
    v_value JSONB;
    v_value_type TEXT;
BEGIN
    IF pg_catalog.jsonb_typeof(p_metadata) <> 'object'
       OR pg_catalog.octet_length(p_metadata::TEXT) > 2048 THEN
        RETURN FALSE;
    END IF;

    FOR v_key, v_value IN
        SELECT key, value
        FROM pg_catalog.jsonb_each(p_metadata)
    LOOP
        IF NOT (v_key = ANY (ARRAY[
            'channel',
            'purpose',
            'account_status',
            'role',
            'session_id',
            'challenge_id',
            'recovery_case_id',
            'resource_type',
            'resource_id',
            'provider',
            'policy_code',
            'risk_level',
            'admin_action_id',
            'verification_method',
            'result_code',
            'count_bucket',
            'device_class'
        ]::TEXT[])) THEN
            RETURN FALSE;
        END IF;

        v_value_type := pg_catalog.jsonb_typeof(v_value);
        IF v_value_type NOT IN ('string', 'number', 'boolean', 'null') THEN
            RETURN FALSE;
        END IF;

        IF v_value_type = 'string'
           AND pg_catalog.length(v_value #>> '{}') > 128 THEN
            RETURN FALSE;
        END IF;
    END LOOP;

    RETURN TRUE;
END;
$$;

CREATE TABLE public.identity_audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    actor_type TEXT NOT NULL,
    actor_id UUID,
    subject_user_id UUID,
    action_code TEXT NOT NULL,
    outcome TEXT NOT NULL,
    reason_code TEXT,
    request_id UUID,
    metadata JSONB NOT NULL DEFAULT '{}'::JSONB,

    CONSTRAINT identity_audit_events_subject_user_fk
        FOREIGN KEY (subject_user_id)
        REFERENCES public.users (id) ON DELETE RESTRICT,
    CONSTRAINT identity_audit_events_actor_type_check
        CHECK (actor_type IN ('user', 'admin', 'system', 'operator')),
    CONSTRAINT identity_audit_events_action_code_check
        CHECK (action_code ~ '^[a-z][a-z0-9_.:-]{2,127}$'),
    CONSTRAINT identity_audit_events_outcome_check
        CHECK (outcome IN ('success', 'failure', 'denied')),
    CONSTRAINT identity_audit_events_reason_code_check
        CHECK (
            reason_code IS NULL
            OR reason_code ~ '^[a-z][a-z0-9_.:-]{1,127}$'
        ),
    CONSTRAINT identity_audit_events_metadata_check
        CHECK (public.is_safe_identity_audit_metadata(metadata))
);

CREATE INDEX identity_audit_events_subject_time_idx
    ON public.identity_audit_events (subject_user_id, occurred_at DESC)
    WHERE subject_user_id IS NOT NULL;

CREATE INDEX identity_audit_events_actor_time_idx
    ON public.identity_audit_events (actor_type, actor_id, occurred_at DESC)
    WHERE actor_id IS NOT NULL;

CREATE INDEX identity_audit_events_action_time_idx
    ON public.identity_audit_events (action_code, occurred_at DESC);

CREATE INDEX identity_audit_events_request_idx
    ON public.identity_audit_events (request_id)
    WHERE request_id IS NOT NULL;

CREATE FUNCTION public.guard_identity_audit_event()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
BEGIN
    RAISE EXCEPTION 'identity audit events are append-only'
        USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER identity_audit_events_10_no_update_delete
    BEFORE UPDATE OR DELETE ON public.identity_audit_events
    FOR EACH ROW EXECUTE FUNCTION public.guard_identity_audit_event();

CREATE TRIGGER identity_audit_events_20_no_truncate
    BEFORE TRUNCATE ON public.identity_audit_events
    FOR EACH STATEMENT EXECUTE FUNCTION public.guard_identity_audit_event();

REVOKE ALL ON TABLE public.identity_audit_events FROM PUBLIC;
REVOKE ALL ON FUNCTION public.is_safe_identity_audit_metadata(JSONB) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.guard_identity_audit_event() FROM PUBLIC;

COMMENT ON TABLE public.identity_audit_events IS
    'Jejak audit Identity Core yang append-only untuk principal aplikasi normal; bukan log payload request atau penyimpanan bukti mentah.';
COMMENT ON FUNCTION public.is_safe_identity_audit_metadata(JSONB) IS
    'Memastikan metadata audit berupa objek datar dengan key allowlist dan nilai skalar terbatas; validasi semantik tetap wajib di runtime.';
