//! Primitive kriptografi untuk Identity Core.
//!
//! Tidak ada key produksi yang dimuat oleh modul ini. Implementasi produksi
//! harus menyediakan KeyProvider dari secret manager/KMS yang disetujui.

use std::collections::HashMap;

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
const ADMIN_TOTP_DOMAIN: &[u8] = b"identity:admin-totp-secret:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyPurpose {
    DataEncryption,
    ContactFingerprint,
    OtpVerification,
    AdminTotpSecret,
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

    /// Versi key yang masih dapat dipakai untuk membaca event dalam jendela rotasi.
    fn key_versions(&self, _purpose: KeyPurpose) -> Result<Vec<KeyVersion>, CryptoError> {
        Ok(Vec::new())
    }
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

/// Context yang mengikat digest OTP pada challenge, sesi, tujuan, kanal, dan kode.
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
    encrypt_for_purpose(
        provider,
        KeyPurpose::DataEncryption,
        plaintext,
        associated_data,
    )
}

/// Enkripsi untuk tujuan key tertentu tanpa mengubah domain key antar-purpose.
fn encrypt_for_purpose(
    provider: &dyn KeyProvider,
    purpose: KeyPurpose,
    plaintext: &[u8],
    associated_data: &[u8],
) -> Result<EncryptedPayload, CryptoError> {
    let versioned_key = provider.active_key(purpose)?;
    encrypt_with_key(versioned_key, plaintext, associated_data)
}

fn encrypt_with_key(
    versioned_key: VersionedKey,
    plaintext: &[u8],
    associated_data: &[u8],
) -> Result<EncryptedPayload, CryptoError> {
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
    decrypt_for_purpose(
        provider,
        KeyPurpose::DataEncryption,
        payload,
        associated_data,
    )
}

fn decrypt_for_purpose(
    provider: &dyn KeyProvider,
    purpose: KeyPurpose,
    payload: &EncryptedPayload,
    associated_data: &[u8],
) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    let key_material = provider.key(purpose, payload.key_version)?;
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

/// Enkripsi secret TOTP Admin dengan key-purpose terpisah dan AAD terikat akun.
pub fn encrypt_admin_totp_secret(
    provider: &dyn KeyProvider,
    admin_user_id: &str,
    secret: &[u8],
) -> Result<EncryptedPayload, CryptoError> {
    if !is_canonical_uuid(admin_user_id) || secret.len() != 20 {
        return Err(CryptoError::InvalidInput);
    }

    let versioned_key = provider.active_key(KeyPurpose::AdminTotpSecret)?;
    let associated_data = admin_totp_associated_data(admin_user_id, versioned_key.version)?;
    encrypt_with_key(versioned_key, secret, &associated_data)
}

/// Dekripsi secret TOTP hanya untuk akun Admin yang sama dan versi key yang tersimpan.
pub fn decrypt_admin_totp_secret(
    provider: &dyn KeyProvider,
    admin_user_id: &str,
    payload: &EncryptedPayload,
) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    if !is_canonical_uuid(admin_user_id) {
        return Err(CryptoError::InvalidInput);
    }

    let associated_data = admin_totp_associated_data(admin_user_id, payload.key_version)?;
    let secret = decrypt_for_purpose(
        provider,
        KeyPurpose::AdminTotpSecret,
        payload,
        &associated_data,
    )?;
    if secret.len() != 20 {
        return Err(CryptoError::AuthenticationFailed);
    }
    Ok(secret)
}

fn admin_totp_associated_data(
    admin_user_id: &str,
    version: KeyVersion,
) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    let version_label = key_version_label(version)?;
    let mut associated_data = Zeroizing::new(ADMIN_TOTP_DOMAIN.to_vec());
    append_field(&mut associated_data, admin_user_id.as_bytes())?;
    append_field(&mut associated_data, b"admin_totp")?;
    append_field(&mut associated_data, version_label.as_bytes())?;
    Ok(associated_data)
}

fn is_canonical_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
}

/// Fingerprint kontak untuk pencarian/rate-limit tanpa menyimpan kontak plaintext.
pub fn contact_fingerprint(
    provider: &dyn KeyProvider,
    channel: &str,
    canonical_contact: &str,
) -> Result<KeyedDigest, CryptoError> {
    validate_contact_fingerprint_input(channel, canonical_contact)?;
    let versioned_key = provider.active_key(KeyPurpose::ContactFingerprint)?;
    contact_fingerprint_with_key(
        versioned_key.version,
        &versioned_key.key,
        channel,
        canonical_contact,
    )
}

/// Fingerprint untuk satu versi key, termasuk versi lama selama jendela rotasi.
pub fn contact_fingerprint_for_version(
    provider: &dyn KeyProvider,
    version: KeyVersion,
    channel: &str,
    canonical_contact: &str,
) -> Result<KeyedDigest, CryptoError> {
    validate_contact_fingerprint_input(channel, canonical_contact)?;
    let key = provider.key(KeyPurpose::ContactFingerprint, version)?;
    contact_fingerprint_with_key(version, &key, channel, canonical_contact)
}

/// Menghasilkan seluruh alias fingerprint yang masih didukung provider.
pub fn contact_fingerprint_aliases(
    provider: &dyn KeyProvider,
    channel: &str,
    canonical_contact: &str,
) -> Result<Vec<KeyedDigest>, CryptoError> {
    validate_contact_fingerprint_input(channel, canonical_contact)?;
    let active = provider.active_key(KeyPurpose::ContactFingerprint)?;
    let mut versions = provider.key_versions(KeyPurpose::ContactFingerprint)?;
    versions.push(active.version);
    versions.sort_by_key(|version| version.0);
    versions.dedup();

    versions
        .into_iter()
        .map(|version| {
            if version == active.version {
                contact_fingerprint_with_key(version, &active.key, channel, canonical_contact)
            } else {
                contact_fingerprint_for_version(provider, version, channel, canonical_contact)
            }
        })
        .collect()
}

fn validate_contact_fingerprint_input(
    channel: &str,
    canonical_contact: &str,
) -> Result<(), CryptoError> {
    if channel.is_empty() || canonical_contact.is_empty() {
        return Err(CryptoError::InvalidInput);
    }
    Ok(())
}

fn contact_fingerprint_with_key(
    version: KeyVersion,
    key_material: &SecretKey,
    channel: &str,
    canonical_contact: &str,
) -> Result<KeyedDigest, CryptoError> {
    let mut input = Zeroizing::new(CONTACT_DOMAIN.to_vec());
    append_field(&mut input, channel.as_bytes())?;
    append_field(&mut input, canonical_contact.as_bytes())?;

    let key = hmac::Key::new(hmac::HMAC_SHA256, key_material.expose());
    let tag = hmac::sign(&key, &input);
    let mut digest = [0_u8; DIGEST_LEN];
    digest.copy_from_slice(tag.as_ref());

    Ok(KeyedDigest {
        key_version: version,
        digest,
    })
}

/// Representasi versi key yang disimpan pada kolom TEXT database.
/// Keyring khusus secret TOTP Admin. Nilai key disediakan oleh secret manager
/// melalui environment injection; tidak ada fallback key atau default development.
pub struct AdminTotpKeyring {
    active_version: KeyVersion,
    keys: HashMap<KeyVersion, SecretKey>,
}

impl AdminTotpKeyring {
    pub fn from_env() -> Result<Self, CryptoError> {
        let active_label = Zeroizing::new(
            std::env::var("ADMIN_TOTP_ENCRYPTION_KEY_ACTIVE_VERSION")
                .map_err(|_| CryptoError::KeyUnavailable)?,
        );
        let active_version = parse_key_version(active_label.as_str())?;
        let encoded_keys = Zeroizing::new(
            std::env::var("ADMIN_TOTP_ENCRYPTION_KEYS").map_err(|_| CryptoError::KeyUnavailable)?,
        );

        let mut keys = HashMap::new();
        for entry in encoded_keys.split(';') {
            let (label, encoded_key) = entry.split_once(':').ok_or(CryptoError::InvalidInput)?;
            let version = parse_key_version(label)?;
            let key_bytes = decode_hex_key(encoded_key)?;
            if keys
                .insert(version, SecretKey::from_bytes(key_bytes))
                .is_some()
            {
                return Err(CryptoError::InvalidInput);
            }
        }

        if keys.is_empty() || !keys.contains_key(&active_version) {
            return Err(CryptoError::KeyUnavailable);
        }

        Ok(Self {
            active_version,
            keys,
        })
    }

    pub fn from_hex_entries(
        active_version: KeyVersion,
        entries: &[(KeyVersion, &str)],
    ) -> Result<Self, CryptoError> {
        if active_version.0 == 0 || entries.is_empty() {
            return Err(CryptoError::InvalidInput);
        }

        let mut keys = HashMap::new();
        for (version, encoded_key) in entries {
            if version.0 == 0 {
                return Err(CryptoError::InvalidInput);
            }
            let key_bytes = decode_hex_key(encoded_key)?;
            if keys
                .insert(*version, SecretKey::from_bytes(key_bytes))
                .is_some()
            {
                return Err(CryptoError::InvalidInput);
            }
        }

        if !keys.contains_key(&active_version) {
            return Err(CryptoError::KeyUnavailable);
        }

        Ok(Self {
            active_version,
            keys,
        })
    }
}

impl KeyProvider for AdminTotpKeyring {
    fn active_key(&self, purpose: KeyPurpose) -> Result<VersionedKey, CryptoError> {
        if purpose != KeyPurpose::AdminTotpSecret {
            return Err(CryptoError::KeyUnavailable);
        }
        let key = self
            .keys
            .get(&self.active_version)
            .ok_or(CryptoError::KeyUnavailable)?;
        Ok(VersionedKey::new(
            self.active_version,
            SecretKey::from_bytes(*key.expose()),
        ))
    }

    fn key(&self, purpose: KeyPurpose, version: KeyVersion) -> Result<SecretKey, CryptoError> {
        if purpose != KeyPurpose::AdminTotpSecret {
            return Err(CryptoError::KeyUnavailable);
        }
        self.keys
            .get(&version)
            .map(|key| SecretKey::from_bytes(*key.expose()))
            .ok_or(CryptoError::KeyUnavailable)
    }

    fn key_versions(&self, purpose: KeyPurpose) -> Result<Vec<KeyVersion>, CryptoError> {
        if purpose != KeyPurpose::AdminTotpSecret {
            return Err(CryptoError::KeyUnavailable);
        }
        let mut versions: Vec<_> = self.keys.keys().copied().collect();
        versions.sort_by_key(|version| version.0);
        Ok(versions)
    }
}

/// Keyring Identity terpisah dari keyring TOTP Admin.
/// Setiap purpose memiliki versi aktif dan kumpulan versi lama sendiri.
struct PurposeKeyring {
    active_version: KeyVersion,
    keys: HashMap<KeyVersion, SecretKey>,
}

impl PurposeKeyring {
    fn from_env(active_name: &str, keys_name: &str) -> Result<Self, CryptoError> {
        let active_label = Zeroizing::new(
            std::env::var(active_name).map_err(|_| CryptoError::KeyUnavailable)?,
        );
        let active_version = parse_key_version(active_label.as_str())?;
        let encoded_keys =
            Zeroizing::new(std::env::var(keys_name).map_err(|_| CryptoError::KeyUnavailable)?);
        let entries = encoded_keys
            .split(';')
            .map(|entry| {
                let (label, encoded_key) =
                    entry.split_once(':').ok_or(CryptoError::InvalidInput)?;
                Ok((parse_key_version(label)?, encoded_key))
            })
            .collect::<Result<Vec<_>, CryptoError>>()?;
        Self::from_hex_entries(active_version, &entries)
    }

    fn from_hex_entries(
        active_version: KeyVersion,
        entries: &[(KeyVersion, &str)],
    ) -> Result<Self, CryptoError> {
        if active_version.0 == 0 || entries.is_empty() {
            return Err(CryptoError::InvalidInput);
        }

        let mut keys = HashMap::new();
        for (version, encoded_key) in entries {
            if version.0 == 0 {
                return Err(CryptoError::InvalidInput);
            }
            let key_bytes = decode_hex_key(encoded_key)?;
            if keys
                .insert(*version, SecretKey::from_bytes(key_bytes))
                .is_some()
            {
                return Err(CryptoError::InvalidInput);
            }
        }

        if !keys.contains_key(&active_version) {
            return Err(CryptoError::KeyUnavailable);
        }

        Ok(Self {
            active_version,
            keys,
        })
    }

    fn active_key(&self) -> Result<VersionedKey, CryptoError> {
        let key = self
            .keys
            .get(&self.active_version)
            .ok_or(CryptoError::KeyUnavailable)?;
        Ok(VersionedKey::new(
            self.active_version,
            SecretKey::from_bytes(*key.expose()),
        ))
    }

    fn key(&self, version: KeyVersion) -> Result<SecretKey, CryptoError> {
        self.keys
            .get(&version)
            .map(|key| SecretKey::from_bytes(*key.expose()))
            .ok_or(CryptoError::KeyUnavailable)
    }

    fn versions(&self) -> Vec<KeyVersion> {
        let mut versions: Vec<_> = self.keys.keys().copied().collect();
        versions.sort_by_key(|version| version.0);
        versions
    }
}

/// Provider produksi untuk tiga purpose Identity. TOTP Admin tetap memakai
/// AdminTotpKeyring dan tidak dapat mengambil key dari provider ini.
pub struct IdentityKeyring {
    purposes: HashMap<KeyPurpose, PurposeKeyring>,
}

impl IdentityKeyring {
    pub fn from_env() -> Result<Self, CryptoError> {
        let mut purposes = HashMap::new();
        purposes.insert(
            KeyPurpose::DataEncryption,
            PurposeKeyring::from_env(
                "IDENTITY_DATA_ENCRYPTION_KEY_ACTIVE_VERSION",
                "IDENTITY_DATA_ENCRYPTION_KEYS",
            )?,
        );
        purposes.insert(
            KeyPurpose::ContactFingerprint,
            PurposeKeyring::from_env(
                "IDENTITY_CONTACT_FINGERPRINT_KEY_ACTIVE_VERSION",
                "IDENTITY_CONTACT_FINGERPRINT_KEYS",
            )?,
        );
        purposes.insert(
            KeyPurpose::OtpVerification,
            PurposeKeyring::from_env(
                "IDENTITY_OTP_VERIFICATION_KEY_ACTIVE_VERSION",
                "IDENTITY_OTP_VERIFICATION_KEYS",
            )?,
        );
        validate_key_material_separation(&purposes)?;
        Ok(Self { purposes })
    }

    fn from_purpose_entries(
        entries: [
            (KeyPurpose, KeyVersion, &[(KeyVersion, &str)]);
            3
        ],
    ) -> Result<Self, CryptoError> {
        let mut purposes = HashMap::new();
        for (purpose, active_version, keys) in entries {
            if purpose == KeyPurpose::AdminTotpSecret
                || purposes
                    .insert(
                        purpose,
                        PurposeKeyring::from_hex_entries(active_version, keys)?,
                    )
                    .is_some()
            {
                return Err(CryptoError::InvalidInput);
            }
        }
        validate_key_material_separation(&purposes)?;
        Ok(Self { purposes })
    }
}

fn validate_key_material_separation(
    purposes: &HashMap<KeyPurpose, PurposeKeyring>,
) -> Result<(), CryptoError> {
    let keys: Vec<&SecretKey> = purposes
        .values()
        .flat_map(|purpose| purpose.keys.values())
        .collect();
    for (index, key) in keys.iter().enumerate() {
        if keys[index + 1..]
            .iter()
            .any(|other| key.expose() == other.expose())
        {
            return Err(CryptoError::InvalidKey);
        }
    }
    Ok(())
}

impl KeyProvider for IdentityKeyring {
    fn active_key(&self, purpose: KeyPurpose) -> Result<VersionedKey, CryptoError> {
        self.purposes
            .get(&purpose)
            .ok_or(CryptoError::KeyUnavailable)?
            .active_key()
    }

    fn key(&self, purpose: KeyPurpose, version: KeyVersion) -> Result<SecretKey, CryptoError> {
        self.purposes
            .get(&purpose)
            .ok_or(CryptoError::KeyUnavailable)?
            .key(version)
    }

    fn key_versions(&self, purpose: KeyPurpose) -> Result<Vec<KeyVersion>, CryptoError> {
        Ok(self
            .purposes
            .get(&purpose)
            .ok_or(CryptoError::KeyUnavailable)?
            .versions())
    }
}

fn decode_hex_key(value: &str) -> Result<[u8; KEY_LEN], CryptoError> {
    if value.len() != KEY_LEN * 2 {
        return Err(CryptoError::InvalidKey);
    }

    let mut decoded = Zeroizing::new([0_u8; KEY_LEN]);
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(pair[0]).ok_or(CryptoError::InvalidKey)?;
        let low = hex_nibble(pair[1]).ok_or(CryptoError::InvalidKey)?;
        decoded[index] = (high << 4) | low;
    }

    if decoded.iter().all(|byte| *byte == 0) {
        return Err(CryptoError::InvalidKey);
    }

    Ok(*decoded)
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

pub fn key_version_label(version: KeyVersion) -> Result<String, CryptoError> {
    if version.0 == 0 {
        return Err(CryptoError::InvalidInput);
    }
    Ok(format!("v{}", version.0))
}

/// Membaca versi key secara ketat; tidak ada fallback untuk label yang tidak dikenal.
pub fn parse_key_version(label: &str) -> Result<KeyVersion, CryptoError> {
    let value = label.strip_prefix('v').ok_or(CryptoError::InvalidInput)?;
    let version = value
        .parse::<u32>()
        .map_err(|_| CryptoError::InvalidInput)?;
    if version == 0 {
        return Err(CryptoError::InvalidInput);
    }
    Ok(KeyVersion(version))
}

/// Menghasilkan keyed digest OTP yang terikat pada challenge dan sesi.
pub fn otp_digest(
    provider: &dyn KeyProvider,
    context: &OtpVerificationContext<'_>,
) -> Result<KeyedDigest, CryptoError> {
    context.validate()?;

    let versioned_key = provider.active_key(KeyPurpose::OtpVerification)?;

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
                KeyPurpose::AdminTotpSecret => 0x44,
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

    struct RotatingTestKeyProvider;

    impl KeyProvider for RotatingTestKeyProvider {
        fn active_key(&self, purpose: KeyPurpose) -> Result<VersionedKey, CryptoError> {
            self.key(purpose, KeyVersion(7))
                .map(|key| VersionedKey::new(KeyVersion(7), key))
        }

        fn key(&self, purpose: KeyPurpose, version: KeyVersion) -> Result<SecretKey, CryptoError> {
            if !matches!(version, KeyVersion(6 | 7)) {
                return Err(CryptoError::KeyUnavailable);
            }
            let marker = match (purpose, version) {
                (KeyPurpose::ContactFingerprint, KeyVersion(6)) => 0x66,
                (KeyPurpose::ContactFingerprint, KeyVersion(7)) => 0x77,
                (KeyPurpose::DataEncryption, _) => 0x11,
                (KeyPurpose::OtpVerification, _) => 0x33,
                (KeyPurpose::AdminTotpSecret, _) => 0x44,
                _ => return Err(CryptoError::KeyUnavailable),
            };
            Ok(SecretKey::from_bytes([marker; KEY_LEN]))
        }

        fn key_versions(&self, purpose: KeyPurpose) -> Result<Vec<KeyVersion>, CryptoError> {
            if purpose == KeyPurpose::ContactFingerprint {
                Ok(vec![KeyVersion(6), KeyVersion(7)])
            } else {
                Ok(vec![KeyVersion(7)])
            }
        }
    }

    #[test]
    fn contact_fingerprint_aliases_cover_active_and_previous_versions() {
        let provider = RotatingTestKeyProvider;
        let aliases = contact_fingerprint_aliases(&provider, "email", "user@example.test").unwrap();

        assert_eq!(
            aliases
                .iter()
                .map(|alias| alias.key_version)
                .collect::<Vec<_>>(),
            vec![KeyVersion(6), KeyVersion(7)]
        );
        assert_ne!(aliases[0].digest, aliases[1].digest);
    }

    #[test]
    fn key_version_labels_are_strict_and_round_trip() {
        assert_eq!(key_version_label(KeyVersion(7)), Ok("v7".to_owned()));
        assert_eq!(
            key_version_label(KeyVersion(0)),
            Err(CryptoError::InvalidInput)
        );
        assert_eq!(parse_key_version("v7"), Ok(KeyVersion(7)));
        assert_eq!(parse_key_version("7"), Err(CryptoError::InvalidInput));
        assert_eq!(parse_key_version("v0"), Err(CryptoError::InvalidInput));
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
        let digest = otp_digest(&provider, &original_context).unwrap();

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
    fn admin_totp_secret_encryption_is_bound_to_admin_and_key_version() {
        let provider = TestKeyProvider;
        let admin_id = "00000000-0000-4000-8000-000000000001";
        let other_admin_id = "00000000-0000-4000-8000-000000000002";
        let secret = [0x5a_u8; 20];
        let encrypted = encrypt_admin_totp_secret(&provider, admin_id, &secret)
            .expect("Admin TOTP secret should be encrypted");

        assert_eq!(encrypted.key_version, KeyVersion(7));
        assert_eq!(
            &decrypt_admin_totp_secret(&provider, admin_id, &encrypted).unwrap()[..],
            &secret
        );
        assert_eq!(
            decrypt_admin_totp_secret(&provider, other_admin_id, &encrypted).unwrap_err(),
            CryptoError::AuthenticationFailed
        );
    }

    #[test]
    fn identity_keyring_keeps_purposes_separate_and_supports_rotation() {
        const DATA_V1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
        const DATA_V2: &str = "1212121212121212121212121212121212121212121212121212121212121212";
        const CONTACT_V1: &str = "2222222222222222222222222222222222222222222222222222222222222222";
        const CONTACT_V2: &str = "2323232323232323232323232323232323232323232323232323232323232323";
        const OTP_V1: &str = "3333333333333333333333333333333333333333333333333333333333333333";

        let provider = IdentityKeyring::from_purpose_entries([
            (
                KeyPurpose::DataEncryption,
                KeyVersion(2),
                &[(KeyVersion(1), DATA_V1), (KeyVersion(2), DATA_V2)],
            ),
            (
                KeyPurpose::ContactFingerprint,
                KeyVersion(2),
                &[(KeyVersion(1), CONTACT_V1), (KeyVersion(2), CONTACT_V2)],
            ),
            (
                KeyPurpose::OtpVerification,
                KeyVersion(1),
                &[(KeyVersion(1), OTP_V1)],
            ),
        ])
        .expect("valid Identity keyring should load");

        assert_eq!(
            provider.active_key(KeyPurpose::DataEncryption).unwrap().version,
            KeyVersion(2)
        );
        assert_eq!(
            provider.key_versions(KeyPurpose::ContactFingerprint).unwrap(),
            vec![KeyVersion(1), KeyVersion(2)]
        );
        assert!(provider.active_key(KeyPurpose::AdminTotpSecret).is_err());
        assert!(provider.key(KeyPurpose::OtpVerification, KeyVersion(2)).is_err());
    }

    #[test]
    fn admin_totp_keyring_supports_versioned_decryption_and_rotation() {
        let provider = AdminTotpKeyring::from_hex_entries(
            KeyVersion(2),
            &[
                (
                    KeyVersion(1),
                    "1111111111111111111111111111111111111111111111111111111111111111",
                ),
                (
                    KeyVersion(2),
                    "2222222222222222222222222222222222222222222222222222222222222222",
                ),
            ],
        )
        .expect("valid Admin keyring should load");
        let admin_id = "00000000-0000-4000-8000-000000000001";
        let secret = [0x6b_u8; 20];
        let encrypted = encrypt_admin_totp_secret(&provider, admin_id, &secret).unwrap();

        assert_eq!(encrypted.key_version, KeyVersion(2));
        assert_eq!(
            provider.key_versions(KeyPurpose::AdminTotpSecret).unwrap(),
            vec![KeyVersion(1), KeyVersion(2)]
        );
        assert!(provider.active_key(KeyPurpose::DataEncryption).is_err());
        assert_eq!(
            &decrypt_admin_totp_secret(&provider, admin_id, &encrypted).unwrap()[..],
            &secret
        );

        let old_provider = AdminTotpKeyring::from_hex_entries(
            KeyVersion(1),
            &[
                (
                    KeyVersion(1),
                    "1111111111111111111111111111111111111111111111111111111111111111",
                ),
                (
                    KeyVersion(2),
                    "2222222222222222222222222222222222222222222222222222222222222222",
                ),
            ],
        )
        .expect("previous Admin key should remain available during rotation");
        let old_encrypted = encrypt_admin_totp_secret(&old_provider, admin_id, &secret).unwrap();
        assert_eq!(old_encrypted.key_version, KeyVersion(1));
        assert_eq!(
            &decrypt_admin_totp_secret(&provider, admin_id, &old_encrypted).unwrap()[..],
            &secret
        );

        let mut tampered_version = old_encrypted;
        tampered_version.key_version = KeyVersion(2);
        assert_eq!(
            decrypt_admin_totp_secret(&provider, admin_id, &tampered_version).unwrap_err(),
            CryptoError::AuthenticationFailed
        );
    }

    #[test]
    fn admin_totp_encryption_rejects_invalid_identity_and_secret_size() {
        let provider = TestKeyProvider;
        assert_eq!(
            encrypt_admin_totp_secret(&provider, "not-a-uuid", &[0x01; 20])
                .err()
                .expect("invalid Admin identity should be rejected"),
            CryptoError::InvalidInput
        );
        assert_eq!(
            encrypt_admin_totp_secret(
                &provider,
                "00000000-0000-4000-8000-000000000001",
                &[0x01; 19]
            )
            .err()
            .expect("invalid Admin TOTP secret size should be rejected"),
            CryptoError::InvalidInput
        );
    }

    #[test]
    fn admin_totp_keyring_rejects_missing_active_and_duplicate_versions() {
        assert_eq!(
            AdminTotpKeyring::from_hex_entries(
                KeyVersion(3),
                &[(
                    KeyVersion(2),
                    "2222222222222222222222222222222222222222222222222222222222222222",
                )]
            )
            .err()
            .expect("missing active key must be rejected"),
            CryptoError::KeyUnavailable
        );
        assert_eq!(
            AdminTotpKeyring::from_hex_entries(
                KeyVersion(2),
                &[
                    (
                        KeyVersion(2),
                        "2222222222222222222222222222222222222222222222222222222222222222",
                    ),
                    (
                        KeyVersion(2),
                        "3333333333333333333333333333333333333333333333333333333333333333",
                    )
                ]
            )
            .err()
            .expect("duplicate key versions must be rejected"),
            CryptoError::InvalidInput
        );
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
