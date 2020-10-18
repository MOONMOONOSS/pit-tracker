use serde::{Serialize, Deserialize};
use serenity::{http::Http, utils::Color};
use std::{error::Error, fs::File, path::Path};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use crate::{
  PunishedUser,
  BotConfig,
  Reminder,
};

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct BotState {
  pub(crate) bans: u64,
  pub(crate) pits: u64,
  #[serde(skip)]
  pub(self) uptime: Option<SystemTime>,
  #[serde(skip)]
  pub(crate) users: Vec<PunishedUser>,
  #[serde(skip)]
  pub(crate) reminders: Vec<Reminder>,
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
        reminders: vec![],
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
      reminders: vec![],
    }))
  }

  pub fn reminder_check(&mut self, ctx: &Arc<Http>, config: &BotConfig) {
    println!("Reminder check...");

    self.reminders.drain_filter(|reminder| {
      if reminder.creation.elapsed().unwrap().as_secs() >= reminder.deadline.as_secs() {
        let channel = config.warn_channel;

        channel.send_message(&ctx, |m| {
          m.embed(|e| {
            e.title("Reminder");
            e.description(&reminder.message);
            e.color(Color::BLURPLE);
            e.author(|a| {
              if let Ok(author) = ctx.get_user(reminder.author) {
                a.icon_url(author.avatar_url().unwrap_or_default());
                a.name(author.name);
              }

              a
            });

            e
          });

          m
        }).unwrap();
        
        true
      } else {
        false
      }
    });
  }

  pub fn add_reminder(&mut self, reminder: Reminder) {
    self.reminders.push(reminder);
  }

  pub fn periodic_strike_removal(&mut self, config: &BotConfig) {
    let settle_duration = Duration::from_secs((config.settle_time as u64 * 60 * 60 * 24) as u64);
    let total_records = self.users.len();
    let mut punishments_forgiven: u64 = 0;
    let mut clean_record: u64 = 0;

    self.users.drain_filter(|user| {
      if let Ok(last_punish) = user.last_punish.elapsed() {
        println!("Last Punish: {}, Settle Time: {}", last_punish.as_secs(), settle_duration.as_secs());

        if last_punish.as_secs() >= settle_duration.as_secs() {
          user.last_punish = SystemTime::now();
          user.times_punished -= 1;
          punishments_forgiven += 1;
        }
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
Total Tracked Users: {}
Punishments Forgiven: {}
New users with no strikes: {}"#,
      total_records,
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
