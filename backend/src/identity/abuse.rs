use std::{future::Future, net::IpAddr, pin::Pin};

use sqlx::PgPool;

use super::{
    authentication::{ip_rate_limit_subject, AuthenticationRateLimitKey},
    http::{AbuseDecision, RegistrationAbuseGuard, RegistrationAction},
};

/// Guard registrasi yang menyimpan fingerprint HMAC, bukan alamat IP atau ID sesi mentah.
pub struct PostgresRegistrationAbuseGuard {
    pool: PgPool,
    key: AuthenticationRateLimitKey,
}

impl PostgresRegistrationAbuseGuard {
    pub fn new(pool: PgPool, key: AuthenticationRateLimitKey) -> Self {
        Self { pool, key }
    }
}

impl RegistrationAbuseGuard for PostgresRegistrationAbuseGuard {
    fn check<'a>(
        &'a self,
        action: RegistrationAction,
        peer_ip: IpAddr,
        subject: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = AbuseDecision> + Send + 'a>> {
        let action = match action {
            RegistrationAction::Create => "create",
            RegistrationAction::Resend => "resend",
            RegistrationAction::Verify => "verify",
        };
        let ip_subject = ip_rate_limit_subject(peer_ip);
        let ip_hash = self
            .key
            .fingerprint(b"usahapintar:registration-ip:v1", &ip_subject);
        let subject_hash = subject.map(|value| {
            self.key
                .fingerprint(b"usahapintar:registration-subject:v1", value.as_bytes())
                .to_vec()
        });
        let pool = &self.pool;

        Box::pin(async move {
            match sqlx::query_scalar::<_, bool>(
                "SELECT public.reserve_registration_http_attempt($1, $2, $3)",
            )
            .bind(action)
            .bind(ip_hash.to_vec())
            .bind(subject_hash)
            .fetch_one(pool)
            .await
            {
                Ok(true) => AbuseDecision::Allowed,
                Ok(false) => AbuseDecision::Limited,
                Err(_) => AbuseDecision::Unavailable,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_fingerprints_use_a_stable_prefix_subject() {
        let ipv4: IpAddr = "192.0.2.10".parse().unwrap();
        let ipv6_a: IpAddr = "2001:db8:abcd:12::1".parse().unwrap();
        let ipv6_b: IpAddr = "2001:db8:abcd:12::ffff".parse().unwrap();

        assert_eq!(ip_rate_limit_subject(ipv4).len(), 5);
        assert_eq!(ip_rate_limit_subject(ipv6_a), ip_rate_limit_subject(ipv6_b));
    }
}
