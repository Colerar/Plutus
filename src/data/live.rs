use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;
use serde_with::{serde_as, BoolFromInt, DefaultOnNull};

use super::{macros::*, share::*, *};

#[derive(Serialize, Debug)]
pub struct UidToRoomIdReq {
  pub uid: u64,
}

impl From<u64> for UidToRoomIdReq {
  fn from(uid: u64) -> Self {
    Self { uid }
  }
}

#[derive(Deserialize, Debug)]
pub struct UidToRoomIdResp {
  pub code: Option<i32>,
  pub message: Option<String>,
  pub data: Option<RoomId>,
}

#[derive(Deserialize, Debug)]
pub struct RoomId {
  pub room_id: u64,
}

#[derive(Serialize, Debug)]
pub struct InitReq {
  #[serde(rename = "id")]
  pub room_id: u64,
}

impl From<u64> for InitReq {
  fn from(room_id: u64) -> Self {
    Self { room_id }
  }
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct InitResp {
  pub code: Option<i32>,
  pub message: Option<String>,
  pub data: Option<InitData>,
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct InitData {
  pub room_id: Option<u64>,
  pub short_id: Option<u64>,
  pub uid: Option<u64>,
  #[serde_as(as = "Option<BoolFromInt>")]
  pub need_p2p: Option<bool>,
  pub is_hidden: Option<bool>,
  pub is_locked: Option<bool>,
  pub is_portrait: Option<bool>,
  pub live_status: Option<LiveStatus>,
  pub hidden_till: Option<u64>,
  pub lock_till: Option<u64>,
  pub encrypted: Option<bool>,
  pub pwd_verified: Option<bool>,
  pub live_time: Option<u64>,
  #[serde(rename = "is_sp")]
  #[serde_as(as = "Option<BoolFromInt>")]
  pub is_special: Option<bool>,
  pub special_type: Option<SpecialType>,
}

#[derive(Debug)]
pub enum SpecialType {
  Normal,
  Paid,
  NewYear,
  Unknown(i32),
}

de_from_code_impl!(SpecialType);

impl FromCode for SpecialType {
  fn from_code(code: i32) -> Self {
    use SpecialType::*;
    match code {
      0 => Normal,
      1 => Paid,
      2 => NewYear,
      unk => Unknown(unk),
    }
  }
}

#[derive(Debug, Deserialize_repr)]
#[repr(u8)]
pub enum LiveStatus {
  Stop = 0, // 暂停
  Live = 1, // 直播
  Carousels = 2, // 轮播
}

#[derive(Serialize, Debug)]
pub struct DanmakuReq {
  #[serde(rename = "id")]
  pub room_id: u64, // real room id
}

impl From<u64> for DanmakuReq {
  fn from(room_id: u64) -> Self {
    Self { room_id }
  }
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct DanmakuResp {
  pub code: Option<i32>,
  pub message: Option<String>,
  pub ttl: Option<i32>,
  pub data: Option<WssDanmaku>,
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct WssDanmaku {
  pub group: String,
  pub refresh_row_factor: f32,
  pub refresh_rate: f32,
  pub max_delay: u32,
  pub token: String,
  #[serde_as(deserialize_as = "DefaultOnNull")]
  pub host_list: Vec<WssHost>,
}

#[derive(Deserialize, Debug)]
pub struct WssHost {
  pub host: String,
  pub port: u16,
  pub wss_port: u16,
  pub ws_port: u16,
}
