use std::path::PathBuf;

use anyhow::Context;
use directories::ProjectDirs;
use once_cell::sync::OnceCell;

pub static DATA_DIR: OnceCell<PathBuf> = OnceCell::new();
pub static CONFIG_DIR: OnceCell<PathBuf> = OnceCell::new();
pub static CACHE_DIR: OnceCell<PathBuf> = OnceCell::new();

macro_rules! set_or_bail {
  ($dir:expr, $const:ident) => {
    if let Err(dir) = $const.set($dir) {
      anyhow::bail!(
        concat!(
          "Failed to set `",
          stringify!($const),
          "`: `{}`, already set one previously"
        ),
        dir.to_string_lossy(),
      )
    }
  };
}

pub(crate) fn init() -> anyhow::Result<()> {
  let proj = ProjectDirs::from("moe.sdl", "", "plutus").context("Failed to init project dirs")?;
  set_or_bail!(proj.data_local_dir().to_path_buf(), DATA_DIR);
  set_or_bail!(proj.config_dir().to_path_buf(), CONFIG_DIR);
  set_or_bail!(proj.cache_dir().to_path_buf(), CACHE_DIR);
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn init_twice() {
    DATA_DIR.set(PathBuf::new()).unwrap();
    let err = init().unwrap_err();
    println!("{:#?}", err);
  }
}
