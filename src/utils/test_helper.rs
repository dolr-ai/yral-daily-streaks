#[cfg(test)]
pub mod db_test_helpers {
    use sqlx::{postgres::PgPoolOptions, PgPool};
    use testcontainers::{runners::AsyncRunner, ContainerAsync};
    use testcontainers_modules::postgres::Postgres;

    /// Holds the container alive for the duration of the test.
    /// Drop this to tear down Postgres automatically.
    pub struct TestDb {
        pub pool: PgPool,
        _container: ContainerAsync<Postgres>,
    }

    impl TestDb {
        pub async fn new() -> Self {
            // Starts a fresh Postgres container
            let container = Postgres::default()
                .start()
                .await
                .expect("Failed to start Postgres container");

            let host = container.get_host().await.unwrap();
            let port = container.get_host_port_ipv4(5432).await.unwrap();

            let url = format!("postgres://postgres:postgres@{}:{}/postgres", host, port);

            let pool = PgPoolOptions::new()
                .max_connections(5)
                .connect(&url)
                .await
                .expect("Failed to connect to test DB");

            // Run your schema migration inline
            sqlx::query(
                "CREATE TABLE IF NOT EXISTS daily_streaks (
                    user_principal TEXT PRIMARY KEY,
                    current_streak  BIGINT NOT NULL DEFAULT 1,
                    last_checkin_epoch_ms BIGINT NOT NULL
                )",
            )
            .execute(&pool)
            .await
            .expect("Failed to create table");

            TestDb {
                pool,
                _container: container,
            }
        }
    }
}
