//! Akses runtime Identity Core melalui operasi PostgreSQL yang diizinkan secara eksplisit.
//!
//! Modul ini tidak memberi akses tabel langsung. Registrasi menggunakan fungsi
//! SECURITY DEFINER yang dikendalikan database; audit menggunakan hak INSERT
//! append-only. Endpoint HTTP dan autentikasi belum termasuk dalam modul ini.

mod audit;
pub mod crypto;
pub mod delivery;
pub mod password;
mod registration;
pub mod verification;

pub use audit::{append_audit_event, AuditActorType, AuditEvent, AuditOutcome};
pub use delivery::{DeliveryChannel, DispatchStatus, OtpDeliveryMessage, OtpDeliveryProvider};
pub use registration::{create_client_user, ClientRegistration, ClientRole, ContactChannel};

pub use verification::{RegistrationStart, VerificationError, VerificationService};
