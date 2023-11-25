use std::{error::Error as StdError, fmt::Display, ops::Deref};

use anyhow::Context;
use axum::{http::StatusCode, response::IntoResponse};
use diesel::result::Error as DieselError;

use crate::resp::{AppCode, Resp};

pub type AppResult<T, E = AnyhowWrapper> = Result<T, E>;
pub type AppResp<T = (), E = AnyhowWrapper> = Result<Resp<T>, E>;

#[repr(transparent)]
#[must_use]
#[derive(thiserror::Error, Debug)]
#[error("{:?}", self.0)]
pub struct AnyhowWrapper(pub anyhow::Error);

#[macro_export]
macro_rules! app_err {
  ($resp_code:expr, $msg:literal $(,)?) => {{
    $crate::error::AnyhowWrapper(
      ::anyhow::anyhow!($msg).context(
        $crate::error::AppError::new()
        .resp_code($resp_code)
      )
    )
  }};
  ($resp_code:expr, $fmt:expr, $($arg:tt)*) => {{
    $crate::error::AnyhowWrapper(
      ::anyhow::anyhow!($fmt, $($arg)*).context(
        $crate::error::AppError::new()
          .resp_code($resp_code)
      )
    )
  }};
}

#[macro_export]
macro_rules! app_err_custom {
  ($http_code:expr, $resp_code:expr, $msg:literal $(,)?) => {{
    $crate::error::AnyhowWrapper(
      ::anyhow::anyhow!($msg).context(
        $crate::error::AppError {
          resp_code: $resp_code,
          http_code: $http_code,
        }
      )
    )
  }};
  ($http_code:expr, $resp_code:expr, $fmt:expr, $($arg:tt)*) => {{
    $crate::error::AnyhowWrapper(
      ::anyhow::anyhow!($fmt, $($arg)*).context(
        $crate::error::AppError {
          resp_code: $resp_code,
          http_code: $http_code,
        }
      )
    )
  }};
}

impl AnyhowWrapper {
  #[inline]
  pub fn new(inner: anyhow::Error) -> Self {
    Self(inner)
  }

  #[inline]
  pub fn new_inner<E>(inner: E) -> Self
  where
    E: StdError + Send + Sync + 'static,
  {
    Self(anyhow::Error::new(inner))
  }

  #[inline]
  pub fn into_inner(self) -> anyhow::Error {
    self.0
  }

  #[inline]
  pub fn app_error(&self) -> Option<&AppError> {
    self.0.downcast_ref::<AppError>()
  }

  pub fn message(&self) -> String {
    self
      .0
      .chain()
      .nth(1)
      .map(|err| err.to_string())
      .unwrap_or_else(|| "Server unknown error".to_string())
  }
}

impl From<anyhow::Error> for AnyhowWrapper {
  fn from(value: anyhow::Error) -> Self {
    AnyhowWrapper(value)
  }
}

impl Deref for AnyhowWrapper {
  type Target = anyhow::Error;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl IntoResponse for AnyhowWrapper {
  fn into_response(self) -> axum::response::Response {
    let Some(app_error) = self.app_error() else {
      log::error!("Unexpected Error: {:?}", self.0);
      let json = Resp::<()>::new_failure(AppCode::SERVER_ERROR, "Server unknown error".to_string());
      return (StatusCode::OK, json).into_response();
    };

    log::info!("Unsuccessful request: {:?}", self.0);

    let resp = Resp::<()>::new_failure(app_error.resp_code, self.message());
    (app_error.http_code, resp).into_response()
  }
}

#[derive(thiserror::Error, Debug, Clone, Copy)]
#[error("Error {}, {}", self.http_code.as_u16(), self.resp_code.0)]
pub struct AppError {
  pub http_code: StatusCode,
  pub resp_code: AppCode,
}

impl Default for AppError {
  #[inline]
  fn default() -> Self {
    Self::new()
  }
}

#[allow(dead_code)]
impl AppError {
  pub fn new() -> AppError {
    AppError {
      http_code: StatusCode::OK,
      resp_code: AppCode::SERVER_ERROR,
    }
  }

  #[inline]
  pub fn http_code(mut self, code: StatusCode) -> Self {
    self.http_code = code;
    self
  }

  #[inline]
  pub fn resp_code(mut self, code: AppCode) -> Self {
    self.resp_code = code;
    self
  }
}

pub trait AnyhowExt<T> {
  fn with_app_error(self, code: AppCode) -> Self;
  fn log(self);
  fn also_log(self) -> Self;
}

impl<T> AnyhowExt<T> for anyhow::Result<T> {
  #[inline]
  fn with_app_error(self, code: AppCode) -> Self {
    self.context(AppError::new().resp_code(code))
  }
  #[inline]
  fn log(self) {
    if let Err(err) = self {
      log::error!("{err:?}");
    };
  }
  #[inline]
  fn also_log(self) -> Self {
    if let Err(ref err) = self {
      log::error!("{err:?}");
    };
    self
  }
}

pub trait IntoAppResult<T> {
  fn into_app_result(self) -> AppResult<T>;

  fn context_into_app<C>(self, context: C) -> AppResult<T>
  where
    C: Display + Send + Sync + 'static;

  fn with_context_into_app<C, F>(self, context: F) -> AppResult<T>
  where
    C: Display + Send + Sync + 'static,
    F: FnOnce() -> C;
}

impl<T> IntoAppResult<T> for anyhow::Result<T> {
  #[inline]
  fn into_app_result(self) -> AppResult<T> {
    match self {
      Ok(ok) => Ok(ok),
      Err(err) => Err(AnyhowWrapper(err)),
    }
  }

  #[inline]
  fn context_into_app<C>(self, context: C) -> AppResult<T>
  where
    C: Display + Send + Sync + 'static,
  {
    match self {
      Ok(t) => Ok(t),
      Err(e) => Err(AnyhowWrapper(e.context(context))),
    }
  }

  #[inline]
  fn with_context_into_app<C, F>(self, context: F) -> AppResult<T>
  where
    C: Display + Send + Sync + 'static,
    F: FnOnce() -> C,
  {
    match self {
      Ok(ok) => Ok(ok),
      Err(error) => Err(AnyhowWrapper(error.context(context()))),
    }
  }
}

macro_rules! impl_into_app {
  () => {
    #[inline]
    fn into_app_result(self) -> AppResult<T> {
      match self {
        Ok(ok) => Ok(ok),
        Err(err) => Err(AnyhowWrapper(anyhow::Error::new(err))),
      }
    }
  };
}

impl<T> IntoAppResult<T> for Result<T, DieselError> {
  impl_into_app!();

  fn context_into_app<C>(self, context: C) -> AppResult<T>
  where
    C: Display + Send + Sync + 'static,
  {
    self
      .context(context)
      .with_app_error(AppCode::DATABASE_ERROR)
      .into_app_result()
  }

  fn with_context_into_app<C, F>(self, context: F) -> AppResult<T>
  where
    C: Display + Send + Sync + 'static,
    F: FnOnce() -> C,
  {
    self
      .with_context(context)
      .with_app_error(AppCode::DATABASE_ERROR)
      .into_app_result()
  }
}
