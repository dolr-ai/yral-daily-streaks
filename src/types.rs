use core::str;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub type DeleteStreakRes = ();


#[derive(Serialize, Deserialize, ToSchema)]
pub struct StreakResponse {
    pub current_streak: Option<String>,
    pub last_checkin_date: Option<String>
}
