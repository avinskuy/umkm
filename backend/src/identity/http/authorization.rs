//! Axum permission middleware. Run after the relevant session middleware.
//! Resource attributes must be attached by trusted server-side repository code.
use super::super::authorization::{
    evaluate, AccessDecision, AccessPurpose, AuthorizationContext, AuthorizationResource,
    AuthorizationRole, DenialReason, Permission,
};
use crate::identity::{
    append_audit_event, AdminAuthenticatedSession, AuditActorType, AuditEvent, AuditOutcome,
    AuthenticatedSession, LoginRole,
};
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use sqlx::PgPool;

#[derive(Clone)]
pub struct AuthorizationApiState {
    pool: PgPool,
    permission: Permission,
    purpose: AccessPurpose,
}
impl AuthorizationApiState {
    pub fn new(pool: PgPool, permission: Permission, purpose: AccessPurpose) -> Self {
        Self {
            pool,
            permission,
            purpose,
        }
    }
}
#[derive(serde::Serialize)]
struct AuthorizationError {
    error: &'static str,
}
fn denied(status: StatusCode, message: &'static str) -> Response {
    (status, Json(AuthorizationError { error: message })).into_response()
}
fn actor(request: &Request) -> Option<(String, AuthorizationRole, AuditActorType)> {
    if let Some(session) = request.extensions().get::<AuthenticatedSession>() {
        let role = match session.role {
            LoginRole::BusinessOwner => AuthorizationRole::BusinessOwner,
            LoginRole::Investor => AuthorizationRole::Investor,
        };
        return Some((session.user_id.clone(), role, AuditActorType::User));
    }
    request
        .extensions()
        .get::<AdminAuthenticatedSession>()
        .map(|session| {
            (
                session.user_id.clone(),
                AuthorizationRole::Admin,
                AuditActorType::Admin,
            )
        })
}
struct AuthorizationAuditDecision<'a> {
    actor_id: &'a str,
    actor_type: AuditActorType,
    action_code: &'static str,
    outcome: AuditOutcome,
    reason_code: &'static str,
    result_code: &'static str,
    resource: Option<&'a AuthorizationResource>,
}

async fn append_decision_audit(
    state: &AuthorizationApiState,
    decision: AuthorizationAuditDecision<'_>,
) -> Result<(), sqlx::Error> {
    let mut transaction = state.pool.begin().await?;
    let safe_type = decision
        .resource
        .map(|resource| resource.resource_type.as_str())
        .filter(|value| !value.is_empty() && value.len() <= 64);
    let safe_id = decision
        .resource
        .map(|resource| resource.resource_id.as_str())
        .filter(|value| !value.is_empty() && value.len() <= 128);
    let event = AuditEvent {
        actor_type: decision.actor_type,
        actor_id: Some(decision.actor_id),
        subject_user_id: None,
        action_code: decision.action_code,
        outcome: decision.outcome,
        reason_code: Some(decision.reason_code),
        request_id: None,
        metadata: serde_json::json!({
            "purpose": state.purpose.code(),
            "policy_code": state.permission.code(),
            "resource_type": safe_type,
            "resource_id": safe_id,
            "result_code": decision.result_code
        }),
    };
    append_audit_event(&mut transaction, &event).await?;
    transaction.commit().await
}

/// Use as a route layer after authentication; missing session/resource context denies access.
pub async fn require_permission(
    State(state): State<AuthorizationApiState>,
    request: Request,
    next: Next,
) -> Response {
    let Some((actor_id, role, actor_type)) = actor(&request) else {
        return denied(StatusCode::UNAUTHORIZED, "Sesi autentikasi diperlukan.");
    };
    let resource = request.extensions().get::<AuthorizationResource>().cloned();
    let decision = evaluate(&AuthorizationContext {
        actor_id: &actor_id,
        role,
        permission: state.permission,
        purpose: state.purpose,
        resource: resource.as_ref(),
    });
    match decision {
        AccessDecision::Allow => {
            if role == AuthorizationRole::Admin
                && append_decision_audit(
                    &state,
                    AuthorizationAuditDecision {
                        actor_id: &actor_id,
                        actor_type,
                        action_code: "authorization.admin.access.granted",
                        outcome: AuditOutcome::Success,
                        reason_code: "policy_allowed",
                        result_code: "allowed",
                        resource: resource.as_ref(),
                    },
                )
                .await
                .is_err()
            {
                return denied(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Audit otorisasi tidak tersedia.",
                );
            }
            next.run(request).await
        }
        AccessDecision::Deny(reason) => {
            if append_decision_audit(
                &state,
                AuthorizationAuditDecision {
                    actor_id: &actor_id,
                    actor_type,
                    action_code: "authorization.permission.denied",
                    outcome: AuditOutcome::Denied,
                    reason_code: "permission_denied",
                    result_code: match reason {
                        DenialReason::RoleNotAllowed => "role_not_allowed",
                        DenialReason::ResourceContextRequired => "resource_context_required",
                        DenialReason::ResourceTypeMismatch => "resource_type_mismatch",
                        DenialReason::ResourceInactive => "resource_inactive",
                        DenialReason::NotOwnerOrPublic => "not_owner_or_public",
                        DenialReason::ConnectionRequired => "connection_required",
                        DenialReason::ConsentRequired => "consent_required",
                        DenialReason::ClassificationRestricted => "classification_restricted",
                        DenialReason::AdminPurposeRequired => "admin_purpose_required",
                        DenialReason::AdminScopeRequired => "admin_scope_required",
                    },
                    resource: resource.as_ref(),
                },
            )
            .await
            .is_err()
            {
                return denied(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Otorisasi tidak dapat diverifikasi.",
                );
            }
            denied(
                StatusCode::FORBIDDEN,
                "Akses ke resource ini tidak diizinkan.",
            )
        }
    }
}
