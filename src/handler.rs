use serenity::{client::EventHandler, model::{event::ResumedEvent, gateway::Ready, guild::Member, id::GuildId, user::User}, prelude::Context, utils::Color};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::{BotConfig, PunishedUser};
use crate::state::BotState;

pub(crate) struct BotHandler {
  pub(self) state: Arc<Mutex<BotState>>,
  pub(self) config: Arc<BotConfig>,
}

impl EventHandler for BotHandler {
  fn guild_ban_addition(&self, _: Context, _: GuildId, banned_user: User) {
    let mut state = self.state.lock().expect("Unable to read from state");

    state.users.drain_filter(|x| x == banned_user);
    state.bans += 1;
  }

  fn guild_member_update(&self, mut ctx: Context, old: Option<Member>, new: Member) {
    let mut user: Option<PunishedUser> = None;

    {
      let mut state = self.state.lock().expect("Unable to read from state");
      let mut found = false;

      if new.roles.contains(&self.config.punishment_role) {
        for punished in state.users.iter_mut() {
          if punished.id == new.user_id() {
            if !punished.pitted {
              punished.pitted = true;
              punished.times_punished += 1;
              punished.last_punish = SystemTime::now();

              user = Some(punished.clone());
            }

            found = true;

            break;
          }
        }

        if user.is_none() && !found {
          state.users.push(PunishedUser {
            pitted: true,
            id: new.user_id(),
            times_punished: 1,
            last_punish: SystemTime::now(),
          });

          return
        }
      } else if let Some(_) = old {
        for punished in state.users.iter_mut() {
          if punished.id == new.user_id() && punished.pitted {
            punished.pitted = false;

            break;
          }
        }
      }
    }
    
    if user.is_some() {
      self.warn_mods(&mut ctx, &user.unwrap());
    }
  }

  fn guild_unavailable(&self, _: Context, id: GuildId) {
    println!("Guild# {} has become unavailable!", id);
  }

  fn ready(&self, _: Context, ready: Ready) {
    if let Some(shard) = ready.shard {
      println!(
        "Connected to Discord as {} on shard {}/{}",
        ready.user.name,
        shard[0],
        shard[1],
      );
    }
  }

  fn resume(&self, _: Context, _: ResumedEvent) {
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

  pub(self) fn warn_mods(&self, ctx: &mut Context, punished: &PunishedUser) {
    let usr = punished.id.to_user(&ctx).unwrap();
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
      .unwrap();
  }
}
