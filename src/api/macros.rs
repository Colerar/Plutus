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
pub(super) use api;

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
pub(super) use url;

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
pub(super) use url_path;

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
        use anyhow::Context;
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

macro_rules! get_query_json_resp_fn {
  (
    $( $fn_name:ident( $form_name:ident : $form_ty:ty ) [url: $api_url:expr] -> $resp_data:ty );+
    $( ; )?
  ) => {
    $(
      async fn $fn_name(&self, $form_name: $form_ty) -> anyhow::Result<$resp_data> {
        use anyhow::Context;
        self
          .0
          .client
          .get($api_url)
          .query($form_name)
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
pub(crate) use get_query_json_resp_fn;
