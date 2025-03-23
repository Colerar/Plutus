use std::{
  fs,
  hash::BuildHasherDefault,
  io::{stdin, BufRead, BufReader, Read},
  num::NonZeroU64,
  process::exit,
  str::FromStr,
  sync::Arc,
  time::Duration,
};

use anyhow::{anyhow, bail, Context};
use chrono::{DateTime, Local, NaiveDateTime, Utc};
use clap::{Parser, Subcommand};
use dashmap::DashMap;
use diesel_async::RunQueryDsl;
use futures_util::StreamExt;
use plutus_core::{
  api::live::MessageConnection,
  client::Client,
  data::{
    live::cmds::{Command, GuardLevel, MaybeCommand},
    passport::{QrLoginData, QrLoginStatus},
  },
};
use serde::Deserialize;
use terminal_link::Link;
use tokio::join;

use crate::{
  data::passport::QrLoginQuery,
  error::AnyhowExt,
  models::{Log, NewLog},
  resp::{Cursor, Paginated, Resp},
  routes::{server, QueryBody, TimeRange},
  state::{AsyncPoolConnection, State},
};
use plutus_core::*;

mod config;
mod error;
mod models;
mod resp;
mod routes;
mod state;

#[rustfmt::skip]
mod schema;

pub type ADashMap<K, V> = DashMap<K, V, BuildHasherDefault<ahash::AHasher>>;

#[derive(Parser, Debug)]
struct Args {
  #[command(subcommand)]
  command: Action,
}

#[derive(Subcommand, Debug)]
enum Action {
  /// Starts the server
  Server,
  /// View saved comments
  Query(QueryCommand),
}

#[derive(Parser, Debug)]
struct QueryCommand {
  /// Should be real room id
  #[arg(short, long)]
  pub room: u64,
  /// Filter specific UID
  #[arg(short, long)]
  pub uid: Option<u64>,
  /// Commands, e.g. "DANMU_MSG", "SUPER_CHAT_MESSAGE", "GUARD_BUY"
  #[clap(short, long, value_delimiter = ' ', num_args = 1..)]
  pub commands: Vec<String>,

  /// Prints raw JSON
  #[clap(long)]
  pub raw: bool,

  /// Only view results after a certain date, parse using ISO 8601 yyyy-mm-ddThh:mm:ss
  #[clap(long, value_parser = parse_date)]
  pub start: Option<DateTime<Utc>>,
  /// Only view results before a certain date
  #[clap(long, value_parser = parse_date)]
  pub end: Option<DateTime<Utc>>,

  #[clap(short, long, default_value = "1")]
  pub page: NonZeroU64,
  #[clap(long, default_value = "500")]
  pub size: NonZeroU64,

  #[clap(short, long, default_value = "http://127.0.0.1:7727")]
  pub server: String,
}

/// assume input is local date time, and convert it to UTC
fn parse_date(arg: &str) -> anyhow::Result<DateTime<Utc>> {
  let local_tz = *Local::now().offset();
  Ok(
    NaiveDateTime::from_str(arg)
      .context("Failed to parse as ISO 8601 yyyy-mm-ddThh:mm:ss")?
      .and_local_timezone(local_tz)
      .single()
      .context("Failed to convert timezone")?
      .with_timezone(&Utc),
  )
}

fn main() -> anyhow::Result<()> {
  tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .context("Failed to build tokio runtime")?
    .block_on(async { async_main().await })
}

pub static mut GLOBAL_STATE: Option<State> = None;

pub fn global_state() -> &'static State {
  #[allow(static_mut_refs)]
  unsafe { GLOBAL_STATE.as_ref().unwrap() }
}

pub static mut STATS_MAP: Option<Arc<ADashMap<String, u64>>> = None;

pub fn stats_map() -> Arc<ADashMap<String, u64>> {
  #[allow(static_mut_refs)]
  unsafe { STATS_MAP.clone().unwrap() }
}

pub const PLUTUS_VERSION: &str = env!("CARGO_PKG_VERSION");

#[inline(always)]
async fn async_main() -> anyhow::Result<()> {
  let args: Args = Args::parse();
  init_logger().context("Failed to init logger")?;

  match args.command {
    Action::Server => {
      main_server().await?;
    },
    Action::Query(action) => {
      query(action).await?;
    },
  }
  Ok(())
}

async fn main_server() -> anyhow::Result<()> {
  log::info!("Plutus v{}", PLUTUS_VERSION);

  let state: State = State::init().await.context("Failed to init plutus")?;

  unsafe {
    GLOBAL_STATE = Some(state.clone());
    STATS_MAP = Some(Arc::new(ADashMap::default()));
  }

  let client = Client::new()?;
  login_if_not(&client).await?;

  let address = state.config.address;
  let (collector, stats_printer, server) = join!(
    tokio::spawn(async move {
      collector(&client, &state.config.rooms)
        .await
        .context("collector error")
        .log()
    }),
    tokio::spawn(async move { stats_printer().await }),
    tokio::spawn(async move {
      if server(&address).await.also_log().is_err() {
        exit(1);
      }
    })
  );
  server?;
  collector?;
  stats_printer?;

  Ok(())
}

fn init_logger() -> anyhow::Result<()> {
  pretty_env_logger::formatted_timed_builder()
    .filter(None, log::LevelFilter::Debug)
    .filter(Some("tokio_postgres"), log::LevelFilter::Info)
    .filter(Some("rustls"), log::LevelFilter::Info)
    .filter(Some("tokio_tungstenite"), log::LevelFilter::Info)
    .filter(Some("h2"), log::LevelFilter::Info)
    .filter(Some("hyper"), log::LevelFilter::Info)
    .filter(Some("reqwest::connect"), log::LevelFilter::Info)
    .parse_env("PLUTUS_LOG")
    .init();

  Ok(())
}

async fn login_if_not(client: &Client) -> anyhow::Result<()> {
  if client.info().get_nav_info().await?.data.mid.is_some() {
    return Ok(());
  };
  let login_qr = client.passport().get_login_qr().await?;
  let qr_data = login_qr.data.context("No login qr data")?;
  qr2term::print_qr(&qr_data.url).context("Failed to generate QR code")?;
  log::warn!("Enter for continue");
  let mut buf = String::new();
  stdin().lock().read_line(&mut buf)?;
  let resp = client
    .passport()
    .login_qr(&QrLoginQuery {
      qrcode_key: &qr_data.qrcode_key,
    })
    .await?;

  match resp.data {
    Some(QrLoginData {
      code: QrLoginStatus::Ok,
      ..
    }) => {
      log::info!("Login successfully!");
      client.save_cookies();
    },
    Some(QrLoginData { code, .. }) => {
      bail!(
        "Failed to login, code: {raw_code} ({code:?})",
        raw_code = code as i32
      )
    },
    None => {
      bail!("Failed to login, empty resp data")
    },
  }
  Ok(())
}

async fn collector(client: &Client, rooms: &[u64]) -> anyhow::Result<()> {
  for room_id in rooms.iter() {
    let room_id = *room_id;
    let client = client.clone();
    tokio::spawn(async move {
      loop {
        log::info!("Connecting to {room_id}");
        let con =
          match MessageConnection::<serde_json::Value>::connect_with_client(&client, room_id).await
          {
            Ok(con) => con,
            Err(err) => {
              log::error!("connect to {room_id} failed, sleep 10s before retrying: {err:?}");
              tokio::time::sleep(Duration::from_secs(10)).await;
              continue;
            },
          };
        while let Some(raw_json) = { con.write().await.next().await } {
          tokio::spawn(async move {
            let Some(cmd_id) = raw_json.get("cmd").and_then(|cmd| cmd.as_str()) else {
              log::warn!(
                "Unknown command, room_id={room_id}, raw_json={}",
                serde_json::to_string(&raw_json)
                  .unwrap_or_else(|err| format!("Failed to deser {err:?}"))
              );
              return;
            };

            let mut related_uid: Option<i64> = None;

            let cmd = serde_json::from_value::<MaybeCommand>(raw_json.clone());
            if let Ok(MaybeCommand::Command(cmd)) = cmd {
              related_uid = match cmd {
                Command::Danmaku { data } => data.data().ok().map(|data| data.user.uid as i64),
                Command::SuperChatMessage { data } => Some(data.uid as i64),
                Command::GuardBuy { data } => Some(data.uid as i64),
                Command::InteractWord { data } => Some(data.uid as i64),
                Command::EntryEffect { data } => Some(data.uid as i64),
                Command::LikeInfoV3Click { data } => Some(data.uid as i64),
                _ => None,
              };
            };

            let mut conn: AsyncPoolConnection = match global_state().db_con().await {
              Ok(ok) => ok,
              Err(err) => {
                log::error!("Failed get db conn: {err:?}");
                return;
              },
            };

            let new_log = NewLog {
              room_id: room_id as i64,
              command: cmd_id.to_string(),
              raw_json,
              related_uid,
              time: chrono::Utc::now(),
            };
            let result = diesel::insert_into(crate::schema::logs::table)
              .values(&new_log)
              .execute(&mut conn)
              .await;
            if let Err(err) = result {
              log::error!("Failed to insert, {new_log:?}, err: {err:?}")
            } else {
              let map = stats_map();
              let mut count = map.entry(new_log.command).or_insert(0);
              *count.value_mut() += 1;
            }
          });
        }
        log::error!("Room {room_id} conn closed, sleep 10s before reconnecting");
        tokio::time::sleep(Duration::from_secs(10)).await;
      }
    });
  }
  Ok(())
}

async fn stats_printer() {
  let dur = Duration::from_secs(60);
  tokio::time::sleep(dur).await;
  let mut timer = tokio::time::interval(dur);
  loop {
    timer.tick().await;
    let mut maps = {
      let map = stats_map();
      let maps: Vec<_> = map.iter().map(|r| (r.key().clone(), *r.value())).collect();
      map.clear();
      maps
    };
    // descending
    maps.sort_unstable_by(|(_k1, v1), (_k2, v2)| v2.cmp(v1));
    let total: u64 = maps.iter().map(|(_k, v)| *v).sum();
    let top5 = maps
      .into_iter()
      .take(5)
      .map(|(k, v)| format!("{k}={v}"))
      .collect::<Vec<_>>()
      .join(", ");
    log::info!(
      "A total of {total} commands were collected in the past minute, and the top five are: {top5}",
    );
  }
}

fn guess_addr_from_config() -> Option<String> {
  #[derive(Deserialize, Debug, Clone)]
  #[serde(rename_all = "kebab-case")]
  pub struct Config {
    #[serde(alias = "addr")]
    pub address: Option<String>,
  }
  let config_path =
    std::env::var("PLUTUS_CONFIG").unwrap_or_else(|_err| "plutus-config.toml".to_string());
  let mut file = fs::File::open(config_path).ok().map(BufReader::new)?;
  let mut buf = Vec::with_capacity(1024);
  file.read_to_end(&mut buf).ok()?;
  let str = std::str::from_utf8(&buf).ok()?;
  let config: Config = toml::from_str(str).ok()?;

  config.address
}

async fn query(query: QueryCommand) -> anyhow::Result<()> {
  let mut host = guess_addr_from_config().unwrap_or(query.server);
  let client = reqwest::Client::new();
  if !host.starts_with("https://") && !host.starts_with("http://") {
    host = format!("http://{host}");
  }

  let resp = client
    .post(format!("{host}/list"))
    .json(&QueryBody {
      room_id: query.room,
      commands: query.commands,
      uid: query.uid,
      time_range: Some(TimeRange {
        start: query.start,
        end: query.end,
      }),
      cursor: Cursor {
        page: query.page,
        size: query.size,
      },
    })
    .send()
    .await
    .context("Failed to get list result")?;
  if !resp.status().is_success() {
    let status = resp.status();
    let text = resp.text().await.context("Failed to parse body as text")?;
    return Err(anyhow!("Failed to get list: {status}, {text}"));
  }
  let resp = resp
    .json::<Resp<Paginated<Log>>>()
    .await
    .context("Failed to deserilaize JSON")?;

  if resp.code.0 != 0 {
    return Err(anyhow!("{resp:?}"));
  }

  let data = resp.data.context("No data")?;
  println!(
    "--- 第 {} 页 / 共 {} 页 ---",
    data.page.current,
    data
      .page
      .max
      .map(|m| m.to_string())
      .unwrap_or_else(|| "Unknown".to_string())
  );
  let mut ignored = 0;
  let mut markdown = String::with_capacity(1024);
  let local_tz = Local::now().timezone();

  for log in data.list {
    if query.raw {
      match serde_json::to_string(&log.raw_json) {
        Ok(json) => {
          println!("{json}",);
        },
        Err(err) => println!("{err}: {}", log.raw_json),
      }

      continue;
    }

    let Some(command) = serde_json::from_value::<Command>(log.raw_json).ok() else {
      ignored += 1;
      continue;
    };
    let ts = log.time.with_timezone(&local_tz).format("%m-%d %H:%M");
    match command {
      Command::CutOff { data } => {
        markdown.push_str(&format!("[{ts}]直播被切断，原因为: {}\n", data.message));
      },
      Command::Danmaku { data } => {
        let data = data.data().context("Unable to parse danmaku info")?;
        let medal = data
          .medal
          .as_ref()
          .map(|medal| {
            let name = &medal.name;
            let level = medal.level;
            let guard_level = format_guard_level(medal.guard_level);
            format!("[{name}-{level}]{guard_level}")
          })
          .unwrap_or_else(String::new);

        let link = format!("https://space.bilibili.com/{}/", data.user.uid);
        let user = Link::new(&data.user.username, &link);
        markdown.push_str(&format!("[{ts}]{user}{medal}: {}\n", data.content));
      },
      Command::GuardBuy { data } => {
        let link = format!("https://space.bilibili.com/{}/", data.uid);
        let user = Link::new(&data.username, &link);
        let level = format_guard_level(data.guard_level);
        markdown.push_str(&format!(
          "[{ts}]{user}购买了{level}，花费{price}\n",
          price = data.price
        ));
      },
      Command::Living { data: _ } => {
        markdown.push_str("[{ts}]开播\n");
      },
      Command::Preparing { data: _ } => {
        markdown.push_str("[{ts}]下播\n");
      },
      Command::RoomSilentOff => {
        markdown.push_str("[{ts}]禁言关闭\n");
      },
      Command::RoomSilentOn { data } => {
        markdown.push_str(&format!("[{ts}]禁言开启:{data:?}\n"));
      },
      Command::SuperChatMessage { data } => {
        let link = format!("https://space.bilibili.com/{}/", data.uid);
        let user = Link::new(&data.user.username, &link);
        let price = data.price;

        markdown.push_str(&format!(
          "[{ts}][SuperChat][{price}]{user}: {}\n",
          data.message.unwrap_or_default()
        ));
      },
      Command::Warning { data } => {
        markdown.push_str(&format!("[{ts}]超管警告: {}\n", data.message));
      },
      _ => {
        ignored += 1;
      },
    }
  }

  if !query.raw && ignored != 0 {
    markdown.push_str(&format!(
      "\n已忽略 {} 条消息，要查看所有原始消息可传入 --raw。\n",
      ignored
    ));
  }

  termimad::print_inline(&markdown);

  Ok(())
}

fn format_guard_level(level: GuardLevel) -> &'static str {
  match level {
    data::live::cmds::GuardLevel::None => "",
    data::live::cmds::GuardLevel::Governor => "[总督]",
    data::live::cmds::GuardLevel::Admiral => "[提督]",
    data::live::cmds::GuardLevel::Captain => "[舰长]",
  }
}
