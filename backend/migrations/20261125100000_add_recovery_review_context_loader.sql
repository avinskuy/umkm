-- Narrow server-side context loader for Admin recovery review authorization.
-- The application receives only the case identifiers/status and a boolean scope decision;
-- it never receives direct SELECT access to recovery or Admin session tables.

CREATE FUNCTION public.get_account_recovery_review_context(
    p_case_id UUID,
    p_reviewer_user_id UUID,
    p_reviewer_session_token_hash BYTEA
)
RETURNS TABLE (
    resource_id UUID,
    target_user_id UUID,
    case_status TEXT,
    target_account_status TEXT,
    admin_scope_granted BOOLEAN
)
LANGUAGE SQL
VOLATILE
SECURITY DEFINER
SET search_path = pg_catalog
AS $$
    SELECT
        recovery_case.id,
        recovery_case.target_user_id,
        recovery_case.status,
        target.account_status,
        COALESCE(
            recovery_case.status = 'under_review'
            AND target.account_status = 'active'
            AND recovery_case.target_user_id <> p_reviewer_user_id,
            FALSE
        ) AS admin_scope_granted
    FROM public.account_recovery_cases AS recovery_case
    JOIN public.users AS reviewer
      ON reviewer.id = p_reviewer_user_id
     AND reviewer.primary_role = 'admin'
     AND reviewer.account_status = 'active'
    JOIN public.admin_sessions AS admin_session
      ON admin_session.user_id = reviewer.id
     AND admin_session.token_hash = p_reviewer_session_token_hash
     AND admin_session.revoked_at IS NULL
     AND admin_session.expires_at > clock_timestamp()
     AND admin_session.last_seen_at > clock_timestamp() - INTERVAL '7 days'
    LEFT JOIN public.users AS target
      ON target.id = recovery_case.target_user_id
    WHERE recovery_case.id = p_case_id
      AND p_case_id IS NOT NULL
      AND p_reviewer_user_id IS NOT NULL
      AND p_reviewer_session_token_hash IS NOT NULL
      AND octet_length(p_reviewer_session_token_hash) = 32;
$$;

REVOKE ALL ON FUNCTION public.get_account_recovery_review_context(UUID, UUID, BYTEA) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.get_account_recovery_review_context(UUID, UUID, BYTEA)
    TO usahapintar_app;

COMMENT ON FUNCTION public.get_account_recovery_review_context(UUID, UUID, BYTEA) IS
    'Returns minimal recovery-case authorization context only for an active Admin and matching active Admin session; does not grant direct table access.';
