use std::{env, net::SocketAddr};

use usahapintar_api::{
    app_with_authentication,
    config::server_address,
    database::{connect, DatabaseConfig},
    identity::{http::AuthenticationApiState, AuthenticationRateLimitKey},
};

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

    let rate_limit_key = zeroize::Zeroizing::new(env::var("AUTH_RATE_LIMIT_HMAC_KEY").map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("AUTH_RATE_LIMIT_HMAC_KEY is required: {error}"),
        )
    })?);
    let rate_limit_key = AuthenticationRateLimitKey::from_hex(rate_limit_key.as_str())?;
    let authentication = AuthenticationApiState::new(pool.clone(), rate_limit_key);

    let host = env::var("HOST").ok();
    let port = env::var("PORT").ok();
    let address = server_address(host.as_deref(), port.as_deref())?;

    let listener = tokio::net::TcpListener::bind(address).await?;

    tracing::info!(%address, "USahapintar API listening");
    tracing::info!("PostgreSQL application pool initialized");

    axum::serve(
        listener,
        app_with_authentication(pool, authentication)
            .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
