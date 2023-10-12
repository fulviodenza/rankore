use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

// TODO: Replace this with an actual database
pub struct Guilds {
    guilds_map: Arc<RwLock<HashMap<u64, Guild>>>,
}

pub struct Guild {
    prefix: String,
    welcome_msg: String,
}
pub trait GuildRepo {
    fn new() -> Self;
    fn set_prefix(&self, guild_id: u64, prefix: &str);
    fn get_prefix(&self, guild_id: u64) -> String;
    fn set_welcome_msg(&self, guild_id: u64, welcome_msg: &str);
}

impl GuildRepo for Guilds {
    fn new() -> Self {
        Guilds {
            guilds_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    fn set_prefix(&self, guild_id: u64, prefix: &str) {
        let guild_binding = self
            .guilds_map
            .read()
            .expect("Failed to acquire prefix_map lock");
        let guild = guild_binding.get(&guild_id).unwrap();

        let new_guild = Guild {
            prefix: prefix.to_string(),
            welcome_msg: guild.welcome_msg.clone(),
        };
        let mut map = self
            .guilds_map
            .write()
            .expect("Failed to acquire prefix_map lock");

        map.insert(guild_id, new_guild);
        println!(
            "State prefix changed to {:?} for guild: {:?}",
            prefix, guild_id
        )
    }
    fn set_welcome_msg(&self, guild_id: u64, welcome_msg: &str) {
        let guild_binding = self
            .guilds_map
            .read()
            .expect("Failed to acquire prefix_map lock");
        let guild = guild_binding.get(&guild_id).unwrap();

        let new_guild = Guild {
            prefix: welcome_msg.to_string(),
            welcome_msg: guild.prefix.clone(),
        };
        let mut map = self
            .guilds_map
            .write()
            .expect("Failed to acquire prefix_map lock");

        map.insert(guild_id, new_guild);
        println!(
            "State welcome_msg changed to {:?} for guild: {:?}",
            welcome_msg, guild_id
        )
    }
    fn get_prefix(&self, guild_id: u64) -> String {
        let locked_data = self.guilds_map.clone();
        let data = locked_data.read().unwrap();
        match data.get(&guild_id) {
            Some(value) => value.prefix.to_string(),
            None => "Key not found.".to_string(),
        }
    }
}
