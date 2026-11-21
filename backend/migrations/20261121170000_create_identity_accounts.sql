-- Identity Core slice 0001: account identity and external provider identity.
-- Runtime grants are provisioned separately; this migration never grants table
-- ownership or unrestricted INSERT access to the public application principal.

CREATE TABLE public.users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    primary_role TEXT NOT NULL,
    account_status TEXT NOT NULL DEFAULT 'pending_verification',
    email_normalized TEXT,
    phone_e164 TEXT,
    initial_contact_channel TEXT NOT NULL,
    email_verified_at TIMESTAMPTZ,
    phone_verified_at TIMESTAMPTZ,
    initial_contact_verified_at TIMESTAMPTZ,
    password_hash TEXT,
    pending_expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    status_changed_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT users_primary_role_check
        CHECK (primary_role IN ('business_owner', 'investor', 'admin')),

    CONSTRAINT users_account_status_check
        CHECK (
            account_status IN (
                'pending_verification',
                'active',
                'deactivated',
                'deletion_pending',
                'deletion_processing',
                'pending_expired',
                'closed'
            )
        ),

    CONSTRAINT users_initial_contact_channel_check
        CHECK (initial_contact_channel IN ('email', 'phone')),

    CONSTRAINT users_email_normalized_check
        CHECK (
            email_normalized IS NULL
            OR (
                length(btrim(email_normalized)) > 0
                AND email_normalized = btrim(email_normalized)
            )
        ),

    CONSTRAINT users_phone_e164_check
        CHECK (
            phone_e164 IS NULL
            OR phone_e164 ~ '^[+][1-9][0-9]{1,14}$'
        ),

    CONSTRAINT users_password_hash_check
        CHECK (password_hash IS NULL OR length(btrim(password_hash)) > 0),

    CONSTRAINT users_account_state_check
        CHECK (
            (
                account_status = 'pending_verification'
                AND pending_expires_at IS NOT NULL
                AND initial_contact_verified_at IS NULL
                AND (
                    (
                        initial_contact_channel = 'email'
                        AND email_normalized IS NOT NULL
                        AND email_verified_at IS NULL
                        AND phone_e164 IS NULL
                        AND phone_verified_at IS NULL
                    )
                    OR (
                        initial_contact_channel = 'phone'
                        AND phone_e164 IS NOT NULL
                        AND phone_verified_at IS NULL
                        AND email_normalized IS NULL
                        AND email_verified_at IS NULL
                    )
                )
            )
            OR (
                account_status IN (
                    'active',
                    'deactivated',
                    'deletion_pending',
                    'deletion_processing'
                )
                AND pending_expires_at IS NULL
                AND initial_contact_verified_at IS NOT NULL
                AND (
                    (
                        email_normalized IS NULL
                        AND email_verified_at IS NULL
                    )
                    OR (
                        email_normalized IS NOT NULL
                        AND email_verified_at IS NOT NULL
                    )
                )
                AND (
                    (
                        phone_e164 IS NULL
                        AND phone_verified_at IS NULL
                    )
                    OR (
                        phone_e164 IS NOT NULL
                        AND phone_verified_at IS NOT NULL
                    )
                )
                AND (email_normalized IS NOT NULL OR phone_e164 IS NOT NULL)
            )
            OR (
                account_status IN ('pending_expired', 'closed')
                AND pending_expires_at IS NULL
                AND email_normalized IS NULL
                AND phone_e164 IS NULL
                AND email_verified_at IS NULL
                AND phone_verified_at IS NULL
                AND initial_contact_verified_at IS NULL
                AND password_hash IS NULL
            )
        )
);

CREATE UNIQUE INDEX users_email_reservation_uidx
    ON public.users (email_normalized)
    WHERE email_normalized IS NOT NULL
      AND account_status IN (
          'pending_verification',
          'active',
          'deactivated',
          'deletion_pending',
          'deletion_processing'
      );

CREATE UNIQUE INDEX users_phone_reservation_uidx
    ON public.users (phone_e164)
    WHERE phone_e164 IS NOT NULL
      AND account_status IN (
          'pending_verification',
          'active',
          'deactivated',
          'deletion_pending',
          'deletion_processing'
      );

CREATE TABLE public.user_identities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    provider TEXT NOT NULL,
    provider_subject TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT user_identities_user_fk
        FOREIGN KEY (user_id)
        REFERENCES public.users (id)
        ON DELETE RESTRICT,

    CONSTRAINT user_identities_provider_check
        CHECK (provider = 'google'),

    CONSTRAINT user_identities_provider_subject_check
        CHECK (length(btrim(provider_subject)) > 0),

    CONSTRAINT user_identities_provider_subject_uidx
        UNIQUE (provider, provider_subject)
);

CREATE INDEX user_identities_user_id_idx
    ON public.user_identities (user_id);

CREATE FUNCTION public.guard_user_identity_fields()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
BEGIN
    IF NEW.primary_role IS DISTINCT FROM OLD.primary_role THEN
        RAISE EXCEPTION 'primary_role is immutable'
            USING ERRCODE = '23514';
    END IF;

    IF NEW.initial_contact_channel IS DISTINCT FROM OLD.initial_contact_channel THEN
        RAISE EXCEPTION 'initial_contact_channel is immutable'
            USING ERRCODE = '23514';
    END IF;

    IF OLD.initial_contact_verified_at IS NOT NULL
       AND NEW.initial_contact_verified_at IS DISTINCT FROM OLD.initial_contact_verified_at
       AND NOT (
           NEW.account_status = 'closed'
           AND NEW.initial_contact_verified_at IS NULL
       ) THEN
        RAISE EXCEPTION 'initial_contact_verified_at is immutable except for completed account deletion'
            USING ERRCODE = '23514';
    END IF;

    IF OLD.initial_contact_verified_at IS NULL
       AND NEW.initial_contact_verified_at IS NOT NULL THEN
        IF OLD.account_status <> 'pending_verification'
           OR NEW.account_status <> 'active' THEN
            RAISE EXCEPTION 'initial contact verification can only be recorded during activation'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.initial_contact_channel = 'email'
           AND (
               NEW.email_normalized IS DISTINCT FROM OLD.email_normalized
               OR NEW.email_normalized IS NULL
               OR NEW.email_verified_at IS NULL
           ) THEN
            RAISE EXCEPTION 'the original email contact must be verified during activation'
                USING ERRCODE = '23514';
        END IF;

        IF NEW.initial_contact_channel = 'phone'
           AND (
               NEW.phone_e164 IS DISTINCT FROM OLD.phone_e164
               OR NEW.phone_e164 IS NULL
               OR NEW.phone_verified_at IS NULL
           ) THEN
            RAISE EXCEPTION 'the original phone contact must be verified during activation'
                USING ERRCODE = '23514';
        END IF;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER users_10_guard_immutable_identity_fields
    BEFORE UPDATE ON public.users
    FOR EACH ROW
    EXECUTE FUNCTION public.guard_user_identity_fields();

CREATE FUNCTION public.touch_user_timestamps()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
BEGIN
    NEW.updated_at := now();

    IF NEW.account_status IS DISTINCT FROM OLD.account_status THEN
        NEW.status_changed_at := now();
    ELSE
        NEW.status_changed_at := OLD.status_changed_at;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER users_20_touch_timestamps
    BEFORE UPDATE ON public.users
    FOR EACH ROW
    EXECUTE FUNCTION public.touch_user_timestamps();

CREATE FUNCTION public.guard_user_identity_binding()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
BEGIN
    IF NEW.user_id IS DISTINCT FROM OLD.user_id
       OR NEW.provider IS DISTINCT FROM OLD.provider
       OR NEW.provider_subject IS DISTINCT FROM OLD.provider_subject THEN
        RAISE EXCEPTION 'provider identity binding is immutable'
            USING ERRCODE = '23514';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER user_identities_10_guard_binding
    BEFORE UPDATE ON public.user_identities
    FOR EACH ROW
    EXECUTE FUNCTION public.guard_user_identity_binding();

CREATE FUNCTION public.assert_identity_auth_method()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_user_id UUID;
    v_primary_role TEXT;
    v_account_status TEXT;
    v_password_hash TEXT;
    v_has_google_identity BOOLEAN;
BEGIN
    IF TG_TABLE_NAME = 'users' THEN
        IF TG_OP = 'DELETE' THEN
            v_user_id := OLD.id;
        ELSE
            v_user_id := NEW.id;
        END IF;
    ELSE
        IF TG_OP = 'DELETE' THEN
            v_user_id := OLD.user_id;
        ELSE
            v_user_id := NEW.user_id;
        END IF;
    END IF;

    SELECT primary_role, account_status, password_hash
    INTO v_primary_role, v_account_status, v_password_hash
    FROM public.users
    WHERE id = v_user_id;

    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    SELECT EXISTS (
        SELECT 1
        FROM public.user_identities
        WHERE user_id = v_user_id
          AND provider = 'google'
    )
    INTO v_has_google_identity;

    IF v_account_status IN ('pending_expired', 'closed') THEN
        IF v_has_google_identity THEN
            RAISE EXCEPTION 'terminal accounts cannot retain provider identities'
                USING ERRCODE = '23514';
        END IF;
    ELSIF v_primary_role IN ('business_owner', 'investor')
          AND v_password_hash IS NULL
          AND NOT v_has_google_identity THEN
        RAISE EXCEPTION 'client accounts without a password require a Google identity'
            USING ERRCODE = '23514';
    END IF;

    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER users_90_auth_method_check
    AFTER INSERT OR UPDATE ON public.users
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW
    EXECUTE FUNCTION public.assert_identity_auth_method();

CREATE CONSTRAINT TRIGGER user_identities_90_auth_method_check
    AFTER INSERT OR UPDATE OR DELETE ON public.user_identities
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW
    EXECUTE FUNCTION public.assert_identity_auth_method();

CREATE FUNCTION public.create_client_user(
    p_primary_role TEXT,
    p_initial_contact_channel TEXT,
    p_email_normalized TEXT,
    p_phone_e164 TEXT,
    p_password_hash TEXT,
    p_google_subject TEXT DEFAULT NULL
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_user_id UUID;
BEGIN
    IF p_primary_role IS NULL
       OR p_primary_role NOT IN ('business_owner', 'investor') THEN
        RAISE EXCEPTION 'client role is invalid'
            USING ERRCODE = '22023';
    END IF;

    IF p_initial_contact_channel IS NULL
       OR p_initial_contact_channel NOT IN ('email', 'phone') THEN
        RAISE EXCEPTION 'initial contact channel is invalid'
            USING ERRCODE = '22023';
    END IF;

    IF p_initial_contact_channel = 'email'
       AND (
           p_email_normalized IS NULL
           OR length(btrim(p_email_normalized)) = 0
           OR p_email_normalized <> btrim(p_email_normalized)
           OR p_phone_e164 IS NOT NULL
       ) THEN
        RAISE EXCEPTION 'email registration requires exactly one normalized email contact'
            USING ERRCODE = '22023';
    END IF;

    IF p_initial_contact_channel = 'phone'
       AND (
           p_phone_e164 IS NULL
           OR p_email_normalized IS NOT NULL
       ) THEN
        RAISE EXCEPTION 'phone registration requires exactly one phone contact'
            USING ERRCODE = '22023';
    END IF;

    IF (p_password_hash IS NULL) = (p_google_subject IS NULL) THEN
        RAISE EXCEPTION 'provide exactly one initial authentication method'
            USING ERRCODE = '22023';
    END IF;

    IF p_password_hash IS NOT NULL AND length(btrim(p_password_hash)) = 0 THEN
        RAISE EXCEPTION 'password hash cannot be empty'
            USING ERRCODE = '22023';
    END IF;

    IF p_google_subject IS NOT NULL AND length(btrim(p_google_subject)) = 0 THEN
        RAISE EXCEPTION 'Google provider subject cannot be empty'
            USING ERRCODE = '22023';
    END IF;

    INSERT INTO public.users (
        primary_role,
        account_status,
        email_normalized,
        phone_e164,
        initial_contact_channel,
        password_hash,
        pending_expires_at
    )
    VALUES (
        p_primary_role,
        'pending_verification',
        p_email_normalized,
        p_phone_e164,
        p_initial_contact_channel,
        p_password_hash,
        now() + INTERVAL '30 minutes'
    )
    RETURNING id INTO v_user_id;

    IF p_google_subject IS NOT NULL THEN
        INSERT INTO public.user_identities (
            user_id,
            provider,
            provider_subject
        )
        VALUES (
            v_user_id,
            'google',
            p_google_subject
        );
    END IF;

    RETURN v_user_id;
END;
$$;

REVOKE ALL ON TABLE public.users FROM PUBLIC;
REVOKE ALL ON TABLE public.user_identities FROM PUBLIC;

REVOKE ALL ON FUNCTION public.guard_user_identity_fields() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.touch_user_timestamps() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.guard_user_identity_binding() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.assert_identity_auth_method() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.create_client_user(TEXT, TEXT, TEXT, TEXT, TEXT, TEXT) FROM PUBLIC;

COMMENT ON TABLE public.users IS
    'Sumber kebenaran akun, role permanen, lifecycle, kontak utama, dan referensi kredensial lokal.';

COMMENT ON TABLE public.user_identities IS
    'Identitas provider eksternal berdasarkan stable provider subject; tidak menyimpan token OAuth.';

COMMENT ON FUNCTION public.create_client_user(TEXT, TEXT, TEXT, TEXT, TEXT, TEXT) IS
    'Membuat akun klien pending dengan role Owner/Investor saja. Runtime principal harus diberi EXECUTE secara eksplisit melalui bootstrap privilege.';
