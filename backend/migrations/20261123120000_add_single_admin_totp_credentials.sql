-- Single internally provisioned Admin account and encrypted TOTP credential storage.
-- Runtime login, enrollment verification, replay protection and provisioning workflow
-- are added by subsequent Identity Core integration work.

CREATE UNIQUE INDEX users_single_admin_account_uidx
    ON public.users ((primary_role))
    WHERE primary_role = 'admin';

CREATE TABLE public.admin_totp_credentials (
    user_id UUID PRIMARY KEY,
    secret_ciphertext BYTEA NOT NULL,
    secret_nonce BYTEA NOT NULL,
    encryption_key_version TEXT NOT NULL,
    enabled_at TIMESTAMPTZ,
    last_accepted_step BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),

    CONSTRAINT admin_totp_credentials_user_fk
        FOREIGN KEY (user_id) REFERENCES public.users (id) ON DELETE RESTRICT,
    CONSTRAINT admin_totp_credentials_ciphertext_check
        CHECK (octet_length(secret_ciphertext) = 36),
    CONSTRAINT admin_totp_credentials_nonce_check
        CHECK (octet_length(secret_nonce) = 12),
    CONSTRAINT admin_totp_credentials_key_version_check
        CHECK (encryption_key_version ~ '^v[1-9][0-9]{0,9}$'),
    CONSTRAINT admin_totp_credentials_step_check
        CHECK (last_accepted_step IS NULL OR (enabled_at IS NOT NULL AND last_accepted_step >= 0)),
    CONSTRAINT admin_totp_credentials_timestamp_check
        CHECK (updated_at >= created_at)
);

CREATE FUNCTION public.guard_admin_totp_credential()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
DECLARE
    v_role TEXT;
    v_status TEXT;
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

    IF OLD.enabled_at IS NOT NULL AND (
        NEW.enabled_at IS DISTINCT FROM OLD.enabled_at
        OR NEW.secret_ciphertext IS DISTINCT FROM OLD.secret_ciphertext
        OR NEW.secret_nonce IS DISTINCT FROM OLD.secret_nonce
        OR NEW.encryption_key_version IS DISTINCT FROM OLD.encryption_key_version
    ) THEN
        RAISE EXCEPTION 'enabled Admin TOTP secret and enrollment cannot be replaced directly'
            USING ERRCODE = '23514';
    END IF;

    IF OLD.last_accepted_step IS NOT NULL
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

CREATE TRIGGER admin_totp_credentials_10_guard
    BEFORE INSERT OR UPDATE ON public.admin_totp_credentials
    FOR EACH ROW EXECUTE FUNCTION public.guard_admin_totp_credential();

CREATE FUNCTION public.guard_admin_totp_credential_append_only()
RETURNS TRIGGER
LANGUAGE plpgsql
SET search_path = pg_catalog
AS $$
BEGIN
    RAISE EXCEPTION 'Admin TOTP credentials cannot be deleted or truncated'
        USING ERRCODE = '23514';
END;
$$;

CREATE TRIGGER admin_totp_credentials_20_no_delete
    BEFORE DELETE ON public.admin_totp_credentials
    FOR EACH ROW EXECUTE FUNCTION public.guard_admin_totp_credential_append_only();

CREATE TRIGGER admin_totp_credentials_30_no_truncate
    BEFORE TRUNCATE ON public.admin_totp_credentials
    FOR EACH STATEMENT EXECUTE FUNCTION public.guard_admin_totp_credential_append_only();

REVOKE ALL ON TABLE public.admin_totp_credentials FROM PUBLIC;
REVOKE ALL ON FUNCTION public.guard_admin_totp_credential() FROM PUBLIC;
REVOKE ALL ON FUNCTION public.guard_admin_totp_credential_append_only() FROM PUBLIC;

COMMENT ON TABLE public.admin_totp_credentials IS
    'Encrypted TOTP secret for the single internally provisioned Admin account; no plaintext secret is stored.';
