//! Primitive kriptografi untuk Identity Core.
//!
//! Tidak ada key produksi yang dimuat oleh modul ini. Implementasi produksi
//! harus menyediakan KeyProvider dari secret manager/KMS yang disetujui.

use ring::{
    aead, hmac,
    rand::{SecureRandom, SystemRandom},
};
use zeroize::Zeroizing;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const DIGEST_LEN: usize = 32;

const CONTACT_DOMAIN: &[u8] = b"identity:contact-fingerprint:v1\0";
const OTP_DOMAIN: &[u8] = b"identity:otp-verification:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyPurpose {
    DataEncryption,
    ContactFingerprint,
    OtpVerification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyVersion(pub u32);

/// Key material yang dibersihkan dari memori saat nilai dilepas.
pub struct SecretKey(Zeroizing<[u8; KEY_LEN]>);

impl SecretKey {
    /// Membungkus key yang diperoleh dari sumber tepercaya.
    ///
    /// Jangan gunakan literal atau key yang disimpan di repository untuk runtime.
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    fn expose(&self) -> &[u8; KEY_LEN] {
        &self.0
    }
}

pub struct VersionedKey {
    pub version: KeyVersion,
    key: SecretKey,
}

impl VersionedKey {
    pub fn new(version: KeyVersion, key: SecretKey) -> Self {
        Self { version, key }
    }
}

/// Abstraksi key provider. Implementasi production belum disediakan.
pub trait KeyProvider: Send + Sync {
    /// Mengambil key aktif untuk operasi enkripsi atau pembuatan MAC baru.
    fn active_key(&self, purpose: KeyPurpose) -> Result<VersionedKey, CryptoError>;

    /// Mengambil key versi tertentu untuk dekripsi/verifikasi data lama.
    fn key(&self, purpose: KeyPurpose, version: KeyVersion) -> Result<SecretKey, CryptoError>;
}

#[derive(Clone, PartialEq, Eq)]
pub struct EncryptedPayload {
    pub key_version: KeyVersion,
    pub nonce: [u8; NONCE_LEN],
    pub ciphertext: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct KeyedDigest {
    pub key_version: KeyVersion,
    pub digest: [u8; DIGEST_LEN],
}

/** Context yang mengikat digest OTP pada challenge, sesi, tujuan, kanal, dan kode. */
pub struct OtpVerificationContext<'a> {
    pub challenge_id: &'a [u8],
    pub session_id: &'a [u8],
    pub purpose: &'a str,
    pub channel: &'a str,
    pub code: &'a [u8],
}

impl OtpVerificationContext<'_> {
    fn validate(&self) -> Result<(), CryptoError> {
        if self.challenge_id.is_empty()
            || self.session_id.is_empty()
            || self.purpose.is_empty()
            || self.channel.is_empty()
            || self.code.is_empty()
        {
            return Err(CryptoError::InvalidInput);
        }
        Ok(())
    }

    fn append_to(&self, input: &mut Vec<u8>) -> Result<(), CryptoError> {
        append_field(input, self.challenge_id)?;
        append_field(input, self.session_id)?;
        append_field(input, self.purpose.as_bytes())?;
        append_field(input, self.channel.as_bytes())?;
        append_field(input, self.code)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    KeyUnavailable,
    RandomnessUnavailable,
    InvalidKey,
    InvalidInput,
    EncryptionFailed,
    AuthenticationFailed,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::KeyUnavailable => "required cryptographic key is unavailable",
            Self::RandomnessUnavailable => "secure randomness is unavailable",
            Self::InvalidKey => "cryptographic key is invalid",
            Self::InvalidInput => "cryptographic input is invalid",
            Self::EncryptionFailed => "authenticated encryption failed",
            Self::AuthenticationFailed => "ciphertext authentication failed",
        };
        f.write_str(message)
    }
}

impl std::error::Error for CryptoError {}

/// Menghasilkan byte acak dari CSPRNG sistem operasi.
pub fn random_bytes<const N: usize>() -> Result<[u8; N], CryptoError> {
    let mut bytes = [0_u8; N];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| CryptoError::RandomnessUnavailable)?;
    Ok(bytes)
}

/// Enkripsi AES-256-GCM dengan nonce acak 96-bit dan associated data.
pub fn encrypt(
    provider: &dyn KeyProvider,
    plaintext: &[u8],
    associated_data: &[u8],
) -> Result<EncryptedPayload, CryptoError> {
    let versioned_key = provider.active_key(KeyPurpose::DataEncryption)?;
    let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, versioned_key.key.expose())
        .map_err(|_| CryptoError::InvalidKey)?;
    let key = aead::LessSafeKey::new(unbound_key);
    let nonce = random_bytes::<NONCE_LEN>()?;
    let mut ciphertext = Zeroizing::new(plaintext.to_vec());

    key.seal_in_place_append_tag(
        aead::Nonce::assume_unique_for_key(nonce),
        aead::Aad::from(associated_data),
        &mut *ciphertext,
    )
    .map_err(|_| CryptoError::EncryptionFailed)?;

    Ok(EncryptedPayload {
        key_version: versioned_key.version,
        nonce,
        ciphertext: ciphertext.to_vec(),
    })
}

/// Dekripsi AES-256-GCM. Plaintext dibersihkan ketika hasil dilepas.
pub fn decrypt(
    provider: &dyn KeyProvider,
    payload: &EncryptedPayload,
    associated_data: &[u8],
) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    let key_material = provider.key(KeyPurpose::DataEncryption, payload.key_version)?;
    let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, key_material.expose())
        .map_err(|_| CryptoError::InvalidKey)?;
    let key = aead::LessSafeKey::new(unbound_key);
    let mut buffer = Zeroizing::new(payload.ciphertext.clone());

    let plaintext = key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(payload.nonce),
            aead::Aad::from(associated_data),
            &mut buffer[..],
        )
        .map_err(|_| CryptoError::AuthenticationFailed)?;

    let plaintext_len = plaintext.len();
    buffer.truncate(plaintext_len);
    Ok(buffer)
}

/// Fingerprint kontak untuk pencarian/rate-limit tanpa menyimpan kontak plaintext.
pub fn contact_fingerprint(
    provider: &dyn KeyProvider,
    channel: &str,
    canonical_contact: &str,
) -> Result<KeyedDigest, CryptoError> {
    if channel.is_empty() || canonical_contact.is_empty() {
        return Err(CryptoError::InvalidInput);
    }

    let mut input = Zeroizing::new(CONTACT_DOMAIN.to_vec());
    append_field(&mut input, channel.as_bytes())?;
    append_field(&mut input, canonical_contact.as_bytes())?;

    let versioned_key = provider.active_key(KeyPurpose::ContactFingerprint)?;
    let key = hmac::Key::new(hmac::HMAC_SHA256, versioned_key.key.expose());
    let tag = hmac::sign(&key, &input);
    let mut digest = [0_u8; DIGEST_LEN];
    digest.copy_from_slice(tag.as_ref());

    Ok(KeyedDigest {
        key_version: versioned_key.version,
        digest,
    })
}

/// Menghasilkan keyed digest OTP yang terikat pada challenge dan sesi.
pub fn otp_digest(
    provider: &dyn KeyProvider,
    version: Option<KeyVersion>,
    context: &OtpVerificationContext<'_>,
) -> Result<KeyedDigest, CryptoError> {
    context.validate()?;

    let versioned_key = match version {
        Some(version) => {
            VersionedKey::new(version, provider.key(KeyPurpose::OtpVerification, version)?)
        }
        None => provider.active_key(KeyPurpose::OtpVerification)?,
    };

    let mut input = Zeroizing::new(OTP_DOMAIN.to_vec());
    context.append_to(&mut input)?;

    let key = hmac::Key::new(hmac::HMAC_SHA256, versioned_key.key.expose());
    let tag = hmac::sign(&key, &input);
    let mut digest = [0_u8; DIGEST_LEN];
    digest.copy_from_slice(tag.as_ref());

    Ok(KeyedDigest {
        key_version: versioned_key.version,
        digest,
    })
}

/// Verifikasi keyed digest menggunakan pembandingan constant-time dari ring.
pub fn verify_otp_digest(
    provider: &dyn KeyProvider,
    version: KeyVersion,
    context: &OtpVerificationContext<'_>,
    expected_digest: &[u8; DIGEST_LEN],
) -> Result<bool, CryptoError> {
    context.validate()?;

    let key_material = provider.key(KeyPurpose::OtpVerification, version)?;
    let mut input = Zeroizing::new(OTP_DOMAIN.to_vec());
    context.append_to(&mut input)?;

    let key = hmac::Key::new(hmac::HMAC_SHA256, key_material.expose());
    Ok(hmac::verify(&key, &input, expected_digest).is_ok())
}

fn append_field(output: &mut Vec<u8>, field: &[u8]) -> Result<(), CryptoError> {
    let length = u32::try_from(field.len()).map_err(|_| CryptoError::InvalidInput)?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(field);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestKeyProvider;

    impl KeyProvider for TestKeyProvider {
        fn active_key(&self, purpose: KeyPurpose) -> Result<VersionedKey, CryptoError> {
            self.key(purpose, KeyVersion(7))
                .map(|key| VersionedKey::new(KeyVersion(7), key))
        }

        fn key(&self, purpose: KeyPurpose, version: KeyVersion) -> Result<SecretKey, CryptoError> {
            if version != KeyVersion(7) {
                return Err(CryptoError::KeyUnavailable);
            }
            let marker = match purpose {
                KeyPurpose::DataEncryption => 0x11,
                KeyPurpose::ContactFingerprint => 0x22,
                KeyPurpose::OtpVerification => 0x33,
            };
            Ok(SecretKey::from_bytes([marker; KEY_LEN]))
        }
    }

    #[test]
    fn authenticated_encryption_round_trips() {
        let provider = TestKeyProvider;
        let encrypted = encrypt(&provider, b"private destination", b"session:123")
            .expect("encryption should succeed");
        let decrypted =
            decrypt(&provider, &encrypted, b"session:123").expect("decryption should succeed");

        assert_eq!(&decrypted[..], b"private destination");
        assert_eq!(encrypted.key_version, KeyVersion(7));
    }

    #[test]
    fn authenticated_encryption_rejects_modified_associated_data() {
        let provider = TestKeyProvider;
        let encrypted =
            encrypt(&provider, b"private destination", b"session:123").expect("encryption");

        assert_eq!(
            decrypt(&provider, &encrypted, b"session:456").unwrap_err(),
            CryptoError::AuthenticationFailed
        );
    }

    #[test]
    fn authenticated_encryption_rejects_modified_ciphertext() {
        let provider = TestKeyProvider;
        let mut encrypted =
            encrypt(&provider, b"private destination", b"session:123").expect("encryption");
        encrypted.ciphertext[0] ^= 0x01;

        assert_eq!(
            decrypt(&provider, &encrypted, b"session:123").unwrap_err(),
            CryptoError::AuthenticationFailed
        );
    }

    #[test]
    fn contact_fingerprints_are_domain_separated() {
        let provider = TestKeyProvider;
        let email = contact_fingerprint(&provider, "email", "user@example.test").unwrap();
        let phone = contact_fingerprint(&provider, "phone", "user@example.test").unwrap();

        assert_ne!(email.digest, phone.digest);
        assert_eq!(email.key_version, KeyVersion(7));
    }

    #[test]
    fn otp_digest_is_bound_to_challenge_and_session() {
        let provider = TestKeyProvider;
        let original_context = OtpVerificationContext {
            challenge_id: b"challenge-a",
            session_id: b"session-a",
            purpose: "registration",
            channel: "email",
            code: b"123456",
        };
        let digest = otp_digest(&provider, None, &original_context).unwrap();

        assert!(verify_otp_digest(
            &provider,
            digest.key_version,
            &original_context,
            &digest.digest,
        )
        .unwrap());

        let modified_context = OtpVerificationContext {
            challenge_id: b"challenge-b",
            ..original_context
        };
        assert!(!verify_otp_digest(
            &provider,
            digest.key_version,
            &modified_context,
            &digest.digest,
        )
        .unwrap());
    }

    #[test]
    fn empty_contact_fields_are_rejected() {
        let provider = TestKeyProvider;
        assert!(matches!(
            contact_fingerprint(&provider, "email", ""),
            Err(CryptoError::InvalidInput)
        ));
    }
}
