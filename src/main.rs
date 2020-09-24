#![feature(drain_filter)]

use clokwerk::{Scheduler, TimeUnits};

use serde::{Serialize, Deserialize};
use serenity::{
  model::{
    channel::Message,
    id::{
      ChannelId,
      RoleId,
      UserId
    },
  },
  prelude::*,
  framework::{
    StandardFramework,
    standard::Args,
    standard::CommandResult,
    standard::macros::*,
  },
  utils::Color,
};

use std::{error::Error, io::Write};
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::thread;
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

#[group]
#[commands(removepit, pitcount, mypits, housekeeping, forcesave)]
struct General;

struct State;

impl TypeMapKey for State {
  type Value = Arc<Mutex<BotState>>;
}

struct Config;

impl TypeMapKey for Config {
  type Value = Arc<BotConfig>;
}

#[command]
#[only_in(guilds)]
#[allowed_roles("Moderators", "COSMIC GAMER")]
fn removepit(ctx: &mut Context, msg: &Message, arg: Args) -> CommandResult {
  use serenity::utils::parse_mention;

  if arg.is_empty() {
    return Ok(())
  }

  let target = UserId(
    parse_mention(
      arg.current().unwrap()
    ).unwrap()
  );

  let data = ctx.data.read();
  let mut clone: Option<PunishedUser> = None;
  if let Some(lock) = data.get::<State>() {
    let mut state = lock.lock().expect("Unable to read from state");

    for user in state.users.iter_mut() {
      if user.id == target && user.times_punished != 0 {
        user.times_punished -= 1;
        clone = Some(user.clone());
        
        break;
      }
    }
  }

  if let Some(user) = clone {
    msg.channel_id.send_message(&ctx, |m| {
      m.embed(|e| {
        e.title("Strike Removed");
        e.description(format!(r#"
Strike for <@{}> removed
Active Strikes: `{}`
"#, &user.id, &user.times_punished)
        );
        e.color(Color::ROHRKATZE_BLUE);

        e
      });

      m
    })?;
  } else {
    msg.reply(&ctx, "User has no pits")?;
  }

  Ok(())
}

#[command]
#[only_in(guilds)]
#[allowed_roles("Moderators", "Dev", "COSMIC GAMER")]
fn housekeeping(ctx: &mut Context, msg: &Message, _: Args) -> CommandResult {
  let data = ctx.data.read();
  if let Some(lock) = data.get::<State>() {
    let mut state = lock.lock().expect("Unable to read from state");

    if let Some(config) = data.get::<Config>() {
      state.periodic_strike_removal(&config);
    }
  }

  msg.reply(&ctx, "Completed housekeeping")?;

  Ok(())
}

#[command]
#[only_in(guilds)]
#[allowed_roles("Moderators", "Dev", "COSMIC GAMER")]
async fn forcesave(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
  let data = ctx.data.read().await;
  let mut ok = false;
  if let Some(lock) = data.get::<State>() {
    let state = lock.lock().expect("Unable to read from state");

    ok = state.flatdb_save().is_ok();
  }

  match ok {
    true => msg.reply(&ctx, "Saved").await?,
    false => msg.reply(&ctx, "Error occured in saving file!").await?,
  };

  Ok(())
}

#[command]
#[only_in(guilds)]
#[allowed_roles("Moderators", "COSMIC GAMER")]
async fn pitcount(ctx: &Context, msg: &Message, arg: Args) -> CommandResult {
  use serenity::utils::parse_mention;

  if arg.is_empty() {
    return Ok(())
  }

  let target = UserId(
    parse_mention(
      arg.current().unwrap()
    ).unwrap()
  );

  let data = ctx.data.read().await;
  let mut clone: Option<PunishedUser> = None;
  if let Some(lock) = data.get::<State>() {
    let state = lock.lock().expect("Unable to read from state");

    for user in state.users.iter() {
      if user.id == target {
        clone = Some(user.clone());
        
        break;
      }
    }
  }

  if let Some(user) = clone {
    msg.channel_id.send_message(&ctx, |m| {
      m.embed(|e| {
        e.title(format!("Pit Stats for {}", &user.id));
        e.description(format!(r#"
User: <@{}>
Active Strikes: `{}`
"#, &user.id, &user.times_punished)
        );
        e.color(Color::ROHRKATZE_BLUE);

        e
      });

      m
    })
      .await
      ?;
  } else {
    msg.reply(&ctx, "Record not found").await?;
  }

  Ok(())
}

#[command]
#[only_in(dm)]
async fn mypits(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
  let target = msg.author.id;

  let data = ctx.data.read().await;
  let mut clone: Option<PunishedUser> = None;
  if let Some(lock) = data.get::<State>() {
    let state = lock.lock().expect("Unable to read from state");

    for user in state.users.iter() {
      if user.id == target {
        clone = Some(user.clone());
        
        break;
      }
    }
  }

  if let Some(user) = clone {
    msg.channel_id.send_message(&ctx, |m| {
      m.embed(|e| {
        e.title(format!("Pit Stats for {}", &user.id));
        e.description(format!(r#"
User: <@{}>
Active Strikes: `{}`
"#, &user.id, &user.times_punished)
        );
        e.color(Color::ROHRKATZE_BLUE);

        e
      });

      m
    })
      .await
      ?;
  } else {
    msg.reply(&ctx, "Record not found").await?;
  }

  Ok(())
}

fn main() {
  let config = Arc::new(BotConfig::read_config().unwrap());
  let state = BotState::new();

  let framework = StandardFramework::new()
    .configure(|c| c
      .with_whitespace(true)
      .prefix("!")
    )
    .group(&GENERAL_GROUP);

  let mut client = Client::new(
    &config.token,
    handler::BotHandler::new(
      Arc::clone(&state),
      Arc::clone(&config),
    )
  )
    .expect("Error creating Discord client");
  
  client.with_framework(framework);
  
  {
    let mut data = client.data.write();
    data.insert::<State>(Arc::clone(&state));
    data.insert::<Config>(Arc::clone(&config));
  }

  let manager = client.shard_manager.clone();
  thread::spawn(move || {
    loop {
      thread::sleep(Duration::from_secs(30));

      let lock = manager.lock();
      let shard_runners = lock.runners.lock();

      for (id, runner) in shard_runners.iter() {
        println!(
          "Shard ID {} is {} with a latency of {:?}",
          id,
          runner.stage,
          runner.latency,
        );
      }
    }
  });

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

      match state.flatdb_save() {
        Ok(_) => {},
        Err(_) => println!("Error in Database save"),
      }
    });
  }

  let thread_handle = scheduler.watch_thread(Duration::from_millis(100));
  
  if let Err(why) = client.start_autosharded() {
    println!("Serenity error: {:?}", why);

    thread_handle.stop();

    if let Ok(mut file) = File::create("./crash.log") {
      file.write_all(why.to_string().as_bytes())
        .expect("Unable to write to error log");
    }

    state
      .lock()
      .expect("Unable to read from state")
      .flatdb_save()
      .unwrap();
  }
}
