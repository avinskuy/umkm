use std::env;
use usahapintar_api::{app, config::server_address};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let host = env::var("HOST").ok();
    let port = env::var("PORT").ok();
    let address = server_address(host.as_deref(), port.as_deref())?;

    let listener = tokio::net::TcpListener::bind(address).await?;

    tracing::info!(%address, "USahapintar API listening");

    axum::serve(listener, app()).await?;

    Ok(())
}
