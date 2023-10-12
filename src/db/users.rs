use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub struct Users {
    users_map: Arc<RwLock<HashMap<u64, User>>>,
}

#[derive(Clone, Debug)]
pub struct User {
    pub id: u64,
    pub score: u64,
}

pub trait UsersRepo {
    fn new() -> Self;
    fn get_user(&self, id: u64) -> User;
    fn insert_user(&mut self, user: User) -> bool;
    fn update_user(&mut self, user: User);
}

impl UsersRepo for Users {
    fn new() -> Self {
        let users_map: Arc<RwLock<HashMap<u64, User>>> =
            Arc::new(RwLock::new(HashMap::from([(0, User { id: 0, score: 0 })])));
        Users { users_map }
    }
    fn get_user(&self, id: u64) -> User {
        let binding = self.users_map.read().unwrap();
        let user = binding.get(&id);
        match user {
            Some(u) => u.clone(),
            None => User { id, score: 0 },
        }
        // User {
        //     id: user.unwrap().id,
        //     score: user.unwrap().score,
        // }
    }
    fn insert_user(&mut self, user: User) -> bool {
        let user_id = user.id;

        let mut map = self
            .users_map
            .write()
            .expect("Failed to acquire users_map lock");

        map.insert(user.id, user);

        println!("User {:?} added.", user_id);
        true
    }
    fn update_user(&mut self, user: User) {
        let user_id = user.id;
        let mut map = self
            .users_map
            .write()
            .expect("Failed to acquire users_map lock");
        if map.get_mut(&user_id).is_some() {
            println!("User updated {:?}", map.keys());
            map.insert(user_id, user);
        } else {
            println!("User not updated {:?}", map.keys());
            map.insert(user_id, user);
        }
    }
}
