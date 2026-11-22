use sqlx::PgPool;

use super::{
    authentication::session_token_hash,
    crypto::{contact_fingerprint, encrypt, key_version_label, KeyProvider},
    delivery::{DispatchStatus, OtpDeliveryProvider},
    registration::ContactChannel,
    verification::{
        contact_aliases, decrypt_destination, destination_associated_data, dispatch_otp_challenge,
        new_uuid, normalize_contact, validate_uuid, verify_otp_session, SessionRow,
        VerificationError,
    },
};

type RecoveryContextRow = (String, String, Vec<u8>, Vec<u8>, String, Vec<u8>, String);
type CandidateRow = (String, String, String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    PasswordReset,
    ForgotEmail,
    AccountAccess,
}

impl RecoveryAction {
    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::PasswordReset => "password_reset",
            Self::ForgotEmail => "forgot_email",
            Self::AccountAccess => "account_access",
        }
    }
}

pub struct RecoveryStart {
    pub case_id: String,
    pub session_id: String,
    pub dispatch_status: DispatchStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryCandidate {
    pub user_id: String,
    pub role: String,
    pub display_name: String,
}

pub struct RecoveryService<'a> {
    pool: &'a PgPool,
    keys: &'a dyn KeyProvider,
    delivery: &'a dyn OtpDeliveryProvider,
}

impl<'a> RecoveryService<'a> {
    pub fn new(
        pool: &'a PgPool,
        keys: &'a dyn KeyProvider,
        delivery: &'a dyn OtpDeliveryProvider,
    ) -> Self {
        Self {
            pool,
            keys,
            delivery,
        }
    }

    /// Membuat kasus recovery tanpa mengikat sesi pada akun sebelum kontak diverifikasi.
    pub async fn start(
        &self,
        channel: ContactChannel,
        email: Option<&str>,
        phone: Option<&str>,
        action: RecoveryAction,
    ) -> Result<RecoveryStart, VerificationError> {
        let (channel_label, destination) = normalize_contact(channel, email, phone)?;
        let session_id = new_uuid()?;
        let case_id = new_uuid()?;
        let aad = destination_associated_data(&session_id, channel_label, "account_recovery");
        let encrypted = encrypt(self.keys, destination.as_bytes(), &aad)?;
        let fingerprint = contact_fingerprint(self.keys, channel_label, &destination)?;

        let mut transaction = self.pool.begin().await?;
        let (persisted_session_id, persisted_case_id): (String, String) = sqlx::query_as(
            "SELECT session_id::text, case_id::text \
               FROM public.create_account_recovery_verification_session( \
                   $1::uuid, $2::uuid, $3, $4, $5, $6, $7, $8, $9, \
                   clock_timestamp() + INTERVAL '10 minutes')",
        )
        .bind(&session_id)
        .bind(&case_id)
        .bind(action.as_db_value())
        .bind(channel_label)
        .bind(encrypted.ciphertext)
        .bind(encrypted.nonce.to_vec())
        .bind(key_version_label(encrypted.key_version)?)
        .bind(fingerprint.digest.to_vec())
        .bind(key_version_label(fingerprint.key_version)?)
        .fetch_one(&mut *transaction)
        .await?;

        if persisted_session_id != session_id || persisted_case_id != case_id {
            return Err(VerificationError::InvalidStoredData);
        }
        transaction.commit().await?;

        let dispatch_status = dispatch_otp_challenge(
            self.pool,
            self.keys,
            self.delivery,
            &session_id,
            "account_recovery",
        )
        .await?;

        Ok(RecoveryStart {
            case_id,
            session_id,
            dispatch_status,
        })
    }

    pub async fn resend(&self, session_id: &str) -> Result<DispatchStatus, VerificationError> {
        dispatch_otp_challenge(
            self.pool,
            self.keys,
            self.delivery,
            session_id,
            "account_recovery",
        )
        .await
    }

    pub async fn verify(&self, session_id: &str, code: &str) -> Result<bool, VerificationError> {
        verify_otp_session(self.pool, self.keys, session_id, "account_recovery", code).await
    }

    /// Menampilkan kandidat hanya setelah OTP valid; identitas akun tetap diperiksa DB saat dipilih.
    pub async fn list_candidates(
        &self,
        case_id: &str,
    ) -> Result<Vec<RecoveryCandidate>, VerificationError> {
        let aliases = self.selection_context(case_id).await?;
        let alias_bytes = aliases
            .into_iter()
            .map(|alias| alias.digest.to_vec())
            .collect::<Vec<_>>();
        let candidates: Vec<CandidateRow> = sqlx::query_as(
            "SELECT user_id::text, primary_role, display_name \
               FROM public.list_account_recovery_candidates($1::uuid, $2)",
        )
        .bind(case_id)
        .bind(alias_bytes)
        .fetch_all(self.pool)
        .await?;

        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        Ok(candidates
            .into_iter()
            .map(|(user_id, role, display_name)| RecoveryCandidate {
                user_id,
                role,
                display_name,
            })
            .collect())
    }

    /// Mengikat akun setelah fingerprint kontak cocok di PostgreSQL.
    pub async fn select_account(
        &self,
        case_id: &str,
        target_user_id: &str,
    ) -> Result<String, VerificationError> {
        self.select_account_with_session(case_id, target_user_id, None)
            .await
    }

    /// Mengikat akun dan, bila tersedia, mencatat kecocokan sesi aktif yang sudah ada
    /// sebelum kasus recovery dimulai. Sesi hanya merupakan sinyal tambahan untuk review.
    pub async fn select_account_with_session(
        &self,
        case_id: &str,
        target_user_id: &str,
        session_token: Option<&str>,
    ) -> Result<String, VerificationError> {
        validate_uuid(case_id)?;
        validate_uuid(target_user_id)?;
        let aliases = self.selection_context(case_id).await?;
        let alias_bytes = aliases
            .into_iter()
            .map(|alias| alias.digest.to_vec())
            .collect::<Vec<_>>();

        let mut transaction = self.pool.begin().await?;
        let selected_case_id = sqlx::query_scalar::<_, String>(
            "SELECT public.bind_account_recovery_session_to_account($1::uuid, $2::uuid, $3)::text",
        )
        .bind(case_id)
        .bind(target_user_id)
        .bind(alias_bytes)
        .fetch_one(&mut *transaction)
        .await?;

        if let Some(token_hash) = session_token.and_then(|token| session_token_hash(token).ok()) {
            let _: bool = sqlx::query_scalar(
                "SELECT public.record_account_recovery_session_signal($1::uuid, $2)",
            )
            .bind(case_id)
            .bind(token_hash.to_vec())
            .fetch_one(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(selected_case_id)
    }

    async fn selection_context(
        &self,
        case_id: &str,
    ) -> Result<Vec<super::crypto::KeyedDigest>, VerificationError> {
        validate_uuid(case_id)?;
        let context: RecoveryContextRow = sqlx::query_as(
            "SELECT session_id::text, channel, destination_ciphertext, destination_nonce, \
                    encryption_key_version, destination_hmac, hmac_key_version \
               FROM public.get_account_recovery_selection_context($1::uuid)",
        )
        .bind(case_id)
        .fetch_one(self.pool)
        .await?;

        let session: SessionRow = (
            context.1, context.2, context.3, context.4, context.5, context.6,
        );
        let destination = decrypt_destination(self.keys, &context.0, "account_recovery", &session)?;
        let channel = match session.0.as_str() {
            "email" => "email",
            "whatsapp" => "whatsapp",
            _ => return Err(VerificationError::InvalidStoredData),
        };
        let aliases = contact_aliases(self.keys, channel, &destination, &session)?;
        Ok(aliases)
    }
}

/// Keputusan manual oleh Admin yang sudah diautentikasi pada lapisan pemanggil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryReviewDecision {
    Approve,
    Reject,
}

impl RecoveryReviewDecision {
    fn as_db_value(self) -> &'static str {
        match self {
            Self::Approve => "approved",
            Self::Reject => "rejected",
        }
    }
}

/// Kode alasan terkontrol untuk keputusan review recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryReviewReason {
    OwnershipConfirmed,
    InsufficientEvidence,
    SuspiciousActivity,
    UnsupportedAction,
    PolicyViolation,
}

impl RecoveryReviewReason {
    fn as_db_value(self) -> &'static str {
        match self {
            Self::OwnershipConfirmed => "ownership_confirmed",
            Self::InsufficientEvidence => "insufficient_evidence",
            Self::SuspiciousActivity => "suspicious_activity",
            Self::UnsupportedAction => "unsupported_action",
            Self::PolicyViolation => "policy_violation",
        }
    }
}

pub struct RecoveryReviewService<'a> {
    pool: &'a PgPool,
}

impl<'a> RecoveryReviewService<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// reviewer_user_id dan reviewer_session_token wajib berasal dari sesi Admin aktif
    /// yang sudah divalidasi server. Jangan menerima keduanya dari payload klien.
    pub async fn review_case(
        &self,
        case_id: &str,
        reviewer_user_id: &str,
        reviewer_session_token: &str,
        decision: RecoveryReviewDecision,
        reason: RecoveryReviewReason,
    ) -> Result<(), VerificationError> {
        validate_uuid(case_id)?;
        validate_uuid(reviewer_user_id)?;
        let reviewer_session_hash = session_token_hash(reviewer_session_token)
            .map_err(|_| VerificationError::InvalidInput)?;

        sqlx::query_scalar::<_, bool>(
            "SELECT public.review_account_recovery_case($1::uuid, $2::uuid, $3, $4, $5)",
        )
        .bind(case_id)
        .bind(reviewer_user_id)
        .bind(reviewer_session_hash.to_vec())
        .bind(decision.as_db_value())
        .bind(reason.as_db_value())
        .fetch_one(self.pool)
        .await?;

        Ok(())
    }
}
