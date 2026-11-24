use std::{env, io, net::SocketAddr, sync::Arc};

use usahapintar_api::{
    app_with_authentication_admin_and_registration,
    config::server_address,
    database::{connect, DatabaseConfig},
    identity::{
        crypto::{AdminTotpKeyring, IdentityKeyring},
        flowkirim_http::FlowKirimHttpProvider,
        http::{
            AdminAuthenticationApiState, AuthenticationApiState, PostgresRegistrationAbuseGuard,
            RegistrationApiState, RegistrationPolicyVersions,
        },
        password::SimplePasswordPolicy,
        resend_http::ResendHttpProvider,
        AuthenticationRateLimitKey,
    },
};
use zeroize::Zeroizing;

fn required_env(name: &'static str) -> Result<String, io::Error> {
    env::var(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, format!("{name} is required")))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let database_config = DatabaseConfig::from_app_env()?;
    let pool = connect(&database_config).await.map_err(|_| {
        std::io::Error::other("failed to connect to PostgreSQL using APP_DATABASE_URL")
    })?;

    let rate_limit_key =
        zeroize::Zeroizing::new(env::var("AUTH_RATE_LIMIT_HMAC_KEY").map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("AUTH_RATE_LIMIT_HMAC_KEY is required: {error}"),
            )
        })?);
    let rate_limit_key = AuthenticationRateLimitKey::from_hex(rate_limit_key.as_str())?;
    let authentication = AuthenticationApiState::new(pool.clone(), rate_limit_key.clone());

    let admin_totp_keyring = Arc::new(AdminTotpKeyring::from_env().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "ADMIN_TOTP_ENCRYPTION_KEY_ACTIVE_VERSION and ADMIN_TOTP_ENCRYPTION_KEYS are required and must be valid",
        )
    })?);
    let admin_authentication =
        AdminAuthenticationApiState::new(pool.clone(), rate_limit_key.clone(), admin_totp_keyring);

    let identity_keyring = Arc::new(IdentityKeyring::from_env().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Identity encryption, contact fingerprint, and OTP verification keys are required and must be valid",
        )
    })?);
    let resend = Arc::new(
        ResendHttpProvider::new(
            Zeroizing::new(required_env("RESEND_API_KEY")?),
            required_env("RESEND_FROM_EMAIL")?,
        )
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Resend API configuration is invalid",
            )
        })?,
    );
    let flowkirim = Arc::new(
        FlowKirimHttpProvider::new(Zeroizing::new(required_env("FLOWKIRIM_API_KEY")?)).map_err(
            |_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "FlowKirim API configuration is invalid",
                )
            },
        )?,
    );
    let abuse_guard = Arc::new(PostgresRegistrationAbuseGuard::new(
        pool.clone(),
        rate_limit_key,
    ));
    let policy_versions = RegistrationPolicyVersions::new(
        required_env("REGISTRATION_TERMS_VERSION")?,
        required_env("REGISTRATION_PRIVACY_VERSION")?,
        env::var("REGISTRATION_MARKETING_NOTICE_VERSION").ok(),
    )
    .map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Registration policy version configuration is invalid",
        )
    })?;
    let registration = RegistrationApiState::new(
        pool.clone(),
        identity_keyring,
        resend,
        abuse_guard,
        Arc::new(SimplePasswordPolicy),
        policy_versions,
    )
    .with_flowkirim_provider(flowkirim);

    let host = env::var("HOST").ok();
    let port = env::var("PORT").ok();
    let address = server_address(host.as_deref(), port.as_deref())?;

    let listener = tokio::net::TcpListener::bind(address).await?;

    tracing::info!(%address, "USahapintar API listening");
    tracing::info!("PostgreSQL application pool initialized");

    axum::serve(
        listener,
        app_with_authentication_admin_and_registration(
            pool,
            authentication,
            admin_authentication,
            registration,
        )
        .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
