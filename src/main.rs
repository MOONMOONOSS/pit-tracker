#![feature(drain_filter)]

use serde::{Serialize, Deserialize};
use serenity::{
  model::{
    channel::GuildChannel,
    guild::Role,
    id::{
      RoleId,
      UserId
    },
    user::User,
  },
  prelude::*,
};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

mod handler;

#[derive(Serialize, Deserialize)]
pub struct BotConfig {
  pub punishment_role: RoleId,
  pub settle_time: u16,
  pub(crate) token: String,
  pub warn_channel: GuildChannel,
  pub warn_role: Role,
  pub warn_threshold: u16,
}

pub struct BotState {
  pub(self) bans: u64,
  pub(self) pits: u64,
  pub(self) uptime: SystemTime,
  pub(self) users: Vec<PunishedUser>,
}

impl BotState {
  pub fn new() -> Arc<Mutex<Self>> {
    Arc::new(Mutex::new(Self {
      bans: 0,
      pits: 0,
      uptime: SystemTime::now(),
      users: vec![],
    }))
  }
}

pub struct PunishedUser {
  pub(self) id: UserId,
  pub(self) user: User,
  pub times_punished: u16,
  pub last_punish: SystemTime,
}

impl PartialEq<serenity::model::prelude::User> for &mut PunishedUser {
  fn eq(&self, other: &serenity::model::prelude::User) -> bool {
    self.id == other.id
  }
}

fn read_config() -> Result<BotConfig, Box<dyn Error>> {
  use std::fs::File;
  use std::io::BufReader;

  let file = File::open("./config.yaml")?;
  let reader = BufReader::new(file);

  Ok(serde_yaml::from_reader(reader)?)
}

#[tokio::main]
async fn main() {
  let config = read_config().unwrap();
  let state = BotState::new();

  let mut client = Client::new(&config.token)
    .event_handler(
      handler::BotHandler::new(
        Arc::clone(&state),
        config
      )
    )
    .await
    .expect("Error creating Discord client");
  
  if let Err(why) = client.start().await {
    println!("Serenity error: {:?}", why);
  }
}
