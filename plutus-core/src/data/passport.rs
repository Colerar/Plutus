use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;

#[derive(Deserialize, Debug)]
pub struct QrCodeGetResp {
  pub code: Option<i32>,
  pub message: Option<String>,
  #[serde(rename = "ts")]
  pub ttl: Option<u64>,
  pub data: Option<QrGetData>,
}

#[derive(Deserialize, Debug)]
pub struct QrGetData {
  pub url: String,
  pub qrcode_key: String,
}

#[derive(Serialize, Debug)]
pub struct QrLoginQuery<'a> {
  pub qrcode_key: &'a str,
}

#[derive(Deserialize, Debug)]
pub struct QrLoginResp {
  pub code: Option<i32>,
  pub message: Option<String>,
  pub data: Option<QrLoginData>,
}

#[derive(Deserialize, Debug)]
pub struct QrLoginData {
  /// For Bilibili Game, cross-domain url
  pub url: String,
  pub refresh_token: String,
  pub timestamp: i64,
  #[serde(default)]
  pub code: QrLoginStatus,
}

#[repr(i32)]
#[derive(Default, Deserialize_repr, Debug, PartialEq, Eq, Clone, Copy)]
pub enum QrLoginStatus {
  #[default]
  Ok = 0,
  Expired = 86038,
  ScanNotConfirm = 86090,
  NotScan = 86101,
}
