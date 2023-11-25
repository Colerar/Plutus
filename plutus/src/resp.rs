use std::num::NonZeroU64;

use axum::{
  response::{IntoResponse, Response},
  Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct AppCode(pub i32);

impl AppCode {
  pub const SUCCESS: AppCode = AppCode(0);
  pub const SERVER_ERROR: AppCode = AppCode(500);
  pub const DATABASE_ERROR: AppCode = AppCode(501);
  pub const INVALID_ARGUMENTS: AppCode = AppCode(510);
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Resp<T = ()> {
  pub code: AppCode,
  pub message: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub data: Option<T>,
}

impl<T> Resp<T> {
  pub fn new_failure(code: AppCode, message: String) -> Resp<T> {
    Resp {
      code,
      message,
      data: None,
    }
  }

  pub fn new_success(data: T) -> Resp<T> {
    Resp {
      code: AppCode::SUCCESS,
      message: "ok".to_string(),
      data: Some(data),
    }
  }
}

impl<T: Serialize> IntoResponse for Resp<T> {
  fn into_response(self) -> Response {
    Json(self).into_response()
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Paginated<T> {
  pub page: Page,
  pub list: Vec<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Page {
  pub current: u64,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max: Option<u64>,
  pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Cursor {
  #[serde(default = "Cursor::default_current_page")]
  pub page: NonZeroU64,
  #[serde(default = "Cursor::default_page_size")]
  pub size: NonZeroU64,
}

impl Cursor {
  #[inline]
  fn default_current_page() -> NonZeroU64 {
    NonZeroU64::new(1).unwrap()
  }
  #[inline]
  fn default_page_size() -> NonZeroU64 {
    NonZeroU64::new(20).unwrap()
  }
}

impl Default for Cursor {
  fn default() -> Self {
    Self {
      page: Self::default_current_page(),
      size: Self::default_page_size(),
    }
  }
}
