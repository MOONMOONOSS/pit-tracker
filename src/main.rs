#![feature(drain_filter)]

use clokwerk::{Scheduler, TimeUnits};

use serde::{Serialize, Deserialize};
use serenity::{
  model::{
    id::{
      ChannelId,
      RoleId,
      UserId
    },
  },
  prelude::*,
framework::StandardFramework};

use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

mod handler;

#[derive(Serialize, Deserialize, Clone)]
pub struct BotConfig {
  pub punishment_role: RoleId,
  pub settle_time: u16,
  pub(crate) token: String,
  pub warn_channel: ChannelId,
  pub warn_role: RoleId,
  pub warn_threshold: u16,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct BotState {
  pub(self) bans: u64,
  pub(self) pits: u64,
  #[serde(skip)]
  pub(self) uptime: Option<SystemTime>,
  #[serde(skip)]
  pub(self) users: Vec<PunishedUser>,
}

impl BotState {
  pub fn new() -> Arc<Mutex<Self>> {
    use std::io::BufReader;

    let mut users: Option<Vec<PunishedUser>> = None;
    let mut main: Option<Self> = None;

    if Path::new("./punished_users.yaml").is_file() {
      let file = File::open("./punished_users.yaml").unwrap();
      let reader = BufReader::new(file);

      users = Some(serde_yaml::from_reader(reader).unwrap());
    } else {
      return Arc::new(Mutex::new(Self {
        bans: 0,
        pits: 0,
        uptime: Some(SystemTime::now()),
        users: vec![],
      }))
    }

    if Path::new("./bot_stats.yaml").is_file() {
      let file = File::open("./bot_stats.yaml").unwrap();
      let reader = BufReader::new(file);

      main = Some(serde_yaml::from_reader(reader).unwrap());
    }

    let unwrapped_main = main.unwrap_or_default();

    Arc::new(Mutex::new(Self {
      bans: unwrapped_main.bans,
      pits: unwrapped_main.pits,
      uptime: Some(SystemTime::now()),
      users: users.unwrap_or_default(),
    }))
  }

  pub fn periodic_strike_removal(&mut self, config: &BotConfig) {
    let settle_duration = Duration::from_secs((config.settle_time * 60 * 60 * 24) as u64);
    let mut punishments_forgiven: u64 = 0;
    let mut clean_record: u64 = 0;

    self.users.drain_filter(|user| {
      if user.last_punish.elapsed().expect("Jebaited by Daylight Savings") >= settle_duration {
        user.last_punish = SystemTime::now();
        user.times_punished -= 1;
        punishments_forgiven += 1;
      }

      if user.times_punished == 0 {
        clean_record += 1;

        true
      } else {
        false
      }
    });

    println!(
r#"Periodic Strike Removal Report
Punishments Forgiven: {}
New users with no strikes: {}"#,
      punishments_forgiven,
      clean_record,
    )
  }

  pub fn flatdb_save(&self) -> Result<(), Box<dyn Error>> {
    if Path::new("./punished_users.yaml").is_file() {
      use std::fs;

      fs::copy("./punished_users.yaml", "./punished_users.yaml.backup")?;
    }

    if Path::new("./bot_stats.yaml").is_file() {
      use std::fs;

      fs::copy("./bot_stats.yaml", "./bot_stats.yaml.backup")?;
    }

    serde_yaml::to_writer(&File::create("./punished_users.yaml")?, &self.users)?;
    serde_yaml::to_writer(&File::create("./bot_stats.yaml")?, &self)?;

    Ok(())
  }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PunishedUser {
  pub(self) id: UserId,
  pub times_punished: u16,
  pub last_punish: SystemTime,
}

impl PartialEq<serenity::model::prelude::User> for &mut PunishedUser {
  fn eq(&self, other: &serenity::model::prelude::User) -> bool {
    self.id == other.id
  }
}

fn read_config() -> Result<BotConfig, Box<dyn Error>> {
  use std::io::BufReader;

  let file = File::open("./config.yaml")?;
  let reader = BufReader::new(file);

  Ok(serde_yaml::from_reader(reader)?)
}

#[tokio::main]
async fn main() {
  let config = Arc::new(read_config().unwrap());
  let state = BotState::new();

  let framework = StandardFramework::new()
    .configure(|c| c
      .with_whitespace(true)
      .prefix("!")
    );

  let mut client = Client::new(&config.token)
    .event_handler(
      handler::BotHandler::new(
        Arc::clone(&state),
        Arc::clone(&config),
      )
    )
    .framework(framework)
    .await
    .expect("Error creating Discord client");
  
  let mut scheduler = Scheduler::new();

  {
    let sch_state = Arc::clone(&state);
    let cloned_config = Arc::clone(&config);
    scheduler.every(1.day()).run(move || {
      let mut state = sch_state.lock().expect("Unable to read from state");

      state.periodic_strike_removal(&cloned_config);
    });
  }

  {
    let sch_state = Arc::clone(&state);
    scheduler.every(10.minutes()).run(move || {
      let state = sch_state.lock().expect("Unable to read from state");

      state.flatdb_save().unwrap();
    });
  }

  let thread_handle = scheduler.watch_thread(Duration::from_millis(100));
  
  if let Err(why) = client.start().await {
    println!("Serenity error: {:?}", why);

    thread_handle.stop();

    state
      .lock()
      .expect("Unable to read from state")
      .flatdb_save()
      .unwrap();
  }
}
