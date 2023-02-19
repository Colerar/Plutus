use crate::data::info::*;

use super::*;

#[allow(dead_code)]
impl<'a> Info<'a> {
  get_json_resp_fn!(
    pub get_nav_info() [url: NAV_INFO.clone()] -> NavInfoResp;
  );
}
