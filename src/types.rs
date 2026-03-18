use core::str;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct GetStreakRes {}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateStreakRes {}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UpdateStreakRes {}

pub type DeleteStreakRes = ();
