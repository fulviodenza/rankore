use std::{collections::HashSet, env, sync::Arc};

use crate::commands::download_leaderboard::DOWNLOAD_LEADERBOARD_COMMAND;
use crate::commands::get_prefix::GET_PREFIX_COMMAND;
use crate::commands::help::HELP_COMMAND;
use crate::commands::leaderboard::LEADERBOARD_COMMAND;
use crate::commands::multipliers::MULTIPLIERS_COMMAND;
use crate::commands::reset_scores::RESET_SCORES_COMMAND;
use crate::commands::set_prefix::SET_PREFIX_COMMAND;
use crate::commands::set_text_multiplier::SET_TEXT_MULTIPLIER_COMMAND;
use crate::commands::set_voice_multiplier::SET_VOICE_MULTIPLIER_COMMAND;
use crate::commands::set_welcome_msg::SET_WELCOME_MSG_COMMAND;
use crate::services::message::{init_active_users, VoiceStateReady};

use async_trait::async_trait;
use db::{
    guilds::{GuildRepo, Guilds},
    users::{Users, UsersRepo},
};
use serenity::{
    framework::{standard::macros::group, StandardFramework},
    model::{
        guild::Member,
        prelude::{ChannelType, Message, Ready},
        voice::VoiceState,
    },
    prelude::{Context, EventHandler, GatewayIntents, TypeMapKey},
    Client,
};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::Mutex;

mod commands;
mod db;
mod services;

#[group]
#[commands(
    set_prefix,
    get_prefix,
    leaderboard,
    set_welcome_msg,
    help,
    reset_scores,
    set_voice_multiplier,
    set_text_multiplier,
    multipliers,
    download_leaderboard
)]
pub struct Bot;

pub struct GlobalStateInner {
    pub guilds: Arc<Guilds>,
    pub users: Arc<Users>,
    pub active_users: Arc<Mutex<HashSet<(i64, i64)>>>,
}

pub struct GlobalState;

impl TypeMapKey for GlobalState {
    type Value = GlobalStateInner;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }
        let Some(guild_id) = msg.guild_id else {
            return;
        };
        crate::services::message::increase_score(
            &ctx,
            msg.author.id.0 as i64,
            msg.author.name.clone(),
            msg.author.bot,
            guild_id.0 as i64,
        )
        .await;
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        let Some(channel_id) = find_welcome_channel(&ctx, new_member.guild_id.0).await else {
            return;
        };
        let data_read = ctx.data.read().await;
        let Some(global_state) = data_read.get::<GlobalState>() else {
            return;
        };
        let template = global_state
            .guilds
            .get_welcome_msg(new_member.guild_id.0 as i64)
            .await
            .filter(|s| !s.is_empty());
        let welcome_message = match template {
            Some(m) => format!("{}, <@{}>!", m, new_member.user.id),
            None => format!("Welcome, <@{}>!", new_member.user.id),
        };
        if let Err(e) = channel_id.say(&ctx.http, welcome_message).await {
            eprintln!("[guild_member_addition] failed to send welcome: {e}");
        }
    }

    async fn voice_state_update(
        &self,
        ctx: Context,
        _old_state: Option<VoiceState>,
        new_state: VoiceState,
    ) {
        crate::services::message::handle_voice(ctx, new_state).await
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let data_read = ctx.data.read().await;
        let Some(global_state) = data_read.get::<GlobalState>() else {
            return;
        };
        let guilds = match global_state.guilds.guilds().await {
            Ok(g) => g,
            Err(e) => {
                eprintln!("[ready] failed to list guilds: {e}");
                return;
            }
        };
        let guild_ids: Vec<i64> = guilds.into_iter().filter_map(|guild| guild.id).collect();
        // Drop the data lock before doing any HTTP work.
        drop(data_read);

        for id in guild_ids {
            let guild = match ctx.http.get_guild(id as u64).await {
                Ok(g) => g,
                Err(_) => continue,
            };
            let channels = match guild.channels(&ctx.http).await {
                Ok(c) => c,
                Err(_) => continue,
            };
            for (channel_id, channel) in channels {
                if channel.kind != ChannelType::Voice {
                    continue;
                }
                let members = match channel.members(&ctx.cache).await {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                for m in members {
                    init_active_users(
                        ctx.clone(),
                        VoiceStateReady {
                            member: m.clone(),
                            user_id: m.user.id,
                            _channel_id: channel_id,
                            guild_id: channel.guild_id,
                        },
                    )
                    .await;
                }
            }
        }
    }
}

async fn find_welcome_channel(
    ctx: &Context,
    guild_id: u64,
) -> Option<serenity::model::id::ChannelId> {
    let guild = ctx.http.get_guild(guild_id).await.ok()?;
    let channels = guild.channels(&ctx.http).await.ok()?;
    channels
        .into_iter()
        .find(|(_, c)| c.name == "welcome")
        .map(|(id, _)| id)
}

#[tokio::main]
async fn main() {
    let db_url =
        env::var("DATABASE_URL").expect("Expected a token for database in the environment");
    let pool = {
        let mut attempt = 0;
        loop {
            match PgPoolOptions::new().max_connections(50).connect(&db_url).await {
                Ok(p) => break p,
                Err(e) if attempt < 10 => {
                    eprintln!("[db] connect attempt {attempt} failed: {e}; retrying...");
                    attempt += 1;
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
                Err(e) => panic!("failed to connect to database: {e}"),
            }
        }
    };

    let token = env::var("DISCORD_TOKEN").expect("Expected a token for discord in the environment");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS;

    let framework = StandardFramework::new()
        .configure(|c| {
            c.dynamic_prefix(|ctx, msg| {
                Box::pin(async move {
                    let guild_id = msg.guild_id?.0 as i64;
                    let data = ctx.data.read().await;
                    let global_state = data.get::<GlobalState>()?;
                    Some(global_state.guilds.get_prefix(guild_id).await)
                })
            })
        })
        .group(&BOT_GROUP);

    let guilds = Guilds::new(&pool).await;
    let users = Users::new(&pool).await;

    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<GlobalState>(GlobalStateInner {
            guilds,
            users,
            active_users: Arc::new(Mutex::new(HashSet::new())),
        })
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        eprintln!("Client error: {:?}", why);
    }
}
