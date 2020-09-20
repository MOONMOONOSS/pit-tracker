use serenity::model::{
  channel::GuildChannel,
  guild::Role,
  id::UserId,
  user::User,
};
use std::time::SystemTime;

pub struct BotConfig {
  pub settle_time: u16,
  pub(crate) token: String,
  pub warn_channel: GuildChannel,
  pub warn_role: Role,
  pub warn_threshold: u16,
}

pub struct BotState {
  pub(self) bans: u64,
  pub(self) pits: u64,
  pub(self) unique_users: u64,
  pub(self) uptime: SystemTime,
  pub(self) users: Vec<PunishedUser>,
}

pub struct PunishedUser {
  pub(self) id: UserId,
  pub(self) user: User,
  pub times_punished: u16,
  pub last_punish: SystemTime,
}

fn main() {
  println!("Hello, world!");
}
