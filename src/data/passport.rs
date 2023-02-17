use either::Either;
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;

#[derive(Deserialize, Debug)]
pub struct QrCodeGetResp {
  pub code: Option<i32>,
  pub status: Option<bool>,
  #[serde(rename = "ts")]
  pub timestamp: Option<u64>,
  pub data: Option<QrGetData>,
}

#[derive(Deserialize, Debug)]
pub struct QrGetData {
  pub url: String,
  #[serde(rename = "oauthKey")]
  pub oauth_key: String,
}

impl QrGetData {
  #[allow(dead_code)]
  pub fn as_req(&self) -> QrLoginReq {
    QrLoginReq {
      oauth_key: self.oauth_key.as_str(),
    }
  }
}

#[derive(Serialize, Debug)]
pub struct QrLoginReq<'a> {
  #[serde(rename = "oauthKey")]
  pub oauth_key: &'a str,
}

#[derive(Deserialize, Debug)]
pub struct QrLoginResp {
  pub code: Option<i32>,
  pub message: Option<String>,
  #[serde(rename = "ts")]
  pub timestamp: Option<u64>,
  pub status: Option<bool>,
  #[serde(with = "either::serde_untagged")]
  pub data: Either<QrLoginErr, QrLoginOk>,
}

#[derive(Deserialize, Debug)]
pub struct QrLoginOk {
  /// For Bilibili Game, cross-domain url
  pub url: String,
}

#[derive(Deserialize_repr, Debug)]
#[repr(i32)]
pub enum QrLoginErr {
  #[serde(other)]
  Unknown,
  KeyError = -1,
  KeyExpired = -2,
  NotScan = -4,
  NotConfirm = -5,
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  #[test]
  fn qr_login_resp_de_test() {
    let data: QrLoginResp = serde_json::from_value(json! {
      {
        "code": 0,
        "message": "okay",
        "data": { "url": "https://example.org" },
      }
    })
    .unwrap();
    assert_eq!(data.data.unwrap_right().url.as_str(), "https://example.org");

    let data: QrLoginResp = serde_json::from_value(json! {
      {
        "code": -200,
        "message": "not okay",
        "data": -4,
      }
    })
    .unwrap();
    assert_eq!(data.data.unwrap_left() as i32, -4);

    let data: QrLoginResp = serde_json::from_value(json! {
      {
        "code": -10,
        "message": "not okay",
        "data": -200,
      }
    })
    .unwrap();
    dbg!(&data);
  }
}
