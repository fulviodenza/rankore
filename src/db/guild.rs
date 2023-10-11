use std::sync::RwLock;

// TODO: Replace this with an actual database
pub struct Guild {
    prefix: RwLock<String>,
}

pub trait GuildRepo {
    fn new() -> Self;
    fn set_prefix(&mut self, prefix: String);
    fn get_prefix(&self) -> String;
}

impl GuildRepo for Guild {
    fn new() -> Self {
        Guild {
            prefix: RwLock::new("!".to_string()),
        }
    }
    fn set_prefix(&mut self, prefix: String) {
        let mut data = self.prefix.write().expect("Failed to acquire write lock");
        *data = prefix;
        println!("State changed to {:?}", *data)
    }

    fn get_prefix(&self) -> String {
        let data = self.prefix.read().expect("Failed to acquire read lock");
        data.clone()
    }
}
