use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

// TODO: Replace this with an actual database
pub struct Guild {
    prefix_map: Arc<RwLock<HashMap<u64, String>>>,
}

pub trait GuildRepo {
    fn new() -> Self;
    fn set_prefix(&self, guild_id: u64, prefix: &str);
    fn get_prefix(&self, guild_id: u64) -> String;
}

impl GuildRepo for Guild {
    fn new() -> Self {
        Guild {
            prefix_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    fn set_prefix(&self, guild_id: u64, prefix: &str) {
        let mut map = self
            .prefix_map
            .write()
            .expect("Failed to acquire prefix_map lock");
        map.insert(guild_id, prefix.to_string());
        println!(
            "State prefix changed to {:?} for guild: {:?}",
            prefix, guild_id
        )
    }

    fn get_prefix(&self, guild_id: u64) -> String {
        let locked_data = self.prefix_map.clone();
        let data = locked_data.read().unwrap();
        match data.get(&guild_id) {
            Some(value) => format!("Value: {}", value),
            None => "Key not found.".to_string(),
        }
    }
}
