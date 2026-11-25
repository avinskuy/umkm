use std::error::Error;

use usahapintar_api::database::{connect, migrations, DatabaseConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = DatabaseConfig::from_env()?;
    let pool = connect(&config).await?;

    println!("PostgreSQL connection established; applying pending migrations.");
    migrations::run(&pool).await?;
    println!("Database migrations completed successfully.");

    pool.close().await;
    Ok(())
}
