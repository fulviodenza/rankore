use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

// TODO: Replace this with an actual database
pub struct Guilds {
    guilds_map: Arc<RwLock<HashMap<i64, Guild>>>,
}

#[derive(Debug)]
pub struct Guild {
    prefix: String,
    welcome_msg: String,
}

impl Default for Guild {
    fn default() -> Self {
        Self {
            prefix: "/".to_string(),
            welcome_msg: "Welcome!".to_string(),
        }
    }
}

#[async_trait]
pub trait GuildRepo {
    fn new() -> Self;
    async fn set_prefix(&self, guild_id: i64, prefix: &str);
    async fn get_prefix(&self, guild_id: i64) -> String;
    async fn set_welcome_msg(&self, guild_id: i64, welcome_msg: &str);
}

#[async_trait]
impl GuildRepo for Guilds {
    fn new() -> Self {
        Guilds {
            guilds_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    async fn set_prefix(&self, guild_id: i64, prefix: &str) {
        let mut guild_binding = self.guilds_map.write().await;
        let guild = guild_binding.entry(guild_id).or_insert_with(Guild::default);
        guild.prefix = prefix.to_string();
        println!(
            "State prefix changed to {:?} for guild: {:?}",
            guild, guild_id
        )
    }
    async fn set_welcome_msg(&self, guild_id: i64, welcome_msg: &str) {
        let mut guild_binding = self.guilds_map.write().await;
        let guild = guild_binding.entry(guild_id).or_insert_with(Guild::default);
        guild.welcome_msg = welcome_msg.to_string();
        println!(
            "State prefix changed to {:?} for guild: {:?}",
            guild, guild_id
        )
    }
    async fn get_prefix(&self, guild_id: i64) -> String {
        let locked_data = self.guilds_map.clone();
        let data = locked_data.read().await;
        match data.get(&guild_id) {
            Some(value) => value.prefix.to_string(),
            None => "Key not found.".to_string(),
        }
    }
}
