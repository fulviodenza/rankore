use std::{env, sync::Mutex};

use crate::commands::set_prefix::SET_PREFIX_COMMAND;
use async_trait::async_trait;
use db::guild::{Guild, GuildRepo};
use serenity::{
    framework::{standard::macros::group, StandardFramework},
    model::prelude::Ready,
    prelude::{Context, EventHandler, GatewayIntents, TypeMapKey},
    Client,
};

mod commands;
mod db;

#[group]
#[commands(set_prefix)]
pub struct Bot;

pub struct GlobalStateInner {
    guild: Mutex<Guild>,
}

pub struct GlobalState {}

impl TypeMapKey for GlobalState {
    type Value = GlobalStateInner;
}
struct Handler;

#[async_trait]
impl EventHandler for Handler {
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
                            // from the map we get a string "Key not found." if the key
                            // is not found, while we get a "Value: !" for example which
                            // we need to split and take only the last character "!"
                            if s == "Key not found." {
                                println!("Using ! as prefix");
                                Some("!".to_string())
                            } else {
                                let clean_prefix: Vec<&str> = s.split(' ').collect();
                                let clean_prefix = clean_prefix.get(1);
                                println!("Using {:?} as prefix", clean_prefix);
                                clean_prefix.map(|&double_ref_str| double_ref_str.to_string())
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
            guild: Mutex::new(Guild::new()),
        })
        .await
        .expect("Error creating client");
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why)
    }
}
