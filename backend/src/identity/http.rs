use std::{future::Future, net::IpAddr, pin::Pin, sync::Arc};

use sqlx::PgPool;

use super::{
    crypto::{random_bytes, KeyProvider},
    delivery::OtpDeliveryProvider,
    recovery::RecoveryService,
    registration::ContactChannel,
    verification::VerificationService,
};

mod admin_authentication;
pub mod authorization;
mod authentication;
mod handlers;
mod password_reset;
mod recovery;
pub use super::password::PasswordPolicy as RegistrationPasswordPolicy;
pub use admin_authentication::{admin_authentication_router, AdminAuthenticationApiState};
pub use authentication::{authentication_router, AuthenticationApiState};
pub use authorization::{require_permission, AuthorizationApiState};
pub use handlers::registration_router;
pub use recovery::recovery_router;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationAction {
    Create,
    Resend,
    Verify,
    RecoveryStart,
    RecoveryResend,
    RecoveryVerify,
    RecoveryCandidates,
    RecoverySelect,
    RecoveryPasswordResetComplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbuseDecision {
    Allowed,
    Limited,
    Unavailable,
}

/// Guard rate limit yang harus diimplementasikan sebelum router dipasang ke aplikasi publik.
///
/// Alamat IP berasal dari koneksi langsung. Jika aplikasi berada di belakang proxy,
/// integrasi harus memvalidasi proxy tepercaya sebelum meneruskan alamat klien.
pub trait RegistrationAbuseGuard: Send + Sync {
    fn check<'a>(
        &'a self,
        action: RegistrationAction,
        peer_ip: IpAddr,
        subject: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = AbuseDecision> + Send + 'a>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationPolicyVersions {
    terms_of_service: String,
    privacy_policy: String,
    marketing_notice: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationApiConfigError {
    InvalidTermsVersion,
    InvalidPrivacyVersion,
    InvalidMarketingVersion,
}

impl RegistrationPolicyVersions {
    pub fn new(
        terms_of_service: String,
        privacy_policy: String,
        marketing_notice: Option<String>,
    ) -> Result<Self, RegistrationApiConfigError> {
        if !valid_version(&terms_of_service) {
            return Err(RegistrationApiConfigError::InvalidTermsVersion);
        }
        if !valid_version(&privacy_policy) {
            return Err(RegistrationApiConfigError::InvalidPrivacyVersion);
        }
        if marketing_notice
            .as_deref()
            .is_some_and(|version| !valid_version(version))
        {
            return Err(RegistrationApiConfigError::InvalidMarketingVersion);
        }

        Ok(Self {
            terms_of_service,
            privacy_policy,
            marketing_notice,
        })
    }
}

fn valid_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.trim() == value
        && !value.chars().any(char::is_whitespace)
}

pub(super) fn valid_contact_input(
    channel: ContactChannel,
    email: Option<&str>,
    phone: Option<&str>,
) -> bool {
    match channel {
        ContactChannel::Email => match (email, phone) {
            (Some(email), None) => {
                let email = email.trim();
                let mut parts = email.split('@');
                let local = parts.next().unwrap_or_default();
                let domain = parts.next().unwrap_or_default();
                email.len() <= 254
                    && !local.is_empty()
                    && !domain.is_empty()
                    && parts.next().is_none()
                    && !email.chars().any(char::is_whitespace)
            }
            _ => false,
        },
        ContactChannel::Phone => match (email, phone) {
            (None, Some(phone)) => {
                let digits = phone.trim().strip_prefix('+').unwrap_or_default();
                (8..=15).contains(&digits.len())
                    && digits.bytes().all(|byte| byte.is_ascii_digit())
                    && !digits.starts_with('0')
            }
            _ => false,
        },
    }
}

#[derive(Clone)]
pub struct RegistrationApiState {
    pub(super) pool: PgPool,
    pub(super) keys: Arc<dyn KeyProvider>,
    pub(super) delivery: Arc<dyn OtpDeliveryProvider>,
    pub(super) abuse_guard: Arc<dyn RegistrationAbuseGuard>,
    pub(super) password_policy: Arc<dyn RegistrationPasswordPolicy>,
    pub(super) policy_versions: RegistrationPolicyVersions,
}

impl RegistrationApiState {
    pub fn new(
        pool: PgPool,
        keys: Arc<dyn KeyProvider>,
        delivery: Arc<dyn OtpDeliveryProvider>,
        abuse_guard: Arc<dyn RegistrationAbuseGuard>,
        password_policy: Arc<dyn RegistrationPasswordPolicy>,
        policy_versions: RegistrationPolicyVersions,
    ) -> Self {
        Self {
            pool,
            keys,
            delivery,
            abuse_guard,
            password_policy,
            policy_versions,
        }
    }

    pub(super) fn verification_service(&self) -> VerificationService<'_> {
        VerificationService::new(&self.pool, self.keys.as_ref(), self.delivery.as_ref())
    }
}

#[derive(Clone)]
pub struct RecoveryApiState {
    pub(super) pool: PgPool,
    pub(super) keys: Arc<dyn KeyProvider>,
    pub(super) delivery: Arc<dyn OtpDeliveryProvider>,
    pub(super) abuse_guard: Arc<dyn RegistrationAbuseGuard>,
    pub(super) password_policy: Option<Arc<dyn RegistrationPasswordPolicy>>,
}

impl RecoveryApiState {
    pub fn new(
        pool: PgPool,
        keys: Arc<dyn KeyProvider>,
        delivery: Arc<dyn OtpDeliveryProvider>,
        abuse_guard: Arc<dyn RegistrationAbuseGuard>,
    ) -> Self {
        Self {
            pool,
            keys,
            delivery,
            abuse_guard,
            password_policy: None,
        }
    }

    pub fn with_password_policy(mut self, policy: Arc<dyn RegistrationPasswordPolicy>) -> Self {
        self.password_policy = Some(policy);
        self
    }

    pub(super) fn recovery_service(&self) -> RecoveryService<'_> {
        RecoveryService::new(&self.pool, self.keys.as_ref(), self.delivery.as_ref())
    }
}

fn opaque_verification_id() -> Result<String, super::crypto::CryptoError> {
    let mut bytes = random_bytes::<16>()?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_versions_reject_empty_whitespace_and_oversized_labels() {
        assert_eq!(
            RegistrationPolicyVersions::new(String::new(), "privacy-v1".to_owned(), None),
            Err(RegistrationApiConfigError::InvalidTermsVersion)
        );
        assert_eq!(
            RegistrationPolicyVersions::new("terms-v1".to_owned(), "privacy v1".to_owned(), None),
            Err(RegistrationApiConfigError::InvalidPrivacyVersion)
        );
        assert_eq!(
            RegistrationPolicyVersions::new(
                "terms-v1".to_owned(),
                "privacy-v1".to_owned(),
                Some("m".repeat(65))
            ),
            Err(RegistrationApiConfigError::InvalidMarketingVersion)
        );
    }

    #[test]
    fn policy_versions_accept_server_managed_labels() {
        let versions = RegistrationPolicyVersions::new(
            "terms-2026-12-v1".to_owned(),
            "privacy-2026-12-v1".to_owned(),
            Some("marketing-2026-12-v1".to_owned()),
        )
        .expect("version labels should be valid");

        assert_eq!(versions.terms_of_service, "terms-2026-12-v1");
        assert_eq!(versions.privacy_policy, "privacy-2026-12-v1");
        assert_eq!(
            versions.marketing_notice.as_deref(),
            Some("marketing-2026-12-v1")
        );
    }

    #[test]
    fn contact_input_must_match_the_selected_channel() {
        assert!(valid_contact_input(
            ContactChannel::Email,
            Some(" user@example.test "),
            None
        ));
        assert!(!valid_contact_input(
            ContactChannel::Email,
            Some("user@example.test"),
            Some("+628123456789")
        ));
        assert!(valid_contact_input(
            ContactChannel::Phone,
            None,
            Some("+628123456789")
        ));
        assert!(!valid_contact_input(
            ContactChannel::Phone,
            None,
            Some("08123456789")
        ));
    }

    #[test]
    fn opaque_verification_ids_are_uuid_v4_shaped() {
        let value = opaque_verification_id().expect("opaque ID generation should succeed");
        assert_eq!(value.len(), 36);
        assert_eq!(value.as_bytes()[14], b'4');
        assert!(matches!(value.as_bytes()[19], b'8'..=b'b'));
    }
}
