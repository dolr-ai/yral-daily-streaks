use sqlx::Row;

use crate::{
    types::StreakResponse,
    utils::error::{Error, Result},
};

#[async_trait::async_trait]
pub trait StreakStore: Send + Sync {
    async fn get_streak(&self, user_principal: &str) -> Result<Option<StreakResponse>>;
    async fn set_streak(&self, user_principal: &str, streak: &str, date: &str) -> Result<()>;
    async fn delete_streak(&self, user_principal: &str) -> Result<()>;
}

#[async_trait::async_trait]
impl StreakStore for sqlx::PgPool {
    async fn get_streak(&self, user_principal: &str) -> Result<Option<StreakResponse>> {
        let row = sqlx::query(
            "SELECT current_streak, last_checkin_date FROM daily_streaks WHERE user_principal = $1",
        )
        .bind(user_principal)
        .fetch_optional(self as &sqlx::PgPool)
        .await
        .map_err(Error::SqlxError)?;

        Ok(row.map(|r| {
            let streak: i64 = r.get("current_streak");
            let date: chrono::NaiveDate = r.get("last_checkin_date");

            StreakResponse {
                current_streak: Some(streak.to_string()),
                last_checkin_date: Some(date.to_string()),
            }
        }))

        // Ok(row.map(|r| StreakResponse {
        //     current_streak: Some(r.current_streak.to_string()),
        //     last_checkin_date: Some(r.last_checkin_date.to_string()),
        // }))
    }

    async fn set_streak(&self, user_principal: &str, streak: &str, date: &str) -> Result<()> {
        let current_streak: i64 = streak.parse().unwrap_or(1);
        let last_checkin = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| Error::Unknown("Invalid date".to_string()))?;

        let _respose = sqlx::query(
            "INSERT INTO daily_streaks (user_principal, current_streak, last_checkin_date) VALUES ($1, $2, $3)
             ON CONFLICT (user_principal) DO UPDATE SET current_streak = EXCLUDED.current_streak, last_checkin_date = EXCLUDED.last_checkin_date",
        )
        .bind(user_principal)
        .bind(current_streak)
        .bind(last_checkin)
        .execute(self)
        .await
        .map_err(Error::SqlxError)?;

        Ok(())
    }

    async fn delete_streak(&self, user_principal: &str) -> Result<()> {
        let _response = sqlx::query("DELETE FROM daily_streaks WHERE user_principal = $1")
            .bind(user_principal)
            .execute(self)
            .await
            .map_err(Error::SqlxError)?;
        Ok(())
    }
}
