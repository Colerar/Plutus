use std::env;

use anyhow::Context;
use pretty_env_logger::formatted_builder;

mod api;
mod client;
mod data;
mod path;
mod serde_as;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  async_main().await
}

async fn async_main() -> anyhow::Result<()> {
  init_logger()?;
  path::init()?;

  Ok(())
}

fn init_logger() -> anyhow::Result<()> {
  let var = env::var("PLUTUS_LOG");
  let log_expr = if let Ok(ref ok) = var {
    ok.as_str()
  } else if cfg!(debug_assertions) {
    "debug"
  } else {
    "info"
  };
  let mut builder = formatted_builder();
  builder.parse_filters(log_expr);
  builder.try_init().context("Failed to init logger")?;
  Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
fn new_test_client() -> client::Client {
  use client::Client;
  init_logger().unwrap();
  if path::DATA_DIR.get().is_none() {
    path::init().unwrap();
  }
  Client::new().unwrap()
}
