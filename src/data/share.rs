use serde::de;

use super::{macros::*, FromCode};

#[derive(Debug)]
pub enum VipType {
  Unknown(i32),
  None,
  Month,
  Year,
}

de_from_code_impl!(VipType);

impl FromCode for VipType {
  fn from_code(code: i32) -> Self {
    use VipType::*;
    match code {
      1 => None,
      2 => Month,
      3 => Year,
      unk => Unknown(unk),
    }
  }
}

#[derive(Debug)]
pub enum VipStatus {
  Unknown(i32),
  Normal,
  IpChangeFrequent,
  RiskLocked,
}

de_from_code_impl!(VipStatus);

impl FromCode for VipStatus {
  fn from_code(code: i32) -> Self {
    use VipStatus::*;
    match code {
      1 => Normal,
      2 => IpChangeFrequent,
      3 => RiskLocked,
      unk => Unknown(unk),
    }
  }
}

pub trait ColorFromU32 {
  fn from_u32(num: u32) -> Self;
}

pub trait ColorFromU64: Sized {
  fn from_u64(num: u64) -> Option<Self>;
}

impl<T: ColorFromU32 + Sized> ColorFromU64 for T {
  fn from_u64(num: u64) -> Option<Self> {
    u32::try_from(num).map(Self::from_u32).ok()
  }
}

pub trait ColorFromHex: Sized {
  fn from_str(num: &str) -> Option<Self>;
}

#[derive(Debug)]
pub struct RgbColor {
  pub red: u8,
  pub green: u8,
  pub blue: u8,
}

impl ColorFromU32 for RgbColor {
  fn from_u32(num: u32) -> RgbColor {
    RgbColor {
      red: ((num & 0xFF0000) >> 16) as u8,
      green: ((num & 0x00FF00) >> 8) as u8,
      blue: (num & 0x0000FF) as u8,
    }
  }
}

impl ColorFromHex for RgbColor {
  fn from_str(num: &str) -> Option<RgbColor> {
    if num.is_empty() {
      return None;
    }
    let num = if let Some(num) = num.strip_prefix('#') {
      num
    } else {
      num
    };
    let mut bytes = [0u8; 3];
    hex::decode_to_slice(num, &mut bytes).ok()?;
    Some(RgbColor {
      red: bytes[0],
      green: bytes[1],
      blue: bytes[2],
    })
  }
}

#[derive(Debug)]
pub struct RgbaColor {
  pub red: u8,
  pub green: u8,
  pub blue: u8,
  pub alpha: u8,
}

impl ColorFromU32 for RgbaColor {
  fn from_u32(num: u32) -> RgbaColor {
    RgbaColor {
      red: ((num & 0xFF000000) >> 24) as u8,
      green: ((num & 0x00FF0000) >> 16) as u8,
      blue: ((num & 0x0000FF00) >> 8) as u8,
      alpha: (num & 0x000000FF) as u8,
    }
  }
}

impl ColorFromHex for RgbaColor {
  fn from_str(num: &str) -> Option<RgbaColor> {
    if num.is_empty() {
      return None;
    }
    let num = if let Some(num) = num.strip_prefix('#') {
      num
    } else {
      num
    };
    let mut bytes = [0u8; 4];
    hex::decode_to_slice(num, &mut bytes).ok()?;
    Some(RgbaColor {
      red: bytes[0],
      green: bytes[1],
      blue: bytes[2],
      alpha: bytes[3],
    })
  }
}

macro_rules! de_color_impl {
  ($T:ty) => {
    impl<'de> de::Deserialize<'de> for $T {
      fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
      where
        D: serde::Deserializer<'de>,
      {
        struct De;
        impl<'de> de::Visitor<'de> for De {
          type Value = $T;

          fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
          where
            E: de::Error,
          {
            Ok(<$T>::from_u32(v))
          }

          fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
          where
            E: de::Error,
          {
            match <$T>::from_str(v) {
              Some(color) => Ok(color),
              None => {
                let unexp = serde::de::Unexpected::Str(v);
                Err(serde::de::Error::invalid_value(unexp, &self))
              }
            }
          }

          forward_ints::de_as!(u32: u8, u16, i8, i16);
          forward_ints::try_from_signed!(u32: i32, i64);
          forward_ints::try_from_unsigned!(u32: u64);

          fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str(concat!(stringify!($T), ", str(hex) or u32"))
          }
        }
        deserializer.deserialize_any(De)
      }
    }
  };
}

de_color_impl!(RgbColor);
de_color_impl!(RgbaColor);

#[cfg(test)]
mod tests {
  use crate::data::share::ColorFromU32;

  use super::RgbColor;

  #[test]
  fn test() {
    dbg!(RgbColor::from_u32(14893055));
  }
}
