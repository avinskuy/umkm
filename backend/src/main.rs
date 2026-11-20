use std::net::SocketAddr;
use usahapintar_api::app;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let address = SocketAddr::from(([127, 0, 0, 1], 5000));
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("failed to bind API listener");

    tracing::info!(%address, "USahapintar API listening");

    axum::serve(listener, app())
        .await
        .expect("API server failed");
}
