use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::{
    select,
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot, RwLock,
    },
};

use super::events::{UserEvents, UserObserver};

#[derive(Clone)]
pub struct Users {
    users_map: Arc<RwLock<HashMap<u64, User>>>,
    pub tx: UnboundedSender<UserEvents>,
}

#[derive(Clone, Debug)]
pub struct User {
    pub id: u64,
    pub score: u64,
    pub nick: String,
}

impl User {
    fn new(id: u64) -> Self {
        let empty_nick = "".to_string();
        Self {
            id,
            score: 0,
            nick: empty_nick,
        }
    }
}

#[async_trait]
pub trait UsersRepo {
    fn new() -> Self;
    async fn get_user(&self, id: u64) -> User;
    async fn insert_user(&mut self, user: User);
    async fn update_user<F>(id: u64, update_fn: F, users_lock: Arc<RwLock<HashMap<u64, User>>>)
    where
        F: Fn(&mut User) + Sync + Send;
    async fn get_users(&mut self) -> Vec<User>;
}

#[async_trait]
impl UsersRepo for Users {
    fn new() -> Self {
        let users_map: Arc<RwLock<HashMap<u64, User>>> = Arc::new(RwLock::new(HashMap::new()));
        let user_map_clone = users_map.clone();
        let (tx, rx) = mpsc::unbounded_channel::<UserEvents>();
        let users = Users { users_map, tx };
        tokio::spawn(Self::notify(rx, user_map_clone));
        users
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

    async fn update_user<F>(id: u64, update_fn: F, users_lock: Arc<RwLock<HashMap<u64, User>>>)
    where
        F: Fn(&mut User) + Sync + Send,
    {
        let mut write_lock = users_lock.write().await;

        let contains = write_lock.contains_key(&id);
        write_lock
            .entry(id)
            .and_modify(|user| {
                update_fn(user);
            })
            .or_insert(User {
                id,
                score: 0,
                nick: "".to_string(),
            });

        match contains {
            true => {}
            false => {
                write_lock.entry(id).and_modify(|user| update_fn(user));
            }
        }
        let user = write_lock.get(&id).unwrap();
        println!("{:?}", user);
    }

    async fn get_users(&mut self) -> Vec<User> {
        self.users_map.read().await.values().cloned().collect()
    }
}

#[async_trait]
impl UserObserver for Users {
    async fn notify(
        mut rx: UnboundedReceiver<UserEvents>,
        user_lock: Arc<RwLock<HashMap<u64, User>>>,
    ) {
        let hashmap: Arc<RwLock<HashMap<u64, oneshot::Sender<()>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        while let Some(event) = rx.recv().await {
            match event {
                UserEvents::Joined(user_id) => {
                    let (tx, mut rx) = oneshot::channel::<()>();
                    let users_map = Arc::clone(&user_lock);
                    hashmap.write().await.insert(user_id, tx);

                    tokio::spawn(async move {
                        loop {
                            select! {
                                _ = tokio::time::sleep(tokio::time::Duration::from_secs(2)) => {
                                    Users::update_user(
                                        user_id,
                                        |user: &mut User| user.score += 1,
                                        users_map.clone(),
                                    )
                                    .await;
                                },
                                _ = &mut rx => {
                                    println!("hello, i entered here");
                                    break
                                },
                            };
                            println!("increased");
                        }
                    });
                }
                UserEvents::Left(user_id) => {
                    let mut writing_hashmap = hashmap.write().await;
                    let sender = writing_hashmap.remove(&user_id);
                    if let Some(sender) = sender {
                        let _ = sender.send(());
                    }
                }
                UserEvents::SentText(user_id, nick) => {
                    let users_map = Arc::clone(&user_lock);
                    Users::update_user(
                        user_id,
                        |user: &mut User| {
                            user.score += 1;
                            user.nick = nick.clone();
                        },
                        users_map,
                    )
                    .await;
                    println!("user: {:?} increased score", user_id);
                }
            }
        }
    }
}
