-- Identity Core slice 0006: password + OTP only; remove unused Google OAuth support.
-- Migration 0001 is immutable. This forward migration removes the provider identity
-- table and narrows the public account-creation function to password registration.

DO $preflight$
BEGIN
    IF EXISTS (SELECT 1 FROM public.user_identities) THEN
        RAISE EXCEPTION
            'Cannot remove provider identities automatically; review existing rows before applying migration 0006';
    END IF;
END;
$preflight$;

DROP TRIGGER IF EXISTS users_90_auth_method_check ON public.users;
DROP TRIGGER IF EXISTS user_identities_90_auth_method_check ON public.user_identities;
DROP TRIGGER IF EXISTS user_identities_10_guard_binding ON public.user_identities;

DROP FUNCTION public.assert_identity_auth_method();
DROP FUNCTION public.guard_user_identity_binding();
DROP FUNCTION public.create_client_user(TEXT, TEXT, TEXT, TEXT, TEXT, TEXT);
DROP TABLE public.user_identities;

CREATE FUNCTION public.assert_client_password_auth_method()
RETURNS TRIGGER
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_primary_role TEXT;
    v_account_status TEXT;
    v_password_hash TEXT;
BEGIN
    SELECT primary_role, account_status, password_hash
      INTO v_primary_role, v_account_status, v_password_hash
      FROM public.users
     WHERE id = NEW.id;

    IF FOUND
       AND v_primary_role IN ('business_owner', 'investor')
       AND v_account_status NOT IN ('pending_expired', 'closed')
       AND v_password_hash IS NULL THEN
        RAISE EXCEPTION 'client accounts require a local password authentication method'
            USING ERRCODE = '23514';
    END IF;

    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER users_90_password_auth_method_check
    AFTER INSERT OR UPDATE ON public.users
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW
    EXECUTE FUNCTION public.assert_client_password_auth_method();

CREATE FUNCTION public.create_client_user(
    p_primary_role TEXT,
    p_initial_contact_channel TEXT,
    p_email_normalized TEXT,
    p_phone_e164 TEXT,
    p_password_hash TEXT
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

    IF p_password_hash IS NULL
       OR length(btrim(p_password_hash)) = 0
       OR p_password_hash <> btrim(p_password_hash) THEN
        RAISE EXCEPTION 'password hash is required and cannot be empty'
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
        clock_timestamp() + INTERVAL '30 minutes'
    )
    RETURNING id INTO v_user_id;

    RETURN v_user_id;
END;
$$;

REVOKE ALL ON FUNCTION public.assert_client_password_auth_method() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.create_client_user(TEXT, TEXT, TEXT, TEXT, TEXT) FROM PUBLIC;

COMMENT ON FUNCTION public.create_client_user(TEXT, TEXT, TEXT, TEXT, TEXT) IS
    'Membuat akun klien pending dengan password lokal dan role Owner/Investor saja. Aktivasi menunggu verifikasi OTP; runtime principal harus diberi EXECUTE secara eksplisit melalui bootstrap privilege.';
