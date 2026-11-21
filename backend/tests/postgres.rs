use sqlx::PgPool;
use usahapintar_api::database::{connect, migrations::run, DatabaseConfig};

#[tokio::test]
#[ignore = "requires a disposable PostgreSQL database; enabled explicitly in CI"]
async fn postgres_connection_and_migration_runner_work() {
    let config = DatabaseConfig::from_env().expect("CI must provide DATABASE_URL");
    let pool = connect(&config)
        .await
        .expect("PostgreSQL connection should succeed");

    run(&pool)
        .await
        .expect("SQLx migrations should complete successfully");

    let result: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("database should execute a simple query");

    assert_eq!(result, 1);

    pool.close().await;
    assert_pool_closed(&pool);
}

fn assert_pool_closed(pool: &PgPool) {
    assert!(pool.is_closed());
}
