use sqlx::PgConnection;

/// Role klien yang dapat dipilih pada registrasi publik.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientRole {
    BusinessOwner,
    Investor,
}

impl ClientRole {
    fn as_db_value(self) -> &'static str {
        match self {
            Self::BusinessOwner => "business_owner",
            Self::Investor => "investor",
        }
    }
}

/// Kanal kontak awal yang diverifikasi saat aktivasi akun.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactChannel {
    Email,
    Phone,
}

impl ContactChannel {
    fn as_db_value(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Phone => "phone",
        }
    }
}

/// Data minimum untuk membuat akun pending melalui fungsi database terkontrol.
///
/// `password_hash` harus merupakan hasil password hashing yang sudah dilakukan
/// di lapisan aplikasi. Jangan pernah mengirim password plaintext ke fungsi ini.
pub struct ClientRegistration<'a> {
    pub role: ClientRole,
    pub initial_contact_channel: ContactChannel,
    pub email_normalized: Option<&'a str>,
    pub phone_e164: Option<&'a str>,
    pub password_hash: &'a str,
}

/// Membuat akun klien pending tanpa memberi aplikasi hak INSERT langsung ke tabel.
pub async fn create_client_user(
    connection: &mut PgConnection,
    registration: &ClientRegistration<'_>,
) -> Result<String, sqlx::Error> {
    sqlx::query_scalar::<_, String>("SELECT public.create_client_user($1, $2, $3, $4, $5)::text")
        .bind(registration.role.as_db_value())
        .bind(registration.initial_contact_channel.as_db_value())
        .bind(registration.email_normalized)
        .bind(registration.phone_e164)
        .bind(registration.password_hash)
        .fetch_one(connection)
        .await
}
