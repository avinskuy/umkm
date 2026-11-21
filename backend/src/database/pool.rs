use std::time::Duration;

use sqlx::{postgres::PgPoolOptions, PgPool};

use super::DatabaseConfig;

const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
const IDLE_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const MAX_CONNECTION_LIFETIME: Duration = Duration::from_secs(30 * 60);

/// Create a bounded pool using the credentials supplied by the caller.
pub async fn connect(config: &DatabaseConfig) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(config.max_connections())
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .idle_timeout(Some(IDLE_TIMEOUT))
        .max_lifetime(Some(MAX_CONNECTION_LIFETIME))
        .connect_with(config.connect_options())
        .await
}
