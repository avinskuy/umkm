use serde_json::Value;
use sqlx::PgConnection;

/// Jenis actor yang diizinkan oleh kontrak audit Identity Core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditActorType {
    User,
    Admin,
    System,
    Operator,
}

impl AuditActorType {
    fn as_db_value(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Admin => "admin",
            Self::System => "system",
            Self::Operator => "operator",
        }
    }
}

/// Outcome audit yang diizinkan oleh kontrak database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOutcome {
    Success,
    Failure,
    Denied,
}

impl AuditOutcome {
    fn as_db_value(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Denied => "denied",
        }
    }
}

/// Metadata audit hanya boleh berisi key allowlist dan nilai skalar non-sensitif.
pub struct AuditEvent<'a> {
    pub actor_type: AuditActorType,
    pub actor_id: Option<&'a str>,
    pub subject_user_id: Option<&'a str>,
    pub action_code: &'a str,
    pub outcome: AuditOutcome,
    pub reason_code: Option<&'a str>,
    pub request_id: Option<&'a str>,
    pub metadata: Value,
}

/// Menambahkan event audit tanpa menyediakan operasi baca/perubahan/penghapusan.
pub async fn append_audit_event(
    connection: &mut PgConnection,
    event: &AuditEvent<'_>,
) -> Result<String, sqlx::Error> {
    let metadata = event.metadata.to_string();

    sqlx::query_scalar::<_, String>(
        "INSERT INTO public.identity_audit_events          (actor_type, actor_id, subject_user_id, action_code, outcome, reason_code, request_id, metadata)          VALUES ($1, $2::uuid, $3::uuid, $4, $5, $6, $7::uuid, $8::jsonb)          RETURNING id::text",
    )
    .bind(event.actor_type.as_db_value())
    .bind(event.actor_id)
    .bind(event.subject_user_id)
    .bind(event.action_code)
    .bind(event.outcome.as_db_value())
    .bind(event.reason_code)
    .bind(event.request_id)
    .bind(metadata)
    .fetch_one(connection)
    .await
}
