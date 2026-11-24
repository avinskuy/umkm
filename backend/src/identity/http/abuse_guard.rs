use std::net::IpAddr;

use sqlx::PgPool;
use zeroize::Zeroizing;

use super::{AbuseDecision, RegistrationAbuseGuard, RegistrationAction};
use crate::identity::authentication::{
    ip_rate_limit_subject, AuthenticationRateLimitKey,
};

const WINDOW_SECONDS: i32 = 15 * 60;

#[derive(Clone)]
pub struct PostgresRegistrationAbuseGuard {
    pool: PgPool,
    key: AuthenticationRateLimitKey,
}

impl PostgresRegistrationAbuseGuard {
    pub fn new(pool: PgPool, key: AuthenticationRateLimitKey) -> Self {
        Self { pool, key }
    }

    fn limits(action: RegistrationAction) -> (&'static str, i32, Option<i32>) {
        match action {
            RegistrationAction::Create => ("registration:create", 10, None),
            RegistrationAction::Resend => ("registration:resend", 30, Some(10)),
            RegistrationAction::Verify => ("registration:verify", 60, Some(10)),
            RegistrationAction::RecoveryStart => ("recovery:start", 10, None),
            RegistrationAction::RecoveryResend => ("recovery:resend", 30, Some(10)),
            RegistrationAction::RecoveryVerify => ("recovery:verify", 30, Some(10)),
            RegistrationAction::RecoveryCandidates => ("recovery:candidates", 30, Some(10)),
            RegistrationAction::RecoverySelect => ("recovery:select", 30, Some(10)),
            RegistrationAction::RecoveryPasswordResetComplete => {
                ("recovery:password_reset_complete", 30, Some(10))
            }
        }
    }

    async fn reserve(
        &self,
        scope: &str,
        fingerprint: &[u8; 32],
        limit: i32,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT public.reserve_identity_abuse_attempt($1, $2, $3, $4)",
        )
        .bind(scope)
        .bind(fingerprint.as_slice())
        .bind(limit)
        .bind(WINDOW_SECONDS)
        .fetch_one(&self.pool)
        .await
    }

    fn fingerprint(&self, scope: &str, value: &[u8]) -> [u8; 32] {
        self.key.fingerprint(
            format!("usahapintar:identity-abuse:{scope}:v1").as_bytes(),
            value,
        )
    }
}

impl RegistrationAbuseGuard for PostgresRegistrationAbuseGuard {
    fn check<'a>(
        &'a self,
        action: RegistrationAction,
        peer_ip: IpAddr,
        subject: Option<&'a str>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = AbuseDecision> + Send + 'a>> {
        Box::pin(async move {
            let (action_scope, ip_limit, subject_limit) = Self::limits(action);
            let ip_bytes = Zeroizing::new(ip_rate_limit_subject(peer_ip));
            let ip_fingerprint = self.fingerprint(&format!("{action_scope}:ip"), &ip_bytes);

            match self
                .reserve(&format!("{action_scope}:ip"), &ip_fingerprint, ip_limit)
                .await
            {
                Ok(true) => {}
                Ok(false) => return AbuseDecision::Limited,
                Err(_) => return AbuseDecision::Unavailable,
            }

            if let (Some(subject), Some(limit)) = (subject, subject_limit) {
                if subject.is_empty() || subject.len() > 256 {
                    return AbuseDecision::Limited;
                }
                let subject_fingerprint =
                    self.fingerprint(&format!("{action_scope}:subject"), subject.as_bytes());
                match self
                    .reserve(
                        &format!("{action_scope}:subject"),
                        &subject_fingerprint,
                        limit,
                    )
                    .await
                {
                    Ok(true) => {}
                    Ok(false) => return AbuseDecision::Limited,
                    Err(_) => return AbuseDecision::Unavailable,
                }
            }

            AbuseDecision::Allowed
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_limits_are_explicit_and_keep_recovery_start_separate() {
        assert_eq!(
            PostgresRegistrationAbuseGuard::limits(RegistrationAction::Create),
            ("registration:create", 10, None)
        );
        assert_eq!(
            PostgresRegistrationAbuseGuard::limits(RegistrationAction::Verify),
            ("registration:verify", 60, Some(10))
        );
        assert_eq!(
            PostgresRegistrationAbuseGuard::limits(RegistrationAction::RecoveryStart),
            ("recovery:start", 10, None)
        );
    }

    #[test]
    fn fingerprints_are_stable_and_scope_separated() {
        let key = AuthenticationRateLimitKey::from_hex(&"ab".repeat(32)).unwrap();
        let guard = PostgresRegistrationAbuseGuard::new(
            PgPool::connect_lazy("postgres://localhost/unused").unwrap(),
            key,
        );
        let first = guard.fingerprint("registration:verify:ip", b"192.0.2.1");
        let same = guard.fingerprint("registration:verify:ip", b"192.0.2.1");
        let other_scope = guard.fingerprint("recovery:verify:ip", b"192.0.2.1");
        assert_eq!(first, same);
        assert_ne!(first, other_scope);
    }
}
