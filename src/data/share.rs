use super::{macros::de_from_code_impl, FromCode};

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
