use std::{
  env,
  io::{stdin, BufRead},
};

pub mod api;
pub mod client;
pub mod data;
pub mod path;
pub mod serde_as;

use anyhow::Context;
use api::live::MessageConnection;
use client::Client;
use futures_util::StreamExt;
use pretty_env_logger::formatted_builder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  async_main().await
}

async fn async_main() -> anyhow::Result<()> {
  init_logger()?;
  path::init()?;
  let client = Client::new()?;

  println!("Room id:");
  let mut buf = String::new();
  stdin().lock().read_line(&mut buf)?;
  let room_id = buf
    .trim()
    .parse::<u64>()
    .context("Your input is not a number")?;

  let con = MessageConnection::connect_with_client(&client, room_id).await?;
  while let Some(msg) = con.write().await.next().await {
    println!("{:?}", msg);
  }

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
  init_logger().unwrap();
  if path::DATA_DIR.get().is_none() {
    path::init().unwrap();
  }
  Client::new().unwrap()
}
