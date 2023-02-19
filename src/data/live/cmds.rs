use std::fmt;

use serde::Deserialize;
use serde_repr::Deserialize_repr;
use serde_with::{serde_as, NoneAsEmptyString};

#[derive(Deserialize)]
#[serde(untagged)]
pub enum MaybeCommand {
  Command(Command),
  Unknown(serde_json::Value),
}

impl fmt::Debug for MaybeCommand {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if f.alternate() {
      match self {
        MaybeCommand::Command(cmd) => write!(f, "{:#?}", cmd),
        MaybeCommand::Unknown(obj) => write!(
          f,
          "UnkJson-{:#?}",
          serde_json::to_string_pretty(obj)
            .unwrap_or_else(|err| format!("Failed to serialize: {err:#?}"))
        ),
      }
    } else {
      match self {
        MaybeCommand::Command(cmd) => write!(f, "{:?}", cmd),
        MaybeCommand::Unknown(obj) => write!(
          f,
          "UnkJson-{}",
          serde_json::to_string(obj).unwrap_or_else(|err| format!("Failed to serialize: {err:?}"))
        ),
      }
    }
  }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "cmd", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Command {
  #[serde(rename = "DANMU_MSG")]
  Danmaku {
    #[serde(rename = "dm_v2")]
    v2: String,
    info: Box<serde_json::Value>,
  },
  GuardBuy {
    data: Box<GuardBuy>,
  },
  OnlineRankCount {
    data: OnlineRankCount,
  },
  StopLiveRoomList {
    data: StopLiveRoomList,
  },
  WatchedChange {
    data: Box<WatchedChange>,
  },
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct GuardBuy {
  pub uid: u64,
  pub username: String,
  pub guard_level: GuardLevel,
  pub num: u32,
  pub price: u32, // 电池
  pub gift_id: u32,
  #[serde_as(deserialize_as = "NoneAsEmptyString")]
  pub gift_name: Option<String>,
  pub start_time: u64,
  pub end_time: u64,
}

#[repr(u8)]
#[derive(Deserialize_repr, Debug)]
pub enum GuardLevel {
  None = 0,
  Governor = 1, // 总督
  Admiral = 2,  // 提督
  Captain = 3,  // 舰长
}

#[derive(Deserialize, Debug)]
pub struct OnlineRankCount {
  pub count: u32,
}

#[derive(Deserialize, Debug)]
pub struct StopLiveRoomList {
  #[serde(rename = "room_id_list")]
  pub list: Vec<u64>,
}

#[derive(Deserialize, Debug)]
pub struct WatchedChange {
  pub num: u32,
  pub text_large: String,
  pub text_small: String,
}
