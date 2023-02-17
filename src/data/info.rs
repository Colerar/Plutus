use serde::Deserialize;
use serde_with::{serde_as, BoolFromInt, NoneAsEmptyString, DefaultOnError};

use super::{macros::*, share::*, *};

#[derive(Deserialize, Debug)]
pub struct NavInfoResp {
  pub code: Option<i32>,
  pub message: Option<String>,
  pub ttl: Option<i32>,
  pub data: NavInfo,
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct NavInfo {
  #[serde(rename = "isLogin")]
  pub is_login: bool,
  #[serde_as(as = "Option<BoolFromInt>")]
  pub email_verified: Option<bool>,
  #[serde_as(as = "Option<BoolFromInt>")]
  pub mobile_verified: Option<bool>,
  #[serde(rename = "face")]
  pub avatar: Option<String>,
  pub level_info: Option<LevelInfo>,
  pub mid: Option<u64>,
  #[serde(rename = "money")]
  pub coin: Option<f64>, // 硬币
  #[serde(rename = "moral")]
  pub moral: Option<f64>, // 节操
  #[serde(rename = "official")]
  pub official: Option<OfficialData>,
  #[serde(rename = "uname")]
  pub username: Option<String>,
  #[serde(rename = "vipDueDate")]
  pub vip_due_date: Option<i64>,
  #[serde(rename = "vipStatus")]
  pub vip_status: VipStatus,
  #[serde(rename = "vipType")]
  pub vip_type: Option<VipType>,
  #[serde_as(as = "Option<BoolFromInt>")]
  pub is_senior_member: Option<bool>, // 硬核会员
  pub is_jury: Option<bool>, // 风纪委员
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct OfficialData {
  pub role: Option<OfficialRole>,
  #[serde_as(deserialize_as = "NoneAsEmptyString")]
  pub title: Option<String>,
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct LevelInfo {
  pub current_level: u32,
  pub current_min: u32,
  pub current_exp: u32,
  #[serde_as(as = "DefaultOnError")]
  pub next_exp: Option<u32>, // "--" for lv.6, so `DefaultOnError`
}

#[derive(Debug)]
pub enum OfficialRole {
  Not,         // 0
  Personal,     // 1, 2, 7
  Organization, // 3..=6
  Unknown(i32), // other
}

de_from_code_impl!(OfficialRole);

impl FromCode for OfficialRole {
  fn from_code(code: i32) -> Self {
    use OfficialRole::*;
    match code {
      0 => Not,
      1 | 2 | 7 => Personal,
      3..=6 => Organization,
      unk => Unknown(unk),
    }
  }
}

#[cfg(test)]
mod tests {
  use serde_json::json;

  use super::*;

  macro_rules! _de_official_role_too_long {
    ( $( $value:expr ),+ $(,)? ) => {
      $({
        let data: Result<OfficialRole, _> = serde_json::from_value(json!($value));
        assert!(matches!(data, Err(_)));
      })+
    };
  }

  #[test]
  fn de_official_role_too_long() {
    _de_official_role_too_long!(i64::MIN, i64::MAX, u64::MAX, u32::MAX);
  }

  macro_rules! _de_official_role {
    (
      $( $value:expr => $(|)? $( $pattern:pat_param )|+ $( if $guard: expr )? ),+
      $(,)?
    ) => {
      $(
        {
          let data: OfficialRole = serde_json::from_value(json!($value)).unwrap();
          assert!(
            matches!(data, $( $pattern )|+ $( if $guard )?),
            concat!(
              "Expected value `", stringify!($value),
              " ` to be parsed to `", stringify!($variant), "`, but: {:#?}"
            ),
            data
          );
        }
      )+
    };
  }

  #[test]
  fn de_official_role() {
    use OfficialRole::*;
    _de_official_role!(
      0 => Not,
      1 => Personal,
      2 => Personal,
      7 => Personal,
      3 => Organization,
      4 => Organization,
      5 => Organization,
      6 => Organization,
      100 => Unknown(100),
      -100 => Unknown(-100),
    );
  }
}
