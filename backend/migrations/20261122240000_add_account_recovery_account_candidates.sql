-- Account recovery candidate discovery after successful contact verification.

CREATE FUNCTION public.get_account_recovery_selection_context(p_case_id UUID)
RETURNS TABLE (
    session_id UUID,
    channel TEXT,
    destination_ciphertext BYTEA,
    destination_nonce BYTEA,
    encryption_key_version TEXT,
    destination_hmac BYTEA,
    hmac_key_version TEXT
)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
BEGIN
    RETURN QUERY
    SELECT s.id, s.channel, s.destination_ciphertext, s.destination_nonce,
           s.encryption_key_version, s.destination_hmac, s.hmac_key_version
      FROM public.account_recovery_cases AS c
      JOIN public.verification_sessions AS s
        ON s.id = c.verification_session_id
     WHERE c.id = p_case_id
       AND c.status = 'account_selection_required'
       AND c.target_user_id IS NULL
       AND s.purpose = 'account_recovery'
       AND s.status = 'verified'
       AND s.user_id IS NULL
       AND s.expires_at > clock_timestamp();

    IF NOT FOUND THEN
        RAISE EXCEPTION 'account recovery selection is not eligible'
            USING ERRCODE = 'P0002';
    END IF;
END;
$$;

CREATE FUNCTION public.list_account_recovery_candidates(
    p_case_id UUID,
    p_destination_hmac_aliases BYTEA[]
)
RETURNS TABLE (user_id UUID, primary_role TEXT, display_name TEXT)
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
DECLARE
    v_session_id UUID;
    v_channel TEXT;
    v_destination_hmac BYTEA;
BEGIN
    IF p_case_id IS NULL
       OR p_destination_hmac_aliases IS NULL
       OR cardinality(p_destination_hmac_aliases) = 0
       OR EXISTS (
           SELECT 1
             FROM pg_catalog.unnest(p_destination_hmac_aliases) AS h(value)
            WHERE h.value IS NULL OR octet_length(h.value) <> 32
       ) THEN
        RAISE EXCEPTION 'account recovery candidates are not available'
            USING ERRCODE = '22023';
    END IF;

    SELECT s.id, s.channel, s.destination_hmac
      INTO v_session_id, v_channel, v_destination_hmac
      FROM public.account_recovery_cases AS c
      JOIN public.verification_sessions AS s
        ON s.id = c.verification_session_id
     WHERE c.id = p_case_id
       AND c.status = 'account_selection_required'
       AND c.target_user_id IS NULL
       AND s.purpose = 'account_recovery'
       AND s.status = 'verified'
       AND s.user_id IS NULL
       AND s.expires_at > clock_timestamp();

    IF NOT FOUND OR NOT (v_destination_hmac = ANY(p_destination_hmac_aliases)) THEN
        RAISE EXCEPTION 'account recovery candidates are not available'
            USING ERRCODE = 'P0002';
    END IF;

    RETURN QUERY
    SELECT DISTINCT u.id, u.primary_role, profile.display_name
      FROM public.recovery_contacts AS rc
      JOIN public.users AS u ON u.id = rc.user_id
      JOIN public.user_profiles AS profile ON profile.user_id = u.id
     WHERE rc.channel = v_channel
       AND rc.contact_hmac = ANY(p_destination_hmac_aliases)
       AND rc.revoked_at IS NULL
       AND u.account_status = 'active'
     ORDER BY profile.display_name, u.id;
END;
$$;

REVOKE ALL ON FUNCTION public.get_account_recovery_selection_context(UUID) FROM PUBLIC;
REVOKE ALL ON FUNCTION public.list_account_recovery_candidates(UUID, BYTEA[]) FROM PUBLIC;

COMMENT ON FUNCTION public.get_account_recovery_selection_context(UUID) IS
    'Returns encrypted contact context only for a verified, unbound recovery case awaiting account selection.';
COMMENT ON FUNCTION public.list_account_recovery_candidates(UUID, BYTEA[]) IS
    'Lists active accounts sharing the verified recovery contact; caller must prove the same HMAC aliases as the recovery session.';
