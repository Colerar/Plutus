use std::{collections::HashMap, fmt};

use anyhow::{bail, Context};
use either::Either;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;
use serde_with::{serde_as, BoolFromInt, NoneAsEmptyString};
use time::OffsetDateTime;

use crate::data::{macros::*, share::*};

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
    #[serde(flatten)]
    data: Box<Danmaku>,
  },
  GuardBuy {
    data: Box<GuardBuy>,
  },
  OnlineRankCount {
    data: OnlineRankCount,
  },
  StopLiveRoomList {
    data: Box<StopLiveRoomList>,
  },
  WatchedChange {
    data: Box<WatchedChange>,
  },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Danmaku {
  #[serde(rename = "dm_v2")]
  pub v2: String,
  #[serde(rename = "info")]
  pub raw_info: Box<serde_json::Value>,
  #[serde(skip)]
  data: OnceCell<DanmakuData>,
}

impl Danmaku {
  #[allow(dead_code)]
  pub fn data(&self) -> anyhow::Result<&DanmakuData> {
    self
      .data
      .get_or_try_init(|| DanmakuData::de_from_value(&self.raw_info))
  }
}

#[derive(Debug)]
pub struct DanmakuData {
  pub content: String,
  pub metadata: DanmakuMeta,
  pub user: DanmakuUser,
  pub medal: Option<UserMedal>,
  pub level: UserLevel,
}

impl DanmakuData {
  fn de_from_value(value: &serde_json::Value) -> anyhow::Result<DanmakuData> {
    use json_value::*;
    let info = value.as_array().context("DanmakuInfo expected Array")?;
    get_as_str!(let content = info[1]);
    get_as_array!(
      let metadata = info[0];
      let user = info[2];
      let medal = info[3];
      let level = info[4];
    );

    let metadata =
      DanmakuMeta::de_from_vec_value(metadata).context("Failed to deserialize DanmakuMeta")?;

    let user = DanmakuUser::de_from_vec_value(user).context("Failed to deserialize DanmakuUser")?;
    let medal = if medal.is_empty() {
      None
    } else {
      Some(UserMedal::de_from_vec_value(medal).context("Failed to deserialize UserMedal")?)
    };

    let level = UserLevel::de_from_vec_value(level).context("Failed to deserialize UserLevel")?;

    Ok(DanmakuData {
      content: content.into(),
      metadata,
      user,
      medal,
      level,
    })
  }
}

#[derive(Debug)]
pub struct DanmakuMeta {
  pub mode: DanmakuMode,
  pub font_size: u64,
  pub color: Option<RgbColor>,
  pub send_time: Option<OffsetDateTime>,
  pub uid_crc32: Option<u32>,
  pub is_emoticon: bool,
  pub emoticon: Option<Emoticon>,
  pub extra: MetaExtra,
}

impl DanmakuMeta {
  fn de_from_vec_value(danamku_meta: &[serde_json::Value]) -> anyhow::Result<DanmakuMeta> {
    use json_value::*;
    get_as_u64!(
      let mode = danamku_meta[1];
      let font_size = danamku_meta[2];
      let color = danamku_meta[3];
    );
    get_as_i64!(
      let send_time = danamku_meta[4];
    );
    get_as_str!(
      let uid_crc32 = danamku_meta[7];
    );

    let mode = DanmakuMode::from_u64(mode).context("Failed to parse int as DanmakuMode")?;
    let color = u32::try_from(color).ok().map(RgbColor::from_u32);
    let send_time = OffsetDateTime::from_unix_timestamp(send_time).ok();
    let mut buf = [0u8; 4];
    let uid_crc32 = hex::decode_to_slice(uid_crc32, &mut buf)
      .map(|_| u32::from_ne_bytes(buf))
      .ok();
    get_as_u64_as_bool!(let is_emoticon = danamku_meta[12]);
    let emoticon: Option<Emoticon> = if is_emoticon {
      get_as_obj_struct!(let emoticon: Emoticon = danamku_meta[13]);
      emoticon
    } else {
      None
    };
    get_value!(let extra_parent = danamku_meta[15]);
    let extra = extra_parent
      .as_object()
      .context("Failed to get danamku_meta.15, not an object")?
      .get("extra")
      .context("Failed to get danamku_meta.15.extra")?
      .as_str()
      .context("Failed to get danamku_meta.15.extra, not a str")?;
    let extra: MetaExtra =
      serde_json::from_str(extra).context("Faield to deserialize MetaExtra")?;
    Ok(DanmakuMeta {
      mode,
      font_size,
      color,
      send_time,
      uid_crc32,
      is_emoticon,
      emoticon,
      extra,
    })
  }
}

#[derive(Deserialize, Debug)]
pub struct MetaExtra {
  #[serde(rename = "emots")]
  pub emoticons: Option<HashMap<String, InlineEmoticon>>,
}

/// Emoticon inline in message text
#[derive(Deserialize, Debug)]
pub struct InlineEmoticon {
  #[serde(rename = "emoticon_id")]
  pub id: u64,
  #[serde(rename = "emoticon_unique")]
  pub unique: String,
  pub emoji: String,
  pub descript: String,
  pub url: String,
  pub width: u32,
  pub height: u32,
  pub count: usize,
}

#[derive(Debug, FromPrimitive)]
#[repr(u8)]
pub enum DanmakuMode {
  Normal = 1,
  Bottom = 4,
  Top = 5,
  Back = 6, // 逆向
  Special = 7,
  Advanced = 9, // 高级
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Emoticon {
  #[serde(rename = "emoticon_unique")]
  pub unique: String,
  pub url: String,
  #[serde_as(as = "BoolFromInt")]
  pub bulge_display: bool,
  #[serde_as(as = "BoolFromInt")]
  pub in_player_area: bool,
  #[serde_as(as = "BoolFromInt")]
  pub is_dynamic: bool,
  pub height: u32,
  pub width: u32,
}

#[derive(Debug)]
pub struct DanmakuUser {
  pub uid: u64,
  pub username: String,
  pub is_admin: bool,               // 房管
  pub is_month_vip: bool,           // 月费老爷
  pub is_year_vip: bool,            // 年费老爷
  pub name_color: Option<RgbColor>, // None 表示默认
}

impl DanmakuUser {
  fn de_from_vec_value(data: &[serde_json::Value]) -> anyhow::Result<DanmakuUser> {
    use json_value::*;

    get_as_u64!(let uid = data[0]);
    get_as_str!(
      let username = data[1];
      let name_color = data[7];
    );
    let name_color = RgbColor::from_str(name_color);
    get_as_u64_as_bool!(
      let is_admin = data[2];
      let is_year_vip = data[3];
      let is_month_vip = data[4];
    );
    Ok(DanmakuUser {
      uid,
      username: username.into(),
      is_admin,
      is_month_vip,
      is_year_vip,
      name_color,
    })
  }
}

#[derive(Debug)]
pub struct UserLevel {
  pub level: u32,
  pub rank: Either<String, u32>,
}

#[derive(Debug)]
pub struct UserMedal {
  pub is_active: bool,
  pub name: String,
  pub level: u32,
  pub guard_level: GuardLevel,
  pub liver_name: String,
  pub liver_uid: u64,
  pub room_id: u64,
  pub color: RgbColor,
  pub gradient: (RgbColor, RgbColor, RgbColor),
}

impl UserMedal {
  fn de_from_vec_value(data: &[serde_json::Value]) -> anyhow::Result<UserMedal> {
    use json_value::*;
    get_as_u64!(
      let level = data[0];
      let room_id = data[3];
      let color = data[4];
      let gradient1 = data[7];
      let gradient2 = data[8];
      let gradient3 = data[9];
      let guard_level = data[10];
      let liver_uid = data[12];
    );
    get_as_string!(
      let name = data[1];
      let liver_name = data[2];
    );
    get_as_u64_as_bool!(
      let is_active = data[11];
    );

    let level = u32::from_u64(level).context("Failed to convert $.0 level u64 to u32")?;
    let color = RgbColor::from_u64(color).context("Failed to convert $.4 u64 to RgbColor")?;
    let gradient1 =
      RgbColor::from_u64(gradient1).context("Failed to convert $.7 u64 to RgbColor")?;
    let gradient2 =
      RgbColor::from_u64(gradient2).context("Failed to convert $.8 u64 to RgbColor")?;
    let gradient3 =
      RgbColor::from_u64(gradient3).context("Failed to convert $.9 u64 to RgbColor")?;
    let guard_level =
      GuardLevel::from_u64(guard_level).context("Failed to convert $.10 u64 to GuardLevel")?;

    Ok(UserMedal {
      is_active,
      name,
      level,
      guard_level,
      liver_name,
      liver_uid,
      room_id,
      color,
      gradient: (gradient1, gradient2, gradient3),
    })
  }
}

impl UserLevel {
  fn de_from_vec_value(data: &[serde_json::Value]) -> anyhow::Result<UserLevel> {
    use json_value::*;
    use Either::*;
    get_as_u64!(let level = data[0]);
    let level = u32::try_from(level).context("$.0 level is not in u32")?;
    get_value!(let rank = data[3]);
    let rank = if rank.is_string() {
      Left(rank.as_str().unwrap().to_owned())
    } else if rank.is_u64() {
      let rank = u32::try_from(rank.as_u64().unwrap()).context("$.3 rank is not in u32")?;
      Right(rank)
    } else {
      bail!("$.3 rank is not string or u64, but: {rank}")
    };
    Ok(UserLevel { level, rank })
  }
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
#[derive(Deserialize_repr, FromPrimitive, Debug)]
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
