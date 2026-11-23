//! RFC 6238 TOTP primitives for the single Admin account.
//!
//! This module only verifies codes and generates enrollment secrets. Persistence,
//! encrypted storage and replay-counter consumption are handled by the identity
//! database runtime and must remain atomic.

use ring::{
    constant_time,
    hmac,
    rand::{SecureRandom, SystemRandom},
};
use zeroize::Zeroizing;

pub const TOTP_SECRET_BYTES: usize = 20;
pub const TOTP_DIGITS: usize = 6;
pub const TOTP_PERIOD_SECONDS: u64 = 30;
const TOTP_MODULUS: u32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TotpError {
    InvalidSecret,
    InvalidCode,
    RandomnessUnavailable,
    GenerationFailed,
}

impl std::fmt::Display for TotpError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidSecret => "TOTP secret is invalid",
            Self::InvalidCode => "TOTP code is invalid",
            Self::RandomnessUnavailable => "secure randomness is unavailable",
            Self::GenerationFailed => "TOTP code generation failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for TotpError {}

/// Generate a 160-bit TOTP secret using the operating system CSPRNG.
pub fn generate_secret() -> Result<Zeroizing<[u8; TOTP_SECRET_BYTES]>, TotpError> {
    let mut secret = Zeroizing::new([0_u8; TOTP_SECRET_BYTES]);
    SystemRandom::new()
        .fill(&mut secret[..])
        .map_err(|_| TotpError::RandomnessUnavailable)?;
    Ok(secret)
}

/// Generate the six-digit code for a specific RFC 6238 time-step.
pub fn code_for_step(secret: &[u8], step: u64) -> Result<String, TotpError> {
    if secret.len() < TOTP_SECRET_BYTES {
        return Err(TotpError::InvalidSecret);
    }

    let key = hmac::Key::new(&hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, secret);
    let digest = hmac::sign(&key, &step.to_be_bytes());
    let bytes = digest.as_ref();
    let offset = usize::from(bytes[bytes.len() - 1] & 0x0f);

    let binary = (u32::from(bytes[offset] & 0x7f) << 24)
        | (u32::from(bytes[offset + 1]) << 16)
        | (u32::from(bytes[offset + 2]) << 8)
        | u32::from(bytes[offset + 3]);
    let code = binary % TOTP_MODULUS;

    Ok(format!("{code:06}"))
}

/// Return the matching time-step within a one-step clock-drift window.
///
/// The caller must atomically persist the returned step only if it is greater
/// than the previously accepted step. This function deliberately does not
/// mutate state or claim that a code has been consumed.
pub fn matching_step(
    secret: &[u8],
    candidate_code: &str,
    unix_seconds: u64,
    last_accepted_step: Option<u64>,
) -> Result<Option<u64>, TotpError> {
    if candidate_code.len() != TOTP_DIGITS
        || !candidate_code.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(TotpError::InvalidCode);
    }

    let current_step = unix_seconds / TOTP_PERIOD_SECONDS;
    let first_step = current_step.saturating_sub(1);
    let last_step = current_step.saturating_add(1);
    let mut matched_step = None;

    for step in first_step..=last_step {
        if last_accepted_step.is_some_and(|last| step <= last) {
            continue;
        }

        let expected = code_for_step(secret, step)?;
        if constant_time::verify_slices_are_equal(
            expected.as_bytes(),
            candidate_code.as_bytes(),
        )
        .is_ok()
        {
            matched_step = Some(matched_step.map_or(step, |matched: u64| matched.max(step)));
        }
    }

    Ok(matched_step)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RFC_TEST_SECRET: &[u8] = b"12345678901234567890";

    #[test]
    fn matches_rfc_6238_sha1_six_digit_vector() {
        assert_eq!(
            code_for_step(RFC_TEST_SECRET, 1).expect("valid TOTP secret should work"),
            "287082"
        );
    }

    #[test]
    fn accepts_current_and_adjacent_time_steps() {
        let previous = code_for_step(RFC_TEST_SECRET, 0).unwrap();
        let current = code_for_step(RFC_TEST_SECRET, 1).unwrap();
        let next = code_for_step(RFC_TEST_SECRET, 2).unwrap();

        assert_eq!(matching_step(RFC_TEST_SECRET, &previous, 59, None).unwrap(), Some(0));
        assert_eq!(matching_step(RFC_TEST_SECRET, &current, 59, None).unwrap(), Some(1));
        assert_eq!(matching_step(RFC_TEST_SECRET, &next, 59, None).unwrap(), Some(2));
    }

    #[test]
    fn rejects_replayed_or_older_time_steps() {
        let code = code_for_step(RFC_TEST_SECRET, 1).unwrap();
        assert_eq!(matching_step(RFC_TEST_SECRET, &code, 59, Some(1)).unwrap(), None);
        assert_eq!(matching_step(RFC_TEST_SECRET, &code, 59, Some(2)).unwrap(), None);
    }

    #[test]
    fn rejects_malformed_codes_and_short_secrets() {
        assert_eq!(
            matching_step(RFC_TEST_SECRET, "12345", 59, None),
            Err(TotpError::InvalidCode)
        );
        assert_eq!(
            matching_step(RFC_TEST_SECRET, "12a456", 59, None),
            Err(TotpError::InvalidCode)
        );
        assert_eq!(
            code_for_step(b"short", 1),
            Err(TotpError::InvalidSecret)
        );
    }

    #[test]
    fn generates_a_fresh_160_bit_secret() {
        let first = generate_secret().expect("system randomness should be available");
        let second = generate_secret().expect("system randomness should be available");
        assert_eq!(first.len(), TOTP_SECRET_BYTES);
        assert_ne!(&first[..], &second[..]);
    }
}
