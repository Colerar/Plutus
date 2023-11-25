use std::{net::SocketAddr, ops::Sub, time::Duration};

use anyhow::Context;
use axum::{
  http::StatusCode,
  response::{IntoResponse, Response},
  routing::{get, post},
  Json, Router,
};
use chrono::{DateTime, Utc};
use diesel::{pg::Pg, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use tower_http::{compression::CompressionLayer, timeout::TimeoutLayer};

use crate::{
  app_err,
  error::{AppResp, IntoAppResult},
  global_state,
  models::Log,
  resp::{AppCode, Cursor, Page, Paginated, Resp},
  schema::logs,
  state::AsyncPoolConnection,
  PLUTUS_VERSION,
};

pub async fn server(addr: &SocketAddr) -> anyhow::Result<()> {
  let router = Router::new()
    .route("/", get(index))
    .route("/list", post(list))
    .fallback(get(fallback))
    .layer(TimeoutLayer::new(Duration::from_secs(5 * 60)))
    .layer(CompressionLayer::new());
  log::info!("Binding server to {addr}");
  axum::Server::try_bind(addr)
    .context("Failed to bind server")?
    .serve(router.into_make_service())
    .await
    .context("Failed to create server")?;
  Ok(())
}

async fn index() -> String {
  format!("Plutus v{}", PLUTUS_VERSION)
}

async fn fallback() -> Response {
  (StatusCode::NOT_FOUND, "No such route").into_response()
}

#[derive(Serialize, Deserialize)]
pub struct QueryBody {
  pub room_id: u64,
  #[serde(default)]
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub commands: Vec<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub uid: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub time_range: Option<TimeRange>,
  #[serde(default)]
  pub cursor: Cursor,
}

#[derive(Serialize, Deserialize, Default)]
pub struct TimeRange {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub start: Option<DateTime<Utc>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub end: Option<DateTime<Utc>>,
}

async fn list(Json(body): Json<QueryBody>) -> AppResp<Paginated<Log>> {
  let conn: &mut AsyncPoolConnection = &mut global_state().db_con().await?;

  fn new_query(
    body: &QueryBody,
  ) -> diesel::query_builder::BoxedSelectStatement<
    '_,
    logs::SqlType,
    diesel::query_builder::FromClause<logs::table>,
    Pg,
  > {
    let mut query = logs::table
      .into_boxed()
      .filter(logs::room_id.eq(body.room_id as i64));
    if !body.commands.is_empty() {
      query = query.filter(logs::command.eq_any(&body.commands));
    }
    if let Some(uid) = body.uid {
      query = query.filter(logs::related_uid.eq(uid as i64));
    }
    if let Some(TimeRange { start, end }) = body.time_range {
      if let Some(start) = start {
        query = query.filter(logs::time.ge(start));
      }
      if let Some(end) = end {
        query = query.filter(logs::time.le(end));
      }
    }
    query
  }

  let offset = (body.cursor.page.get().sub(1) * body.cursor.size.get()) as i64;
  let limit = body.cursor.size.get() as i64;

  let count: i64 = new_query(&body)
    .count()
    .get_result::<i64>(conn)
    .await
    .context_into_app("Failed to count columns size")?;

  let max = (count as u64).div_ceil(body.cursor.size.get());

  #[allow(clippy::collapsible_if)]
  if max != 0 {
    if !(1..=max).contains(&body.cursor.page.get()) || !(1..=1000).contains(&body.cursor.size.get())
    {
      return Err(app_err!(AppCode::INVALID_ARGUMENTS, "Invalid cursor"));
    }
  }

  let logs: Vec<Log> = new_query(&body)
    .limit(limit)
    .offset(offset)
    .order_by(logs::time)
    .get_results(conn)
    .await
    .context_into_app("Failed to query logs")?;

  Ok(Resp::new_success(Paginated {
    page: Page {
      current: body.cursor.page.get(),
      max: Some(max),
      size: logs.len() as u64,
    },
    list: logs,
  }))
}
