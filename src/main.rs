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
use std::sync::Arc;
use std::time::{Duration, SystemTime};

mod handler;
mod state;

use crate::state::BotState;

#[derive(Serialize, Deserialize, Clone)]
pub struct BotConfig {
  pub punishment_role: RoleId,
  pub settle_time: u16,
  pub(crate) token: String,
  pub warn_channel: ChannelId,
  pub warn_role: RoleId,
  pub warn_threshold: u16,
}

impl BotConfig {
  pub fn read_config() -> Result<Self, Box<dyn Error>> {
    use std::io::BufReader;

    let file = File::open("./config.yaml")?;
    let reader = BufReader::new(file);

    Ok(serde_yaml::from_reader(reader)?)
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

#[tokio::main]
async fn main() {
  let config = Arc::new(BotConfig::read_config().unwrap());
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
