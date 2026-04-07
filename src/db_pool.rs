use log::warn;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct DbPool {
    master: PgPool,
    replica: PgPool,
    use_replica: Arc<AtomicBool>, // false = use master, true = use replica
}

impl DbPool {
    pub async fn new(
        pg_hosts: &str,
        pg_database_password: &str,
        pg_port: u16,
    ) -> Result<Self, sqlx::Error> {
        let hosts: Vec<&str> = pg_hosts.split(',').collect();
        let master_url = format!(
            "postgres://postgres:{}@{}:{}/daily_streaks_db?sslmode=require",
            pg_database_password, hosts[0], pg_port
        );
        let replica_url = format!(
            "postgres://postgres:{}@{}:{}/daily_streaks_db?sslmode=require",
            pg_database_password, hosts[1], pg_port
        );

        Ok(Self {
            master: create_pool(&master_url).await?,
            replica: create_pool(&replica_url).await?,
            use_replica: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Run a query with automatic failover — switches to replica if master fails
    pub async fn execute<F, Fut, T>(&self, f: F) -> Result<T, sqlx::Error>
    where
        F: Fn(PgPool) -> Fut, // takes owned PgPool clone
        Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
    {
        match f(self.master.clone()).await {
            Ok(r) => Ok(r),
            Err(e) if is_connection_error(&e) => {
                warn!("Master failed, trying replica");
                f(self.replica.clone()).await // PgPool clone is cheap (Arc inside)
            }
            Err(e) => Err(e),
        }
    }

    pub fn active_pool(&self) -> &PgPool {
        if self.use_replica.load(Ordering::Relaxed) {
            &self.replica
        } else {
            &self.master
        }
    }

    pub fn toggle_pool(&self) {
        let current = self.use_replica.load(Ordering::Relaxed);
        self.use_replica.store(!current, Ordering::Relaxed);
        warn!(
            "Connection error — switched to {}",
            if !current { "replica" } else { "master" }
        );
    }
}

fn is_connection_error(e: &sqlx::Error) -> bool {
    matches!(
        e,
        sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed | sqlx::Error::Io(_)
    )
}

async fn create_pool(url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .test_before_acquire(true)
        .connect(url)
        .await
}
