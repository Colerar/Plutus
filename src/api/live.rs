use std::{io::Cursor, sync::Arc, time::Duration};

use super::*;
use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use reqwest::Url;
use tokio::{
  sync::{
    mpsc::{self, Receiver},
    RwLock,
  },
  task::JoinHandle,
};
use tokio_tungstenite as tokio_ws2;
use tokio_ws2::{
  connect_async,
  tungstenite::{self as ws2, protocol::WebSocketConfig},
};

use crate::{
  client::Client,
  data::live::{cmds::*, *},
};

#[allow(dead_code)]
impl<'a> Live<'a> {
  get_query_json_resp_fn!(
    // UID to real room id
    pub uid_to_room_id(qr_req: &UidToRoomIdReq) [url: UID_TO_ROOM_ID.clone()] -> UidToRoomIdResp;
    pub init_room(qr_req: &InitReq) [url: ROOM_INIT.clone()] -> InitResp;
    pub danmaku_info(qr_req: &DanmakuReq) [url: LIVE_DANMAKU.clone()] -> DanmakuResp;
  );
}

#[derive(Debug)]
pub struct MessageConnection {
  heartbeat_job: Option<JoinHandle<()>>,
  main_job: Option<JoinHandle<()>>,
  rx: Receiver<MaybeCommand>,
  close: bool,
}

impl MessageConnection {
  pub async fn connect_with_client(
    client: &Client,
    room_id: u64,
  ) -> anyhow::Result<Arc<RwLock<Self>>> {
    let mid = {
      let client = Client::clone(client);
      async move {
        client
          .info()
          .get_nav_info()
          .await?
          .data
          .mid
          .context("NavInfo $.data.mid is None")
      }
    };
    let real_room_id = {
      let client = Client::clone(client);
      async move {
        client
          .live()
          .init_room(&room_id.into())
          .await
          .context("Failed to init room")?
          .data
          .context("InitRoomResp $.data is None")?
          .room_id
          .context("InitRoomResp $.data.room_id is None")
      }
    };

    let (mid, real_room_id) = tokio::join!(mid, real_room_id);
    let mid = mid?;
    let real_room_id = real_room_id?;

    let danmaku = client
      .live()
      .danmaku_info(&room_id.into())
      .await
      .context("Failed to get DanmakuResp")?;
    let danmaku_data = danmaku.data.context("DanmakuResp $.data is None")?;
    let host_data = danmaku_data
      .host_list
      .get(0)
      .context("DanmakuResp $.data.host_list is empty")?;
    let key = danmaku_data.token.clone();
    let url = host_data
      .to_url()
      .with_context(|| format!("Failed to convert WssHost to Url: {:?}", host_data))?;

    Self::connect(url, mid, real_room_id, key).await
  }

  pub async fn connect(
    url: Url,
    mid: u64,
    room_id: u64,
    key: String,
  ) -> anyhow::Result<Arc<RwLock<Self>>> {
    let config = NetworkConfig::default();

    let (ws, _) = connect_async(url.clone())
      .await
      .with_context(|| format!("Failed to connect WebSocket: {:?}", url))?;

    let (tx, rx) = mpsc::channel::<MaybeCommand>(config.channel_buffer);

    let con = MessageConnection {
      heartbeat_job: None,
      main_job: None,
      rx,
      close: false,
    };
    let con = Arc::new(RwLock::new(con));

    let (mut wss_tx, mut wss_rx) = ws.split();

    {
      let msg = Certificate::new(mid, room_id, key).with_head(1);
      log::debug!("Send Certificate Packet: {:?}", msg);
      let binary = msg.into_binary_frame().unwrap();
      wss_tx.send(binary).await.unwrap();
    }

    let heartbeat_job = tokio::spawn({
      let con = Arc::clone(&con);
      async move {
        let mut close = false;
        while !close {
          close = con.read().await.close;
          let msg = Message::heartbeat(1);
          log::debug!("Send Heartbeat Packet: {:?}", msg);
          let binary = msg
            .into_binary_frame()
            .context("Heartbeat Packet into binary frame failed")
            .unwrap();

          if let Err(err) = wss_tx.send(binary).await {
            log::error!("{:#?}", err);
            con.write().await.close();
            break;
          };

          tokio::time::sleep(config.heartbeat_interval).await;
        }
      }
    });
    con.write().await.heartbeat_job = Some(heartbeat_job);

    let main_job = tokio::spawn({
      let con = Arc::clone(&con);
      async move {
        while let Some(msg) = wss_rx.next().await {
          use ws2::error::ProtocolError::*;
          use ws2::Error::*;
          use MessagePayload::{self as Payload, *};

          let msg = match msg {
            Ok(ok) => ok,
            Err(err) => match err {
              ConnectionClosed | Protocol(ResetWithoutClosingHandshake) => {
                log::debug!("Remote closed: {}", &url);
                con.write().await.close();
                break;
              }
              err => panic!("{err:#?}"),
            },
          };
          let ws2::Message::Binary(binary) = msg else { continue };
          let mut cursor = Cursor::new(binary);
          let payload = Payload::from_reader(&mut cursor).unwrap();
          match payload {
            ref payload @ HeartbeatResp { .. } | ref payload @ CertificateResp(_) => {
              log::debug!("{payload:?}");
            }
            Command(cmds) => {
              for cmd in cmds {
                if let Err(err) = tx
                  .send(cmd)
                  .await
                  .context("Failed to send Command to channel")
                {
                  log::error!("{:#?}", err);
                  con.write().await.close();
                  break;
                }
              }
            }
            _ => unreachable!(),
          };
        }
      }
    });
    con.write().await.main_job = Some(main_job);

    Ok(con)
  }

  pub fn receiver(&mut self) -> &mut Receiver<MaybeCommand> {
    &mut self.rx
  }

  fn close(&mut self) {
    if self.close {
      return;
    }
    if let Some(ref job) = self.heartbeat_job {
      job.abort();
      self.heartbeat_job = None;
    }
    if let Some(ref job) = self.main_job {
      job.abort();
      self.main_job = None;
    }
    self.close = true;
  }
}

impl Drop for MessageConnection {
  #[inline]
  fn drop(&mut self) {
    self.close();
  }
}

#[derive(Debug)]
pub struct NetworkConfig {
  /// The interval of sending heartbeat packet, the default is 30 seconds.
  pub heartbeat_interval: Duration,
  /// The size of the mpsc channel. Backpressure is controlled by this option.
  /// When value is `None`, unbounded mpsc will be created. Otherwise,
  /// it's the buffer size of mpsc.
  ///
  /// Default is `None`, i.e. unbounded channel.
  pub channel_buffer: usize,
  pub websocket_config: WebSocketConfig,
}

impl Default for NetworkConfig {
  fn default() -> Self {
    Self {
      heartbeat_interval: Duration::from_secs(30),
      channel_buffer: 64,
      websocket_config: Default::default(),
    }
  }
}
