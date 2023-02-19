pub mod cmds;

use std::{
  io::{Cursor, Read, Write},
  str::FromStr,
};

use cmds::*;

use anyhow::{bail, Context};
use byteorder::BigEndian as BE;
use byteorder::{ReadBytesExt, WriteBytesExt};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;
use serde_with::{serde_as, BoolFromInt, DefaultOnNull};
use tokio_tungstenite as tokio_ws2;
use tokio_ws2::tungstenite as ws2;

use super::{macros::*, *};

#[derive(Serialize, Debug)]
pub struct UidToRoomIdReq {
  pub uid: u64,
}

impl From<u64> for UidToRoomIdReq {
  fn from(uid: u64) -> Self {
    Self { uid }
  }
}

#[derive(Deserialize, Debug)]
pub struct UidToRoomIdResp {
  pub code: Option<i32>,
  pub message: Option<String>,
  pub data: Option<RoomId>,
}

#[derive(Deserialize, Debug)]
pub struct RoomId {
  pub room_id: u64,
}

#[derive(Serialize, Debug)]
pub struct InitReq {
  #[serde(rename = "id")]
  pub room_id: u64,
}

impl From<u64> for InitReq {
  fn from(room_id: u64) -> Self {
    Self { room_id }
  }
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct InitResp {
  pub code: Option<i32>,
  pub message: Option<String>,
  pub data: Option<InitData>,
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct InitData {
  pub room_id: Option<u64>,
  pub short_id: Option<u64>,
  pub uid: Option<u64>,
  #[serde_as(as = "Option<BoolFromInt>")]
  pub need_p2p: Option<bool>,
  pub is_hidden: Option<bool>,
  pub is_locked: Option<bool>,
  pub is_portrait: Option<bool>,
  pub live_status: Option<LiveStatus>,
  pub hidden_till: Option<i64>,
  pub lock_till: Option<i64>,
  pub encrypted: Option<bool>,
  pub pwd_verified: Option<bool>,
  pub live_time: Option<i64>,
  #[serde(rename = "is_sp")]
  #[serde_as(as = "Option<BoolFromInt>")]
  pub is_special: Option<bool>,
  pub special_type: Option<SpecialType>,
}

#[derive(Debug)]
pub enum SpecialType {
  Normal,
  Paid,
  NewYear,
  Unknown(i32),
}

de_from_code_impl!(SpecialType);

impl FromCode for SpecialType {
  fn from_code(code: i32) -> Self {
    use SpecialType::*;
    match code {
      0 => Normal,
      1 => Paid,
      2 => NewYear,
      unk => Unknown(unk),
    }
  }
}

#[derive(Debug, Deserialize_repr)]
#[repr(u8)]
pub enum LiveStatus {
  Stop = 0,      // 暂停
  Live = 1,      // 直播
  Carousels = 2, // 轮播
}

#[derive(Serialize, Debug)]
pub struct DanmakuReq {
  #[serde(rename = "id")]
  pub room_id: u64, // real room id
}

impl From<u64> for DanmakuReq {
  fn from(room_id: u64) -> Self {
    Self { room_id }
  }
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct DanmakuResp {
  pub code: Option<i32>,
  pub message: Option<String>,
  pub ttl: Option<i32>,
  pub data: Option<WssDanmaku>,
}

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct WssDanmaku {
  pub group: String,
  pub refresh_row_factor: f32,
  pub refresh_rate: f32,
  pub max_delay: u32,
  pub token: String,
  #[serde_as(deserialize_as = "DefaultOnNull")]
  pub host_list: Vec<WssHost>,
}

#[derive(Deserialize, Debug)]
pub struct WssHost {
  pub host: String,
  pub port: u16,
  pub wss_port: u16,
  pub ws_port: u16,
}

#[allow(dead_code)]
impl WssHost {
  pub fn to_url(&self) -> Result<reqwest::Url, url::ParseError> {
    const SCHEMA: &str = "wss://";
    const PATH: &str = "/sub";
    const SEP: &str = ":";
    let port = self.wss_port.to_string();
    let mut host =
      String::with_capacity(SCHEMA.len() + self.host.len() + PATH.len() + SEP.len() + port.len());
    host.push_str(SCHEMA);
    host.push_str(self.host.as_str());
    host.push_str(SEP);
    host.push_str(port.as_str());
    host.push_str(PATH);
    reqwest::Url::from_str(host.as_str())
  }
}

#[derive(Debug)]
pub struct MessageHead {
  pub size: u32,
  pub head_size: u16,
  pub protocol: PacketProtocol, // u16
  pub pkt_type: PacketType,     // u32
  pub sequence: u32,
}

#[allow(dead_code)]
impl MessageHead {
  const SIZE: usize = 16;

  pub fn certificate(sequence: u32) -> MessageHead {
    MessageHead {
      size: 0,
      sequence,
      head_size: Self::SIZE as u16,
      protocol: PacketProtocol::Special,
      pkt_type: PacketType::Certificate,
    }
  }

  pub fn heartbeat(sequence: u32) -> MessageHead {
    MessageHead {
      size: 0,
      sequence,
      head_size: Self::SIZE as u16,
      protocol: PacketProtocol::Special,
      pkt_type: PacketType::Heartbeat,
    }
  }

  pub fn from_reader<R: Read>(reader: &mut R) -> std::result::Result<MessageHead, HeadReadError> {
    use HeadReadError::*;
    let head = MessageHead {
      size: reader.read_u32::<BE>()?,
      head_size: reader.read_u16::<BE>()?,
      protocol: {
        let num = reader.read_u16::<BE>()?;
        PacketProtocol::from_u16(num).ok_or_else(|| InvalidProtocol(num))?
      },
      pkt_type: {
        let num = reader.read_u32::<BE>()?;
        PacketType::from_u32(num).ok_or_else(|| InvalidType(num))?
      },
      sequence: reader.read_u32::<BE>()?,
    };
    std::result::Result::Ok(head)
  }

  pub fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
    writer.write_u32::<BE>(self.size)?;
    writer.write_u16::<BE>(self.head_size)?;
    writer.write_u16::<BE>(self.protocol.clone() as u16)?;
    writer.write_u32::<BE>(self.pkt_type.clone() as u32)?;
    writer.write_u32::<BE>(self.sequence)?;
    std::result::Result::Ok(())
  }
}

#[derive(Debug)]
pub struct Message {
  pub head: MessageHead,
  pub payload: MessagePayload,
}

impl Message {
  pub fn into_binary_frame(mut self) -> anyhow::Result<ws2::Message> {
    let mut buf: Vec<u8> = Vec::with_capacity(128);
    self
      .write_to(&mut buf)
      .context("Failed to into_binary_frame")?;
    Ok(ws2::Message::Binary(buf))
  }

  pub fn heartbeat(sequence: u32) -> Message {
    Message {
      head: MessageHead::heartbeat(sequence),
      payload: MessagePayload::Heartbeat,
    }
  }

  pub fn write_to<W: Write>(&mut self, writer: &mut W) -> anyhow::Result<()> {
    use MessagePayload::*;
    let mut buf: Vec<u8> = Vec::with_capacity(128);
    match self.payload {
      Heartbeat => {
        buf
          .write(b"[object Object]")
          .context("Failed to write heartbeat body")?;
      }
      Certificate(ref body) => {
        serde_json::to_writer(&mut buf, body).context("Failed to serialize Certificate body")?;
      }
      _ => {
        bail!(
          "`{}` packet is not for sending, cannot be serialized",
          self.payload.to_string(),
        )
      }
    };
    let payload_size = buf.len() as u32;
    self.head.size = payload_size + self.head.head_size as u32;

    self.head.write_to(writer).context("Failed to write head")?;
    writer
      .write_all(buf.as_slice())
      .context("Failed to write payload buf")?;

    Ok(())
  }
}

#[repr(u8)]
#[derive(strum_macros::Display, Debug)]
pub enum MessagePayload {
  Heartbeat,
  HeartbeatResp { popular: u32 },
  Certificate(Certificate),
  CertificateResp(CertificateResp),
  Command(Vec<MaybeCommand>),
}

impl MessagePayload {
  pub fn from_reader<R: Read>(reader: &mut R) -> anyhow::Result<MessagePayload> {
    let head = MessageHead::from_reader(reader).context("Failed to read MessageHead")?;
    use PacketProtocol::{CommandBrotli, CommandZlib, Special};
    let payload: MessagePayload = match head.pkt_type {
      PacketType::Command => MessagePayload::Command(match head.protocol {
        PacketProtocol::Command => {
          vec![serde_json::from_reader(reader).context("Failed to deserialize Command")?]
        }
        CommandZlib => {
          let mut cmds = Vec::with_capacity(16);
          let mut head_buf = [0u8; MessageHead::SIZE];
          let mut zlib_rdr = flate2::read::ZlibDecoder::new(reader);

          while zlib_rdr
            .read(&mut head_buf)
            .context("Failed to read head")?
            == 16
          {
            let mut head_cursor = Cursor::new(head_buf);
            let head =
              MessageHead::from_reader(&mut head_cursor).context("Failed to read MessageHead")?;
            let mut buf: Vec<u8> = vec![0; (head.size - head.head_size as u32) as usize];
            zlib_rdr
              .read_exact(&mut buf)
              .context("Failed to read body")?;
            let cmd: MaybeCommand =
              serde_json::from_slice(&buf).context("Failed to read Json with Zlib")?;
            cmds.push(cmd);
          }
          cmds
        }
        CommandBrotli => todo!("CommandBrotli is not implemented"),
        _ => bail!("Unexpected protocol: {:?}", head.protocol),
      }),
      PacketType::HeartbeatResp if head.protocol == Special => MessagePayload::HeartbeatResp {
        popular: reader
          .read_u32::<BE>()
          .context("Failed to read HeartbeatResp")?,
      },
      PacketType::CertificateResp if head.protocol == Special => MessagePayload::CertificateResp(
        serde_json::from_reader(reader).context("Failed to deserialize CertificateResp")?,
      ),
      _ => bail!("Unsupported packet, header: {:#?}", &head),
    };

    Ok(payload)
  }
}

#[derive(Serialize)]
pub struct Certificate {
  #[serde(rename = "uid")]
  pub mid: u64,
  #[serde(rename = "roomid")]
  pub room_id: u64,
  pub key: String,
}

impl std::fmt::Debug for Certificate {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Certificate")
      .field("mid", &self.mid)
      .field("room_id", &self.room_id)
      .field(
        "key",
        if !self.key.is_empty() {
          &"**********"
        } else {
          &""
        },
      )
      .finish()
  }
}

#[allow(dead_code)]
impl Certificate {
  #[inline]
  pub fn new(mid: u64, room_id: u64, key: String) -> Certificate {
    Certificate { mid, room_id, key }
  }

  #[inline]
  pub fn with_head(self, sequence: u32) -> Message {
    Message {
      head: MessageHead::certificate(sequence),
      payload: MessagePayload::Certificate(self),
    }
  }
}

#[derive(Deserialize, Debug)]
pub struct CertificateResp {
  pub code: i32,
}

#[allow(dead_code)]
impl CertificateResp {
  #[inline]
  fn is_success(&self) -> bool {
    self.code == 0
  }
}

#[derive(thiserror::Error, Debug)]
pub enum HeadReadError {
  #[error("Io Error, failed to read: `{0:#?}`")]
  Io(#[from] std::io::Error),
  #[error("invalid protocol `{0}`")]
  InvalidProtocol(u16),
  #[error("invalid type: `{0}`")]
  InvalidType(u32),
}

#[repr(u16)]
#[derive(FromPrimitive, Clone, Debug, PartialEq, Eq)]
pub enum PacketProtocol {
  Command = 0,
  Special = 1,
  CommandZlib = 2,
  CommandBrotli = 3,
}

#[repr(u32)]
#[derive(FromPrimitive, Clone, Debug)]
pub enum PacketType {
  Heartbeat = 2,
  HeartbeatResp = 3,
  Command = 5,
  Certificate = 7,
  CertificateResp = 8,
}
