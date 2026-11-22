//! Password hashing dan verifikasi menggunakan Argon2id.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};

const MEMORY_COST_KIB: u32 = 19 * 1024;
const TIME_COST: u32 = 2;
const PARALLELISM: u32 = 1;
const OUTPUT_LENGTH: usize = 32;
const MAX_ENCODED_HASH_LENGTH: usize = 512;
const MAX_VERIFY_MEMORY_KIB: u32 = 256 * 1024;
const MAX_VERIFY_TIME_COST: u32 = 10;
const MAX_VERIFY_PARALLELISM: u32 = 4;

/// Kebijakan password yang digunakan bersama oleh registrasi dan pemulihan akun.
pub trait PasswordPolicy: Send + Sync {
    fn accepts(&self, password: &str) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordError {
    HashingFailed,
    InvalidEncodedHash,
}

impl std::fmt::Display for PasswordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::HashingFailed => "password hashing failed",
            Self::InvalidEncodedHash => "stored password hash is invalid or unsupported",
        };
        f.write_str(message)
    }
}

impl std::error::Error for PasswordError {}

/// Hash password dengan Argon2id dan salt unik dari CSPRNG sistem.
pub fn hash_password(password: &[u8]) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    let params = Params::new(MEMORY_COST_KIB, TIME_COST, PARALLELISM, Some(OUTPUT_LENGTH))
        .map_err(|_| PasswordError::HashingFailed)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    argon2
        .hash_password(password, &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| PasswordError::HashingFailed)
}

/// Verifikasi encoded PHC hash. Parameter dibatasi agar hash tersimpan yang
/// rusak/tidak tepercaya tidak dapat meminta penggunaan memori atau CPU berlebihan.
pub fn verify_password(password: &[u8], encoded_hash: &str) -> Result<bool, PasswordError> {
    if encoded_hash.len() > MAX_ENCODED_HASH_LENGTH {
        return Err(PasswordError::InvalidEncodedHash);
    }

    let parsed = PasswordHash::new(encoded_hash).map_err(|_| PasswordError::InvalidEncodedHash)?;

    if parsed.algorithm.as_str() != "argon2id" || parsed.version != Some(19) {
        return Err(PasswordError::InvalidEncodedHash);
    }

    let mut memory_cost = None;
    let mut time_cost = None;
    let mut parallelism = None;

    for (name, value) in parsed.params.iter() {
        match name.as_str() {
            "m" => memory_cost = value.decimal().ok(),
            "t" => time_cost = value.decimal().ok(),
            "p" => parallelism = value.decimal().ok(),
            _ => {}
        }
    }

    let memory_cost = memory_cost.ok_or(PasswordError::InvalidEncodedHash)?;
    let time_cost = time_cost.ok_or(PasswordError::InvalidEncodedHash)?;
    let parallelism = parallelism.ok_or(PasswordError::InvalidEncodedHash)?;

    if !(MEMORY_COST_KIB..=MAX_VERIFY_MEMORY_KIB).contains(&memory_cost)
        || !(TIME_COST..=MAX_VERIFY_TIME_COST).contains(&time_cost)
        || !(PARALLELISM..=MAX_VERIFY_PARALLELISM).contains(&parallelism)
    {
        return Err(PasswordError::InvalidEncodedHash);
    }

    Ok(Argon2::default().verify_password(password, &parsed).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hashes_are_salted_and_verifiable() {
        let first =
            hash_password(b"sample-password-with-sufficient-length").expect("password should hash");
        let second =
            hash_password(b"sample-password-with-sufficient-length").expect("password should hash");

        assert_ne!(first, second);
        assert!(first.starts_with("$argon2id$"));
        assert!(verify_password(b"sample-password-with-sufficient-length", &first).unwrap());
        assert!(!verify_password(b"incorrect-password", &first).unwrap());
    }

    #[test]
    fn verification_rejects_non_argon2id_hashes() {
        assert_eq!(
            verify_password(b"password", "$2b$10$invalid").unwrap_err(),
            PasswordError::InvalidEncodedHash
        );
    }

    #[test]
    fn verification_rejects_excessive_hash_parameters() {
        let encoded = "$argon2id$v=19$m=999999999,t=2,p=1$c2FsdHNhbHRzYWx0c2FsdA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        assert_eq!(
            verify_password(b"password", encoded).unwrap_err(),
            PasswordError::InvalidEncodedHash
        );
    }
}
