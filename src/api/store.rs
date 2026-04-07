use sqlx::Row;

use crate::{
    db_pool::DbPool,
    types::StreakDbRow,
    utils::error::{Error, Result},
};

#[async_trait::async_trait]
pub trait StreakStore: Send + Sync {
    async fn get_streak(&self, user_principal: &str) -> Result<Option<StreakDbRow>>;
    async fn set_streak(
        &self,
        user_principal: &str,
        streak: i64,
        last_checkin_epoch_ms: i64,
    ) -> Result<()>;
    async fn delete_streak(&self, user_principal: &str) -> Result<()>;
}

#[async_trait::async_trait]
impl StreakStore for sqlx::PgPool {
    async fn get_streak(&self, user_principal: &str) -> Result<Option<StreakDbRow>> {
        let row = sqlx::query(
            "SELECT current_streak, last_checkin_epoch_ms FROM daily_streaks WHERE user_principal = $1",
        )
        .bind(user_principal)
        .fetch_optional(self as &sqlx::PgPool)
        .await
        .map_err(Error::SqlxError)?;

        Ok(row.map(|r| StreakDbRow {
            current_streak: r.get("current_streak"),
            last_checkin_epoch_ms: r.get("last_checkin_epoch_ms"),
        }))
    }

    async fn set_streak(
        &self,
        user_principal: &str,
        streak: i64,
        last_checkin_epoch_ms: i64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO daily_streaks (user_principal, current_streak, last_checkin_epoch_ms)
             VALUES ($1, $2, $3)
             ON CONFLICT (user_principal) DO UPDATE
               SET current_streak = EXCLUDED.current_streak,
                   last_checkin_epoch_ms = EXCLUDED.last_checkin_epoch_ms",
        )
        .bind(user_principal)
        .bind(streak)
        .bind(last_checkin_epoch_ms)
        .execute(self)
        .await
        .map_err(Error::SqlxError)?;

        Ok(())
    }

    async fn delete_streak(&self, user_principal: &str) -> Result<()> {
        sqlx::query("DELETE FROM daily_streaks WHERE user_principal = $1")
            .bind(user_principal)
            .execute(self)
            .await
            .map_err(Error::SqlxError)?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl StreakStore for DbPool {
    async fn get_streak(&self, user_principal: &str) -> Result<Option<StreakDbRow>> {
        let row = self.execute(|pool| async move {
            sqlx::query(
                "SELECT current_streak, last_checkin_epoch_ms FROM daily_streaks WHERE user_principal = $1",
            )
            .bind(user_principal)
            .fetch_optional(&pool)
            .await
        }).await?;

        Ok(row.map(|r| StreakDbRow {
            current_streak: r.get("current_streak"),
            last_checkin_epoch_ms: r.get("last_checkin_epoch_ms"),
        }))
    }

    async fn set_streak(
        &self,
        user_principal: &str,
        streak: i64,
        last_checkin_epoch_ms: i64,
    ) -> Result<()> {
        self.execute(|pool| async move {
            sqlx::query(
                "INSERT INTO daily_streaks (user_principal, current_streak, last_checkin_epoch_ms)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (user_principal) DO UPDATE
                   SET current_streak = EXCLUDED.current_streak,
                       last_checkin_epoch_ms = EXCLUDED.last_checkin_epoch_ms",
            )
            .bind(user_principal)
            .bind(streak)
            .bind(last_checkin_epoch_ms)
            .execute(&pool)
            .await
        })
        .await?;

        Ok(())
    }

    async fn delete_streak(&self, user_principal: &str) -> Result<()> {
        self.execute(|pool| async move {
            sqlx::query("DELETE FROM daily_streaks WHERE user_principal = $1")
                .bind(user_principal)
                .execute(&pool)
                .await
        })
        .await?;
        Ok(())
    }
}
