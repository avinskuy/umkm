//! Protected Admin endpoint for manual account-recovery review.
//! The route is mounted only inside the isolated Admin authentication router.

use axum::{
    extract::{DefaultBodyLimit, Extension, Path, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use super::{
    admin_authentication::{
        is_same_origin_request, require_admin_session, AdminAuthenticationApiState,
        AdminSessionTokenHash,
    },
    authorization::AuthorizationApiState,
};
use crate::identity::{
    AccessPurpose, AdminAuthenticatedSession, AuthorizationResource, DataClassification,
    Permission, RecoveryReviewDecision, RecoveryReviewReason, RecoveryReviewService,
    ResourceVisibility, VerificationError,
};

const MAX_REQUEST_BYTES: usize = 8 * 1024;
const REVIEW_ROUTE: &str = "/api/v1/identity/admin/recovery/cases/{case_id}/review";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewRequest {
    decision: String,
    reason_code: String,
}

#[derive(Serialize)]
struct MessageResponse {
    message: &'static str,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: &'static str,
}

fn error(status: StatusCode, message: &'static str) -> Response {
    let mut response = (status, Json(ErrorResponse { error: message })).into_response();
    no_store(&mut response);
    response
}

fn message(status: StatusCode, value: &'static str) -> Response {
    let mut response = (status, Json(MessageResponse { message: value })).into_response();
    no_store(&mut response);
    response
}

fn no_store(response: &mut Response) {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
}

fn valid_uuid_shape(value: &str) -> bool {
    value.len() == 36
        && [8, 13, 18, 23]
            .into_iter()
            .all(|position| value.as_bytes().get(position) == Some(&b'-'))
        && value
            .bytes()
            .enumerate()
            .all(|(position, byte)| {
                matches!(position, 8 | 13 | 18 | 23) || byte.is_ascii_hexdigit()
            })
}

fn decision_from_request(value: &str) -> Option<RecoveryReviewDecision> {
    match value {
        "approve" => Some(RecoveryReviewDecision::Approve),
        "reject" => Some(RecoveryReviewDecision::Reject),
        _ => None,
    }
}

fn reason_from_request(value: &str) -> Option<RecoveryReviewReason> {
    match value {
        "ownership_confirmed" => Some(RecoveryReviewReason::OwnershipConfirmed),
        "insufficient_evidence" => Some(RecoveryReviewReason::InsufficientEvidence),
        "suspicious_activity" => Some(RecoveryReviewReason::SuspiciousActivity),
        "unsupported_action" => Some(RecoveryReviewReason::UnsupportedAction),
        "policy_violation" => Some(RecoveryReviewReason::PolicyViolation),
        _ => None,
    }
}

/// Attach resource context from a narrow SECURITY DEFINER loader, never from client claims.
async fn load_recovery_resource(
    State(pool): State<PgPool>,
    Path(case_id): Path<String>,
    mut request: Request,
    next: Next,
) -> Response {
    if !valid_uuid_shape(&case_id) {
        return error(StatusCode::BAD_REQUEST, "ID kasus pemulihan tidak valid.");
    }

    let Some(session) = request
        .extensions()
        .get::<AdminAuthenticatedSession>()
        .cloned()
    else {
        return error(StatusCode::UNAUTHORIZED, "Sesi Admin diperlukan.");
    };
    let Some(token_hash) = request
        .extensions()
        .get::<AdminSessionTokenHash>()
        .copied()
    else {
        return error(StatusCode::UNAUTHORIZED, "Sesi Admin diperlukan.");
    };

    let context = sqlx::query_as::<_, (String, Option<String>, String, Option<String>, bool)>(
        "SELECT resource_id::text, target_user_id::text, case_status, \
                target_account_status, admin_scope_granted \
           FROM public.get_account_recovery_review_context($1::uuid, $2::uuid, $3)",
    )
    .bind(&case_id)
    .bind(&session.user_id)
    .bind(token_hash.0.to_vec())
    .fetch_optional(&pool)
    .await;

    let (resource_id, target_user_id, case_status, target_status, admin_scope_granted) =
        match context {
            Ok(Some(context)) => context,
            Ok(None) => {
                return error(StatusCode::NOT_FOUND, "Kasus pemulihan tidak ditemukan.");
            }
            Err(sqlx::Error::Database(database_error))
                if database_error.code().as_deref() == Some("22P02") =>
            {
                return error(StatusCode::BAD_REQUEST, "ID kasus pemulihan tidak valid.");
            }
            Err(_) => {
                return error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Konteks review pemulihan tidak tersedia.",
                );
            }
        };

    let resource = AuthorizationResource {
        resource_type: "recovery_case".to_owned(),
        resource_id,
        owner_user_id: target_user_id,
        classification: DataClassification::Confidential,
        visibility: ResourceVisibility::Private,
        connected: false,
        consent_granted: false,
        admin_scope_granted,
        active: case_status == "under_review" && target_status.as_deref() == Some("active"),
    };
    request.extensions_mut().insert(resource);
    next.run(request).await
}

fn review_error(error_value: VerificationError) -> Response {
    match error_value {
        VerificationError::InvalidInput => {
            error(StatusCode::BAD_REQUEST, "Data review pemulihan tidak valid.")
        }
        VerificationError::Database(sqlx::Error::Database(database_error))
            if database_error.code().as_deref() == Some("42501") =>
        {
            error(StatusCode::FORBIDDEN, "Sesi Admin tidak berwenang melakukan review.")
        }
        VerificationError::Database(sqlx::Error::Database(database_error))
            if database_error.code().as_deref() == Some("P0002") =>
        {
            error(
                StatusCode::CONFLICT,
                "Kasus pemulihan sudah berubah atau tidak lagi dapat direview.",
            )
        }
        VerificationError::Database(sqlx::Error::Database(database_error))
            if database_error.code().as_deref() == Some("22023") =>
        {
            error(
                StatusCode::BAD_REQUEST,
                "Tindakan pemulihan ini belum dapat disetujui.",
            )
        }
        _ => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Review pemulihan sementara tidak tersedia.",
        ),
    }
}

async fn review_case(
    State(pool): State<PgPool>,
    Extension(session): Extension<AdminAuthenticatedSession>,
    Extension(token_hash): Extension<AdminSessionTokenHash>,
    Path(case_id): Path<String>,
    headers: HeaderMap,
    payload: Result<Json<ReviewRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if !is_same_origin_request(&headers) {
        return error(
            StatusCode::FORBIDDEN,
            "Permintaan lintas situs tidak diizinkan.",
        );
    }

    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(_) => return error(StatusCode::BAD_REQUEST, "Data review pemulihan tidak valid."),
    };
    let Some(decision) = decision_from_request(&request.decision) else {
        return error(StatusCode::BAD_REQUEST, "Keputusan review tidak valid.");
    };
    let Some(reason) = reason_from_request(&request.reason_code) else {
        return error(StatusCode::BAD_REQUEST, "Alasan review tidak valid.");
    };

    if (decision == RecoveryReviewDecision::Approve
        && reason != RecoveryReviewReason::OwnershipConfirmed)
        || (decision == RecoveryReviewDecision::Reject
            && reason == RecoveryReviewReason::OwnershipConfirmed)
    {
        return error(
            StatusCode::BAD_REQUEST,
            "Alasan review tidak sesuai dengan keputusan.",
        );
    }

    match RecoveryReviewService::new(&pool)
        .review_case(
            &case_id,
            &session.user_id,
            &token_hash.0,
            decision,
            reason,
        )
        .await
    {
        Ok(()) => message(StatusCode::OK, "Keputusan review berhasil dicatat."),
        Err(error_value) => review_error(error_value),
    }
}

pub(super) fn recovery_review_router(admin_state: AdminAuthenticationApiState) -> Router {
    let pool = admin_state.pool.clone();

    Router::new()
        .route(REVIEW_ROUTE, post(review_case))
        .route_layer(middleware::from_fn_with_state(
            AuthorizationApiState::new(
                pool.clone(),
                Permission::AdminRecoveryReview,
                AccessPurpose::RecoveryReview,
            ),
            super::authorization::require_permission,
        ))
        .route_layer(middleware::from_fn_with_state(
            pool.clone(),
            load_recovery_resource,
        ))
        .route_layer(middleware::from_fn_with_state(
            admin_state,
            require_admin_session,
        ))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .with_state(pool)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_case_path_requires_uuid_shape() {
        assert!(valid_uuid_shape(
            "00000000-0000-4000-8000-000000000001"
        ));
        assert!(!valid_uuid_shape("not-a-uuid"));
        assert!(!valid_uuid_shape(
            "00000000-0000-4000-8000-00000000000z"
        ));
        assert!(!valid_uuid_shape(
            "00000000000040008000000000000001"
        ));
    }

    #[test]
    fn review_decision_and_reason_codes_are_allowlisted() {
        assert_eq!(
            decision_from_request("approve"),
            Some(RecoveryReviewDecision::Approve)
        );
        assert_eq!(
            decision_from_request("reject"),
            Some(RecoveryReviewDecision::Reject)
        );
        assert_eq!(decision_from_request("delete"), None);

        assert_eq!(
            reason_from_request("ownership_confirmed"),
            Some(RecoveryReviewReason::OwnershipConfirmed)
        );
        assert_eq!(
            reason_from_request("insufficient_evidence"),
            Some(RecoveryReviewReason::InsufficientEvidence)
        );
        assert_eq!(reason_from_request("free_text"), None);
    }
}
