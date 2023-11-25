use std::{sync::Arc, time::Duration};

use anyhow::Context;

use crate::{config::Config, error::*, resp::AppCode};
use diesel_async::{
  pooled_connection::{AsyncDieselConnectionManager, PoolableConnection, RecyclingMethod},
  AsyncPgConnection,
};

pub type AsyncPool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;
pub type AsyncPoolConnection<'a> =
  bb8::PooledConnection<'a, AsyncDieselConnectionManager<AsyncPgConnection>>;

#[derive(Clone, Debug)]
pub struct State {
  pub config: Arc<Config>,
  db_pool: AsyncPool,
}

impl State {
  pub async fn init() -> anyhow::Result<Self> {
    let config_path =
      std::env::var("PLUTUS_CONFIG").unwrap_or_else(|_err| "plutus-config.toml".to_string());
    let config = Config::load(config_path)
      .context("Failed to load plutus config")
      .map(Arc::new)?;

    let db_pool = bb8::Pool::builder()
      .connection_timeout(Duration::from_secs(3))
      .build(AsyncDieselConnectionManager::<AsyncPgConnection>::new(
        &config.database_url,
      ))
      .await
      .context("Failed to create bb8 pool.")?;

    db_pool
      .get()
      .await
      .context("Failed to get db connection from pool")?
      .ping(&RecyclingMethod::Verified)
      .await
      .context("Failed to ping database")?;

    Ok(State { config, db_pool })
  }

  pub async fn db_con(&self) -> AppResult<AsyncPoolConnection<'_>> {
    self
      .db_pool
      .get()
      .await
      .context("Failed to get pooled database connection")
      .with_app_error(AppCode::DATABASE_ERROR)
      .into_app_result()
  }

  pub async fn db_con_owned(&self) -> AppResult<AsyncPoolConnection<'static>> {
    self
      .db_pool
      .get_owned()
      .await
      .context("Failed to get pooled database connection")
      .with_app_error(AppCode::DATABASE_ERROR)
      .into_app_result()
  }
}
