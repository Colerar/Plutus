use self::macros::*;

mod macros;

api!(Passport, Live, Info);

url!(
  MAIN: "https://api.bilibili.com",
  WWW: "https://www.bilibili.com",
  LIVE: "https://live.bilibili.com",
  LIVE_API: "https://api.live.bilibili.com",
  PASSPORT: "https://passport.bilibili.com",
);

url_path!(
  base: PASSPORT,
  LOGIN_QR_GET: "x/passport-login/web/qrcode/generate",
  LOGIN_QR: "x/passport-login/web/qrcode/poll",
);

url_path!(
  base: MAIN,
  NAV_INFO: "x/web-interface/nav",
  SPI: "x/frontend/finger/spi"
);

url_path!(
  base: LIVE_API,
  UID_TO_ROOM_ID: "room/v2/Room/room_id_by_uid",
  ROOM_INIT: "room/v1/Room/room_init",
  LIVE_DANMAKU: "xlive/web-room/v1/index/getDanmuInfo",
);
