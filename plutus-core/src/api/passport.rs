use super::{get_json_resp_fn, Passport};

use crate::data::passport::*;

use crate::api::*;

#[allow(dead_code)]
impl<'a> Passport<'a> {
  get_json_resp_fn!(
    pub get_login_qr() [url: LOGIN_QR_GET.clone()] -> QrCodeGetResp;
  );
  post_form_json_resp_fn!(
    pub login_qr(qr_req: &QrLoginReq<'_>) [url: LOGIN_QR.clone()] -> QrLoginResp;
  );
}
