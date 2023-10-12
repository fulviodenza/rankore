use std::{env, sync::Mutex};

use crate::commands::set_prefix::SET_PREFIX_COMMAND;
use async_trait::async_trait;
use db::guild::{GuildRepo, Guilds};
use serenity::{
    framework::{standard::macros::group, StandardFramework},
    model::prelude::{Message, Ready},
    prelude::{Context, EventHandler, GatewayIntents, TypeMapKey},
    Client,
};

mod commands;
mod db;
mod services;

#[group]
#[commands(set_prefix)]
pub struct Bot;

pub struct GlobalStateInner {
    guild: Mutex<Guilds>,
}

pub struct GlobalState {}

impl TypeMapKey for GlobalState {
    type Value = GlobalStateInner;
}
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        crate::services::message::handle_message(ctx, msg)
    }
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let framework = StandardFramework::new()
        .configure(|c| {
            c.dynamic_prefix(|ctx, msg| {
                Box::pin(async move {
                    let guild_id = msg.guild_id.unwrap().0;
                    let data = ctx.data.read().await;
                    let global_state = data.get::<GlobalState>();

                    let prefix = Some(
                        global_state
                            .unwrap()
                            .guild
                            .lock()
                            .unwrap()
                            .get_prefix(guild_id),
                    );

                    println!("Arrived here with prefix: {:?}", prefix);

                    match prefix {
                        Some(s) => {
                            if s == "Key not found." {
                                println!("Using ! as prefix");
                                Some("!".to_string())
                            } else {
                                Some(s)
                            }
                        }
                        None => {
                            println!("Using ! as prefix");
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
            guild: Mutex::new(Guilds::new()),
        })
        .await
        .expect("Error creating client");
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why)
    }
}
