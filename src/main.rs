use std::{collections::HashSet, env, sync::Arc};

use crate::commands::leaderboard::LEADERBOARD_COMMAND;
use crate::commands::set_prefix::SET_PREFIX_COMMAND;
use crate::commands::set_welcome_msg::SET_WELCOME_MSG_COMMAND;

use async_trait::async_trait;
use db::{
    guild::{GuildRepo, Guilds},
    users::{Users, UsersRepo},
};
use serenity::{
    framework::{standard::macros::group, StandardFramework},
    model::{
        prelude::{Message, Ready},
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
#[commands(set_prefix, leaderboard, set_welcome_msg)]
pub struct Bot;

pub struct GlobalStateInner {
    guild: Arc<Mutex<Arc<Guilds>>>,
    users: Arc<Mutex<Arc<Users>>>,
    pub active_users: Arc<Mutex<HashSet<i64>>>,
}

pub struct GlobalState {}

impl TypeMapKey for GlobalState {
    type Value = GlobalStateInner;
}
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        crate::services::message::increase_score(
            Arc::new(ctx),
            msg.author.id.0 as i64,
            msg.author.name,
        )
        .await
    }

    async fn voice_state_update(&self, ctx: Context, state: VoiceState) {
        crate::services::message::handle_voice(ctx, state).await
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token for discord in the environment");
    let db_url =
        env::var("DATABASE_URL").expect("Expected a token for database in the environment");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .unwrap();

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES;
    let framework = StandardFramework::new()
        .configure(|c| {
            c.dynamic_prefix(|ctx, msg| {
                Box::pin(async move {
                    let guild_id = msg.guild_id.unwrap().0 as i64;
                    let data = ctx.data.write().await;
                    let global_state = data.get::<GlobalState>();

                    let guild = global_state.unwrap().guild.lock().await;
                    let prefix = guild.get_prefix(guild_id).await;

                    let prefix = Some(prefix);

                    match prefix {
                        Some(s) => {
                            let val = s;
                            if val == "Key not found." {
                                println!("Using ! as prefix");
                                guild.set_prefix(guild_id, "!").await;
                                Some("!".to_string())
                            } else {
                                Some(val)
                            }
                        }
                        None => {
                            println!("Using ! as prefix");
                            guild.set_prefix(guild_id, "!").await;
                            Some("!".to_string())
                        }
                    }
                })
            })
        })
        .group(&BOT_GROUP);
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<GlobalState>(GlobalStateInner {
            guild: Arc::new(Mutex::new(Guilds::new(&pool).await)),
            users: Arc::new(Mutex::new(Users::new(&pool).await)),
            active_users: Arc::new(Mutex::new(HashSet::new())),
        })
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why)
    }
}
