use crate::data::info::*;

use super::*;

#[allow(dead_code)]
impl Info<'_> {
  get_json_resp_fn!(
    pub get_nav_info() [url: NAV_INFO.clone()] -> NavInfoResp;
  );
  get_json_resp_fn!(
    pub get_spi() [url: SPI.clone()] -> SpiResp;
  );
}
