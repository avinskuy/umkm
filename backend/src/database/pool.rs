use std::time::Duration;

use sqlx::{postgres::PgPoolOptions, PgPool};

use super::DatabaseConfig;

pub async fn connect(config: &DatabaseConfig) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(config.max_connections())
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(config.connect_options())
        .await
}
