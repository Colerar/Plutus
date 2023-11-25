use chrono::Utc;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Queryable, Selectable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::logs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Log {
  pub id: i64,
  pub room_id: i64,
  pub command: String,
  pub raw_json: Value,
  pub time: chrono::DateTime<Utc>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub related_uid: Option<i64>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = crate::schema::logs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewLog {
  pub room_id: i64,
  pub command: String,
  pub raw_json: Value,
  pub time: chrono::DateTime<Utc>,
  pub related_uid: Option<i64>,
}
