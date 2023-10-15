use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

pub struct Users {
    users_map: Arc<RwLock<HashMap<u64, User>>>,
}

#[derive(Clone, Debug)]
pub struct User {
    pub id: u64,
    pub score: u64,
}

impl User {
    fn new(id: u64) -> Self {
        Self { id, score: 0 }
    }
}

#[async_trait]
pub trait UsersRepo {
    fn new() -> Self;
    async fn get_user(&self, id: u64) -> User;
    async fn insert_user(&mut self, user: User);
    async fn update_user(&mut self, user: User);
}

#[async_trait]
impl UsersRepo for Users {
    fn new() -> Self {
        let users_map: Arc<RwLock<HashMap<u64, User>>> =
            Arc::new(RwLock::new(HashMap::from([(0, User { id: 0, score: 0 })])));
        Users { users_map }
    }
    async fn get_user(&self, id: u64) -> User {
        let mut binding = self.users_map.write().await;
        let user = binding.entry(id).or_insert_with(|| User::new(id));
        user.clone()
    }

    async fn insert_user(&mut self, user: User) {
        let user_id = user.id;
        let mut map = self.users_map.write().await;
        map.insert(user.id, user);
        println!("User {:?} added.", user_id);
    }

    async fn update_user(&mut self, user: User) {
        let user_id = user.id;
        let mut map = self.users_map.write().await;
        if map.get_mut(&user_id).is_some() {
            println!("User updated {:?}", map.keys());
            map.insert(user_id, user);
        } else {
            println!("User not updated {:?}", map.keys());
            map.insert(user_id, user);
        }
    }
}
