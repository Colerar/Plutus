use crate::{data::live::*};

use super::*;

#[allow(dead_code)]
impl<'a> Live<'a> {
  get_query_json_resp_fn!(
    // UID to real room id
    uid_to_room_id(qr_req: &UidToRoomIdReq) [url: UID_TO_ROOM_ID.clone()] -> UidToRoomIdResp;
    init_room(qr_req: &InitReq) [url: ROOM_INIT.clone()] -> InitResp;
    danmaku_info(qr_req: &DanmakuReq) [url: LIVE_DANMAKU.clone()] -> DanmakuResp;
  );
}
