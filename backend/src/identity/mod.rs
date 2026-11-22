//! Akses runtime Identity Core melalui operasi PostgreSQL yang diizinkan secara eksplisit.
//!
//! Modul ini tidak memberi akses tabel langsung. Registrasi menggunakan fungsi
//! SECURITY DEFINER yang dikendalikan database; audit menggunakan hak INSERT
//! append-only. Router HTTP registrasi tersedia sebagai komponen terinjeksi, tetapi
//! sengaja belum dipasang pada aplikasi runtime.

pub mod abuse;
mod audit;
pub mod authentication;
pub mod crypto;
pub mod delivery;
pub mod http;
pub mod password;
mod registration;
pub mod verification;

pub use abuse::PostgresRegistrationAbuseGuard;
pub use audit::{append_audit_event, AuditActorType, AuditEvent, AuditOutcome};
pub use authentication::{
    AuthenticatedSession, AuthenticationConfigError, AuthenticationError,
    AuthenticationRateLimitKey, AuthenticationService, LoginRole, LoginSession, RotatedSession,
};
pub use crypto::{EnvironmentKeyProvider, EnvironmentKeyProviderError};
pub use delivery::{DeliveryChannel, DispatchStatus, OtpDeliveryMessage, OtpDeliveryProvider};
pub use registration::{create_client_user, ClientRegistration, ClientRole, ContactChannel};

pub use verification::{RegistrationStart, VerificationError, VerificationService};
