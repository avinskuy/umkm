use std::env;

use usahapintar_api::{
    app,
    config::server_address,
    database::{connect, DatabaseConfig},
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

    let host = env::var("HOST").ok();
    let port = env::var("PORT").ok();
    let address = server_address(host.as_deref(), port.as_deref())?;

    let listener = tokio::net::TcpListener::bind(address).await?;

    tracing::info!(%address, "USahapintar API listening");
    tracing::info!("PostgreSQL application pool initialized");

    axum::serve(listener, app(pool.clone()))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    pool.close().await;

    Ok(())
}

async fn shutdown_signal() {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::error!(%error, "failed to listen for shutdown signal");
    }

    tracing::info!("shutdown signal received");
}
