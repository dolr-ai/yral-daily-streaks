use core::str;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub type DeleteStreakRes = ();
pub const MS_24H: i64 = 24 * 60 * 60 * 1000;
pub const MS_48H: i64 = 48 * 60 * 60 * 1000;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StreakResponse {
    pub principal_id: String,
    pub streak_count: i64,
    pub just_incremented: bool,
    pub streak_action: String, // "incremented" | "unchanged" | "reset"
    pub server_now_epoch_ms: i64,
    pub last_credited_at_epoch_ms: i64,
    pub next_increment_eligible_at_epoch_ms: i64,
    pub streak_expires_at_epoch_ms: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StreakDbRow {
    pub current_streak: i64,
    pub last_checkin_epoch_ms: i64,
}

pub fn build_response(
    principal_id: String,
    streak_count: i64,
    just_incremented: bool,
    streak_action: String,
    now_ms: i64,
    last_credited_at_epoch_ms: i64,
) -> StreakResponse {
    StreakResponse {
        principal_id,
        streak_count,
        just_incremented,
        streak_action,
        server_now_epoch_ms: now_ms,
        last_credited_at_epoch_ms,
        next_increment_eligible_at_epoch_ms: last_credited_at_epoch_ms + MS_24H,
        streak_expires_at_epoch_ms: last_credited_at_epoch_ms + MS_48H,
    }
}

pub fn now_epoch_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as i64
}

pub fn compute_streak(
    last_checkin_epoch_ms: Option<i64>,
    current: i64,
    now_ms: i64,
) -> (i64, bool, String, i64) {
    match last_checkin_epoch_ms {
        None => (1, true, "incremented".to_string(), now_ms),
        Some(last) => {
            let elapsed = now_ms - last;
            if elapsed < MS_24H {
                (current, false, "unchanged".to_string(), last)
            } else if elapsed < MS_48H {
                (current + 1, true, "incremented".to_string(), now_ms)
            } else {
                (1, true, "reset".to_string(), now_ms)
            }
        }
    }
}
