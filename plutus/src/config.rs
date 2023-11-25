use std::{
  fs::File,
  io::{BufReader, Read},
  net::SocketAddr,
  path::Path,
  str::FromStr,
};

use anyhow::Context;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
  #[serde(alias = "addr", default = "Config::default_address")]
  pub address: SocketAddr,
  pub database_url: String,
  pub rooms: Vec<u64>,
}

impl Config {
  fn default_address() -> SocketAddr {
    SocketAddr::from_str("127.0.0.1:7727").unwrap()
  }

  pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
    let path = path.as_ref();
    let file = File::open(path)
      .with_context(|| format!("Failed to open config file `{}`", path.to_string_lossy()))?;
    let size_hint = file
      .metadata()
      .map(|metadata| metadata.len() as usize)
      .unwrap_or(8 * 1024);
    let mut buf = String::with_capacity(size_hint);
    BufReader::new(file)
      .read_to_string(&mut buf)
      .with_context(|| format!("Failed to read config file `{}`", path.to_string_lossy()))?;
    toml::from_str(&buf).with_context(|| {
      format!(
        "Failed to deserilaize config file: {}",
        path.to_string_lossy()
      )
    })
  }
}
