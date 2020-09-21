use serenity::{
  async_trait,
  client::EventHandler,
  model::{
    event::ResumedEvent,
    gateway::Ready,
    guild::Member,
    id::GuildId,
    user::User,
  },
  prelude::Context,
  utils::Color,
};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::{BotConfig, PunishedUser};
use crate::state::BotState;

pub(crate) struct BotHandler {
  pub(self) state: Arc<Mutex<BotState>>,
  pub(self) config: Arc<BotConfig>,
}

impl BotHandler {
  pub(self) async fn warn_mods(&self, ctx: &Context, punished: &PunishedUser) {
    let usr = punished.id.to_user(&ctx).await.unwrap();
    let _ = self.config.warn_channel.send_message(&ctx, |m| {
      m.embed(|e| {
        e.title("Pit Threshold Reached");
        e.description(format!(r#"
User: <@{}>
Also known as: `{}`
Active Strikes: `{}`
"#, punished.id, usr.tag(), punished.times_punished)
        );
        e.color(Color::ORANGE);

        e
      });

      m
    })
      .await
      .unwrap();
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
    if new.roles.contains(&self.config.punishment_role) {
      let mut user: Option<PunishedUser> = None;
      {
        let mut state = self.state.lock().expect("Unable to read from state");

        for punished in state.users.iter_mut() {
          if punished.id == new.user.id {
            punished.times_punished += 1;
            punished.last_punish = SystemTime::now();

            user = Some(punished.clone());

            break;
          }
        }

        state.pits += 1;

        if user.is_none() {
          state.users.push(PunishedUser {
            id: new.user.id,
            times_punished: 1,
            last_punish: SystemTime::now(),
          });

          return
        }
      }

      if user.is_some() {
        self.warn_mods(&ctx, &user.unwrap()).await;
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
  pub(crate) fn new(data: Arc<Mutex<BotState>>, config: Arc<BotConfig>) -> Self {
    Self {
      state: Arc::clone(&data),
      config,
    }
  }
}
