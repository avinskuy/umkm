-- profil publik dan bukti penerimaan kebijakan --

CREATE TABLE public.user_profiles (
    user_id UUID PRIMARY KEY,
    display_name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),

    CONSTRAINT user_profiles_user_fk
        FOREIGN KEY (user_id) REFERENCES public.users (id) ON DELETE RESTRICT,
    CONSTRAINT user_profiles_display_name_check
        CHECK (
            length(btrim(display_name)) BETWEEN 1 AND 120
            AND display_name = btrim(display_name)
        )
);

CREATE FUNCTION public.touch_user_profile_updated_at()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
BEGIN
    NEW.updated_at := clock_timestamp();
    RETURN NEW;
END;
$$;

CREATE TRIGGER user_profiles_20_touch_updated_at
    BEFORE UPDATE ON public.user_profiles
    FOR EACH ROW
    EXECUTE FUNCTION public.touch_user_profile_updated_at();

CREATE TABLE public.policy_acceptance_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    policy_code TEXT NOT NULL,
    policy_version TEXT NOT NULL,
    accepted_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),

    CONSTRAINT policy_acceptance_records_user_fk
        FOREIGN KEY (user_id) REFERENCES public.users (id) ON DELETE RESTRICT,
    CONSTRAINT policy_acceptance_records_code_check
        CHECK (policy_code IN ('terms_of_service', 'privacy_policy')),
    CONSTRAINT policy_acceptance_records_version_check
        CHECK (
            length(btrim(policy_version)) BETWEEN 1 AND 64
            AND policy_version = btrim(policy_version)
        ),
    CONSTRAINT policy_acceptance_records_user_policy_version_uidx
        UNIQUE (user_id, policy_code, policy_version)
);

CREATE INDEX policy_acceptance_records_user_accepted_idx
    ON public.policy_acceptance_records (user_id, accepted_at DESC);

CREATE TABLE public.consent_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    purpose TEXT NOT NULL,
    action TEXT NOT NULL,
    notice_version TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),

    CONSTRAINT consent_events_user_fk
        FOREIGN KEY (user_id) REFERENCES public.users (id) ON DELETE RESTRICT,
    CONSTRAINT consent_events_purpose_check
        CHECK (purpose = 'marketing'),
    CONSTRAINT consent_events_action_check
        CHECK (action IN ('granted', 'withdrawn')),
    CONSTRAINT consent_events_notice_version_check
        CHECK (
            length(btrim(notice_version)) BETWEEN 1 AND 64
            AND notice_version = btrim(notice_version)
        )
);

CREATE INDEX consent_events_user_purpose_occurred_idx
    ON public.consent_events (user_id, purpose, occurred_at DESC, id DESC);

CREATE FUNCTION public.guard_user_policy_history()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
BEGIN
    RAISE EXCEPTION 'policy acceptance and consent history is append-only'
        USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER policy_acceptance_records_10_append_only
    BEFORE UPDATE OR DELETE ON public.policy_acceptance_records
    FOR EACH ROW
    EXECUTE FUNCTION public.guard_user_policy_history();

CREATE TRIGGER consent_events_10_append_only
    BEFORE UPDATE OR DELETE ON public.consent_events
    FOR EACH ROW
    EXECUTE FUNCTION public.guard_user_policy_history();

CREATE TRIGGER policy_acceptance_records_90_no_truncate
    BEFORE TRUNCATE ON public.policy_acceptance_records
    FOR EACH STATEMENT
    EXECUTE FUNCTION public.guard_user_policy_history();

CREATE TRIGGER consent_events_90_no_truncate
    BEFORE TRUNCATE ON public.consent_events
    FOR EACH STATEMENT
    EXECUTE FUNCTION public.guard_user_policy_history();

CREATE FUNCTION public.create_client_user_with_profile_and_policy(
    p_display_name TEXT,
    p_primary_role TEXT,
    p_initial_contact_channel TEXT,
    p_email_normalized TEXT,
    p_phone_e164 TEXT,
    p_password_hash TEXT,
    p_terms_version TEXT,
    p_privacy_version TEXT,
    p_marketing_consent BOOLEAN,
    p_marketing_notice_version TEXT DEFAULT NULL
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_user_id UUID;
BEGIN
    IF p_display_name IS NULL
       OR length(btrim(p_display_name)) NOT BETWEEN 1 AND 120
       OR p_display_name <> btrim(p_display_name) THEN
        RAISE EXCEPTION 'public display name is invalid'
            USING ERRCODE = '22023';
    END IF;

    IF p_terms_version IS NULL
       OR length(btrim(p_terms_version)) NOT BETWEEN 1 AND 64
       OR p_terms_version <> btrim(p_terms_version)
       OR p_privacy_version IS NULL
       OR length(btrim(p_privacy_version)) NOT BETWEEN 1 AND 64
       OR p_privacy_version <> btrim(p_privacy_version) THEN
        RAISE EXCEPTION 'required policy version is invalid'
            USING ERRCODE = '22023';
    END IF;

    IF p_marketing_consent IS NULL
       OR (p_marketing_consent AND (
           p_marketing_notice_version IS NULL
           OR length(btrim(p_marketing_notice_version)) NOT BETWEEN 1 AND 64
           OR p_marketing_notice_version <> btrim(p_marketing_notice_version)
       ))
       OR (NOT p_marketing_consent AND p_marketing_notice_version IS NOT NULL) THEN
        RAISE EXCEPTION 'optional marketing consent input is invalid'
            USING ERRCODE = '22023';
    END IF;

    v_user_id := public.create_client_user(
        p_primary_role,
        p_initial_contact_channel,
        p_email_normalized,
        p_phone_e164,
        p_password_hash
    );

    INSERT INTO public.user_profiles (user_id, display_name)
    VALUES (v_user_id, p_display_name);

    INSERT INTO public.policy_acceptance_records (user_id, policy_code, policy_version)
    VALUES
        (v_user_id, 'terms_of_service', p_terms_version),
        (v_user_id, 'privacy_policy', p_privacy_version);

    IF p_marketing_consent THEN
        INSERT INTO public.consent_events (user_id, purpose, action, notice_version)
        VALUES (v_user_id, 'marketing', 'granted', p_marketing_notice_version);
    END IF;

    RETURN v_user_id;
END;
$$;

REVOKE ALL ON TABLE public.user_profiles FROM PUBLIC;
REVOKE ALL ON TABLE public.policy_acceptance_records FROM PUBLIC;
REVOKE ALL ON TABLE public.consent_events FROM PUBLIC;

REVOKE ALL ON FUNCTION public.touch_user_profile_updated_at() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.guard_user_policy_history() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.create_client_user_with_profile_and_policy(
    TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, BOOLEAN, TEXT
) FROM PUBLIC;

COMMENT ON TABLE public.user_profiles IS
    'Profil dasar pengguna; display_name merupakan nama tampilan publik, bukan bukti identitas legal.';
COMMENT ON TABLE public.policy_acceptance_records IS
    'Bukti penerimaan Terms of Service dan Privacy Policy per versi; append-only.';
COMMENT ON TABLE public.consent_events IS
    'Riwayat persetujuan opsional seperti marketing; tanpa event granted berarti tidak ada persetujuan aktif.';
COMMENT ON FUNCTION public.create_client_user_with_profile_and_policy(
    TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, TEXT, BOOLEAN, TEXT
) IS
    'Membuat akun pending, profil publik, penerimaan kebijakan wajib, dan consent marketing opsional secara atomik.';
