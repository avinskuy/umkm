-- Persist only privacy-preserving device/session signals at password login.

DROP FUNCTION public.create_password_login_session(UUID, BYTEA);

CREATE FUNCTION public.create_password_login_session(
    p_user_id UUID,
    p_token_hash BYTEA,
    p_remember_device BOOLEAN,
    p_user_agent_summary TEXT,
    p_ip_prefix_hash BYTEA,
    p_ip_prefix_hmac_key_version TEXT
)
RETURNS UUID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_session_id UUID;
BEGIN
    IF p_user_id IS NULL
       OR p_token_hash IS NULL
       OR octet_length(p_token_hash) <> 32
       OR p_remember_device IS NULL
       OR p_ip_prefix_hash IS NULL
       OR octet_length(p_ip_prefix_hash) <> 32
       OR p_ip_prefix_hmac_key_version IS NULL
       OR length(btrim(p_ip_prefix_hmac_key_version)) = 0
       OR (
           p_user_agent_summary IS NOT NULL
           AND (
               length(p_user_agent_summary) NOT BETWEEN 1 AND 256
               OR p_user_agent_summary <> btrim(p_user_agent_summary)
               OR p_user_agent_summary ~ '[[:cntrl:]]'
           )
       ) THEN
        RAISE EXCEPTION 'password login session device metadata is invalid'
            USING ERRCODE = '22023';
    END IF;

    INSERT INTO public.sessions (
        user_id,
        token_hash,
        remember_device,
        user_agent_summary,
        ip_prefix_hash,
        ip_prefix_hmac_key_version
    )
    SELECT
        u.id,
        p_token_hash,
        p_remember_device,
        p_user_agent_summary,
        p_ip_prefix_hash,
        p_ip_prefix_hmac_key_version
      FROM public.users AS u
     WHERE u.id = p_user_id
       AND u.account_status = 'active'
       AND u.password_hash IS NOT NULL
    RETURNING id INTO v_session_id;

    IF v_session_id IS NULL THEN
        RAISE EXCEPTION 'active password account was not found'
            USING ERRCODE = 'P0002';
    END IF;

    RETURN v_session_id;
END;
$$;

REVOKE ALL ON FUNCTION public.create_password_login_session(
    UUID, BYTEA, BOOLEAN, TEXT, BYTEA, TEXT
) FROM PUBLIC;

COMMENT ON FUNCTION public.create_password_login_session(
    UUID, BYTEA, BOOLEAN, TEXT, BYTEA, TEXT
) IS
    'Creates a password session with user-consented remember-device state, sanitized user-agent summary, and versioned HMAC of a coarse IP prefix; raw IP is never stored.';
