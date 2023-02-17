use std::{
  fs::{self, File, OpenOptions},
  io::{BufReader, BufWriter},
  sync::Arc,
};

use anyhow::Context;
use reqwest_cookie_store::{CookieStore, CookieStoreRwLock};

use crate::path::DATA_DIR;

pub struct Client {
  #[allow(dead_code)] // used it in macro, cannot detect
  pub(crate) client: reqwest::Client,
  pub(crate) cookie_store: Arc<CookieStoreRwLock>,
}

macro_rules! api_getter {
  (
    $( $api_struct:ident ),+
    $(,)?
  ) => {
    $(
      paste::item! {
        pub fn [<$api_struct:lower>](&self) -> crate::api::$api_struct {
          crate::api::$api_struct::new(self)
        }
      }
    )+
  };
}

#[allow(dead_code)]
impl Client {
  api_getter!(Passport, Live, Info);

  pub fn new() -> anyhow::Result<Client> {
    let buf = Self::open_cookies_file().map(BufReader::new)?;
    let cookie_store = CookieStore::load_json(buf)
      .map(CookieStoreRwLock::new)
      .map(Arc::new)
      .unwrap();

    let client = reqwest::ClientBuilder::new()
      .cookie_provider(Arc::clone(&cookie_store))
      // Reqwest respect the system's proxy configuration, but need the
      // `socks5` feature to be enabled, we already enabled it.
      .build()
      .context("Failed to build reqwest Client")?;
    Ok(Client {
      client,
      cookie_store,
    })
  }

  fn open_cookies_file() -> anyhow::Result<File> {
    let cookie_path = DATA_DIR
      .get()
      .context("Failed to get DATA_DIR")?
      .join("cookies.jsonl");
    log::debug!("Cookie path: `{}`", cookie_path.to_string_lossy());
    if let Some(parent) = cookie_path.parent() {
      if !parent.exists() {
        fs::create_dir_all(parent)
          .with_context(|| format!("Failed to create directory: {}", parent.to_string_lossy()))?;
      }
    }
    OpenOptions::new()
      .create(true)
      .read(true)
      .write(true)
      .open(cookie_path)
      .context("Failed to create cookie file")
  }

  pub fn csrf(&self) -> Option<String> {
    self
      .cookie_store
      .read()
      .unwrap()
      .get_bili("bili_jct")
      .map(|cookie| cookie.value().to_string())
  }

  pub fn clear_cookies(&self) {
    let mut cookies = self.cookie_store.write().unwrap();
    cookies.clear();
  }

  /// Check login offline
  pub fn check_login_offline(&self) -> bool {
    let cookies = self.cookie_store.read().unwrap();
    cookies.contains_bili(<CookieStore as CookiesBiliExt>::SESSDATA)
      && cookies.contains_bili(<CookieStore as CookiesBiliExt>::CSRF)
  }
}

impl Drop for Client {
  fn drop(&mut self) {
    match Self::open_cookies_file().map(BufWriter::new) {
      Ok(mut buf) => {
        let save_result = self.cookie_store.read().unwrap().save_json(&mut buf);
        if let Err(err) = save_result {
          log::error!("Failed to save cookies: {:#?}", err);
        };
      }
      Err(err) => {
        log::error!("Failed to open cookie storage file: {:#?}", err);
      }
    }
  }
}

trait CookiesBiliExt {
  const DOMAIN: &'static str = "bilibili.com";
  const ROOT: &'static str = "/";
  const CSRF: &'static str = "bili_jct";
  const SESSDATA: &'static str = "SESSDATA";

  fn contains_bili(&self, name: &str) -> bool;
  fn get_bili(&self, name: &str) -> Option<&cookie_store::Cookie>;
}

impl CookiesBiliExt for CookieStore {
  #[inline]
  fn contains_bili(&self, name: &str) -> bool {
    self.contains(Self::DOMAIN, Self::ROOT, name)
  }

  #[inline]
  fn get_bili(&self, name: &str) -> Option<&cookie_store::Cookie> {
    self.get(Self::DOMAIN, Self::ROOT, name)
  }
}
