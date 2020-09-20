use serenity::{
  async_trait,
  client::EventHandler,
  model::{
    event::ResumedEvent,
    gateway::Ready,
    guild::Member,
    id::{
      GuildId,
      RoleId,
    },
    user::User,
  },
  prelude::Context,
};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::{BotConfig, BotState};

pub(crate) struct BotHandler {
  pub(self) state: Arc<Mutex<BotState>>,
  pub(self) config: BotConfig,
}

impl BotHandler {
  pub(self) fn warn_mods(&self, ctx: &Context, member: &Member) {
    
  }
}

#[async_trait]
impl EventHandler for BotHandler {
  async fn guild_ban_addition(&self, _: Context, _: GuildId, banned_user: User) {
    let mut state = self.state.lock().expect("Unable to read from state");

    state.users.drain_filter(|x| x == banned_user);
    state.bans += 1;
  }

  async fn guild_member_update(&self, ctx: Context, _: Option<Member>, new: Member) {
    let mut should_warn: bool = false;

    if new.roles.contains(&self.config.punishment_role) {
      {
        let mut state = self.state.lock().expect("Unable to read from state");

        for punished in state.users.iter_mut() {
          if punished.user == new.user {
            punished.times_punished += 1;
            punished.last_punish = SystemTime::now();

            should_warn = punished.times_punished >= self.config.warn_threshold;

            break;
          }
        }
      }

      if should_warn {

      }
    }
  }

  async fn guild_unavailable(&self, _: Context, id: GuildId) {
    println!("Guild# {} has become unavailable!", id);
  }

  async fn ready(&self, _: Context, ready: Ready) {
    println!("Connected to Discord as {}", ready.user.name);
  }

  async fn resume(&self, _: Context, _: ResumedEvent) {
    println!("Resumed connection to Discord");
  }
}

impl BotHandler {
  pub(crate) fn new(data: Arc<Mutex<BotState>>, config: BotConfig) -> Self {
    Self {
      state: Arc::clone(&data),
      config,
    }
  }
}
