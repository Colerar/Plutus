macro_rules! api {
  (
    $($name:ident),+
    $(,)?
  ) => {
    $(
      paste::item! {
        pub mod [<$name:lower>];
      }

      pub struct $name<'a>(&'a crate::client::Client);

      impl<'a> $name<'a> {
        pub fn new(client: &crate::client::Client) -> $name {
          $name(client)
        }
      }
    )+
  };
}

api!(Passport, Live, Info);

macro_rules! url {
  (
    $( $name:ident: $url_expr:expr ),+
    $(,)?
  ) => {
    $(
      #[allow(dead_code)]
      pub(crate) static $name: once_cell::sync::Lazy<reqwest::Url> = once_cell::sync::Lazy::new(|| {
        reqwest::Url::parse($url_expr).unwrap()
      });
    )+
  };
}

macro_rules! url_path {
  (
    base: $base:ident,
    $( $name:ident: $url_expr:expr ),+
    $(,)?
  ) => {
    $(
      #[allow(dead_code)]
      pub(crate) static $name: once_cell::sync::Lazy<reqwest::Url> = once_cell::sync::Lazy::new(|| {
        let mut new = $base.clone();
        new.set_path($url_expr);
        new
      });
    )+
  };
}

url!(
  MAIN: "https://api.bilibili.com",
  WWW: "https://www.bilibili.com",
  LIVE: "https://live.bilibili.com",
  LIVE_API: "https://live.api.bilibili.com",
  PASSPORT: "https://passport.bilibili.com",
);

url_path!(
  base: PASSPORT,
  LOGIN_QR_GET: "qrcode/getLoginUrl",
  LOGIN_QR: "qrcode/getLoginInfo",
);

url_path!(
  base: MAIN,
  NAV_INFO: "x/web-interface/nav",
);

macro_rules! get_json_resp_fn {
  (
    $( $fn_name:ident() [url: $api_url:expr] -> $resp_data:ty );+
    $( ; )?
  ) => {
    $(
      async fn $fn_name(&self) -> anyhow::Result<$resp_data> {
        use anyhow::Context;
        self
          .0
          .client
          .get($api_url)
          .send()
          .await
          .context(concat!(stringify!($fn_name), " failed"))?
          .json()
          .await
          .context(concat!("Failed to deserialize ", stringify!($resp_data)))
      }
    )+
  };
}
pub(crate) use get_json_resp_fn;

macro_rules! post_form_json_resp_fn {
  (
    $( $fn_name:ident( $form_name:ident : $form_ty:ty ) [url: $api_url:expr] -> $resp_data:ty );+
    $( ; )?
  ) => {
    $(
      async fn $fn_name(&self, $form_name: $form_ty) -> anyhow::Result<$resp_data> {
        self
          .0
          .client
          .post($api_url)
          .form($form_name)
          .send()
          .await
          .context(concat!(stringify!($fn_name), " failed"))?
          .json()
          .await
          .context(concat!("Failed to deserialize ", stringify!($resp_data)))
      }
    )+
  };
}
pub(crate) use post_form_json_resp_fn;

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test() {
    assert_eq!(
      "https://passport.bilibili.com/qrcode/getLoginUrl",
      LOGIN_QR_GET.as_str()
    );
    assert_eq!("https://www.bilibili.com/", WWW.as_str());
  }
}
