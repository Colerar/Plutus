use std::{
  fs::{create_dir_all, File, OpenOptions},
  io::{BufReader, BufWriter},
  sync::Arc,
};

use anyhow::{anyhow, Context};
use reqwest_cookie_store::{CookieStore, CookieStoreRwLock};

#[derive(Clone)]
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
      pastey::item! {
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
    let cookie_store = CookieStore::load(buf, |cookie| ::serde_json::from_str(cookie))
      .map(CookieStoreRwLock::new)
      .map(Arc::new)
      .map_err(|err| anyhow!(err))
      .context("Failed to load cookie store")?;

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
    let cwd = std::env::current_dir().context("Cannot get current dir")?;
    let cookie_path = cwd.join("cookies.jsonl");
    log::trace!("Cookie path: {}", cookie_path.display());

    if let Some(parent) = cookie_path.parent() {
      create_dir_all(parent)
        .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    OpenOptions::new()
      .truncate(true)
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

  pub fn save_cookies(&self) {
    match Self::open_cookies_file().map(BufWriter::new) {
      Ok(mut buf) => {
        let save_result = self
          .cookie_store
          .read()
          .unwrap()
          .save(&mut buf, ::serde_json::to_string);
        if let Err(err) = save_result {
          log::error!("Failed to save cookies: {:#?}", err);
        };
      },
      Err(err) => {
        log::error!("Failed to open cookie storage file: {:#?}", err);
      },
    }
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
    self.save_cookies();
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
