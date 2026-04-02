use sqlx::Row;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

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
        .map_err(|e| Error::SqlxError(e))?;

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
        .map_err(|e| Error::SqlxError(e))?;

        Ok(())
    }

    async fn delete_streak(&self, user_principal: &str) -> Result<()> {
        let _response = sqlx::query("DELETE FROM daily_streaks WHERE user_principal = $1")
            .bind(user_principal)
            .execute(self)
            .await
            .map_err(|e| Error::SqlxError(e))?;
        Ok(())
    }
}

// pub struct MockStreakStore {
//     pub test_db: TestDb,
// }

// impl MockStreakStore {
//     pub async fn new() -> Self {
//         MockStreakStore {
//             test_db: TestDb::new().await,
//         }
//     }

//     pub async fn key_exists(&self, user_principal: &str) -> bool {
//         let data = self.data.read().await;
//         data.contains_key(user_principal)
//     }
// }

// #[async_trait::async_trait]
// impl StreakStore for MockStreakStore {

//     async fn get_streak(&self, user_principal: &str) -> Result<Option<StreakResponse>> {
//         let row = sqlx::query(
//             "SELECT current_streak, last_checkin_date FROM user_streaks WHERE user_principal = $1",
//         )
//         .bind(user_principal)
//         .fetch_optional(&self.test_db.pool as &sqlx::PgPool)
//         .await
//         .map_err(|e| Error::SqlxError(e))?;

//         Ok(row.map(|r| {
//             let streak: i64 = r.get("current_streak");
//             let date: String = r.get("last_checkin_date");

//             StreakResponse {
//                 current_streak: Some(streak.to_string()),
//                 last_checkin_date: Some(date.to_string()),
//             }
//         }))

//         // Ok(row.map(|r| StreakResponse {
//         //     current_streak: Some(r.current_streak.to_string()),
//         //     last_checkin_date: Some(r.last_checkin_date.to_string()),
//         // }))
//     }

//     async fn set_streak(&self, user_principal: &str, streak: &str, date: &str) -> Result<()> {
//         self.data.write().await.insert(
//             user_principal.to_string(),
//             (streak.to_string(), date.to_string()),
//         );
//         Ok(())
//     }

//     async fn delete_streak(&self, user_principal: &str) -> Result<()> {
//         self.data.write().await.remove(user_principal);
//         Ok(())
//     }
// }
