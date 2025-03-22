use async_trait::async_trait;
use sqlx::{Pool, Postgres};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    select,
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot, RwLock,
    },
};

use crate::db;

use super::events::{Observer, UserEvents};

#[derive(Clone)]
pub struct Users {
    pub pool: Pool<Postgres>,
    pub tx: UnboundedSender<UserEvents>,
}

#[derive(Clone, Debug)]
pub struct User {
    pub id: i64,
    pub score: i64,
    pub nick: String,
    pub is_bot: bool,
    pub guild_id: i64,
    pub has_left: bool,  // Changed to snake_case
}

#[async_trait]
pub trait UsersRepo {
    async fn new(pool: &Pool<Postgres>) -> Arc<Self>;
    async fn update_user(pool: &Pool<Postgres>, id: User);
    async fn get_users(&self, guild_id: i64) -> Vec<User>;
    async fn reset_scores(&self, guild_id: i64);
}

#[async_trait]
impl UsersRepo for Users {
    async fn new(pool: &Pool<Postgres>) -> Arc<Self> {
        let (tx, rx) = mpsc::unbounded_channel::<UserEvents>();
        let users = Arc::new(Users {
            tx,
            pool: pool.clone(),
        });
        let users_clone = Arc::clone(&users);
        tokio::spawn(async move {
            users_clone.notify(rx).await;
        });
        users
    }

    async fn update_user(pool: &Pool<Postgres>, user: User) {
        let temp_user = sqlx::query!(
            "select * from users where id = $1 and guild_id = $2",
            user.id,
            user.guild_id
        )
        .fetch_one(pool)
        .await;
        
        match temp_user {
            Ok(u) => {
                let res = sqlx::query!(
                    "UPDATE users SET score = $1, nick = $2, is_bot = $3, hasleft = $4 WHERE id = $5 and guild_id = $6",
                    u.score + 1,
                    user.nick,
                    user.is_bot,
                    user.has_left,
                    user.id,
                    user.guild_id,
                )
                .execute(pool)
                .await;
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        println!("[update_user]: got error {}", e)
                    }
                }
            }
            Err(_) => {
                let _ = sqlx::query!(
                    "INSERT into users(id, score, nick, is_bot, guild_id, hasleft) values ($1, $2, $3, $4, $5, $6)",
                    user.id,
                    0,
                    user.nick,
                    user.is_bot,
                    user.guild_id,
                    user.has_left,
                )
                .execute(pool)
                .await;
            }
        }
    }

    async fn get_users(&self, guild_id: i64) -> Vec<User> {
        let result = sqlx::query!(
            "select * from users WHERE guild_id = $1",
            guild_id
        )
        .fetch_all(&self.pool)
        .await
        .unwrap();

        result
            .iter()
            .map(|row| User {
                id: row.id,
                score: row.score,
                nick: row.nick.clone(),
                is_bot: row.is_bot,
                guild_id: row.guild_id,
                has_left: row.hasleft.unwrap_or(false),
            })
            .collect()
    }

    async fn reset_scores(&self, guild_id: i64) {
        let _ = sqlx::query!(
            "UPDATE users SET score = $1 WHERE guild_id = $2",
            0,
            guild_id
        )
        .execute(&self.pool)
        .await;
    }
}

#[async_trait]
impl Observer for Users {
    async fn notify(&self, mut rx: UnboundedReceiver<UserEvents>) {
        let hashmap: Arc<RwLock<HashMap<i64, oneshot::Sender<()>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        while let Some(event) = rx.recv().await {
            let users_pool = self.pool.clone();

            match event {
                UserEvents::Joined(user_id, nick, is_bot, guild_id, multiplier) => {
                    let (tx, mut rx) = oneshot::channel::<()>();
                    hashmap.write().await.insert(user_id, tx);
                    tokio::spawn(async move {
                        loop {
                            let user_pool_clone = users_pool.clone();

                            select! {
                                _ = tokio::time::sleep(tokio::time::Duration::from_secs(multiplier as u64)) => {
                                    db::users::Users::update_user(&user_pool_clone, User {
                                        id: user_id,
                                        score: 0,
                                        nick: nick.clone(),
                                        is_bot,
                                        guild_id,
                                        has_left: false
                                    })
                                    .await;
                                },
                                _ = &mut rx => {
                                    break
                                },
                            };
                        }
                    });
                }
                UserEvents::Left(user_id) => {
                    let mut writing_hashmap = hashmap.write().await;
                    let sender = writing_hashmap.remove(&user_id);
                    if let Some(sender) = sender {
                        let _ = sender.send(());
                    }
                    
                    let temp_users = sqlx::query!(
                        "select * from users where id = $1",
                        user_id
                    )
                    .fetch_all(&users_pool)
                    .await;
                
                    if let Ok(users) = temp_users {
                        for row in users {
                            let mut user = User {
                                id: row.id,
                                score: row.score,
                                nick: row.nick,
                                is_bot: row.is_bot,
                                guild_id: row.guild_id,
                                has_left: row.hasleft.unwrap_or(false)
                            };
                            user.has_left = true;
                            Users::update_user(
                                &users_pool,
                                user
                            )
                            .await;
                        }
                    }
                }
                UserEvents::SentText(user_id, nick, is_bot, guild_id, multiplier) => {
                    Users::update_user(
                        &self.pool,
                        User {
                            id: user_id,
                            score: multiplier,
                            nick,
                            is_bot,
                            guild_id,
                            has_left: false
                        },
                    )
                    .await;
                }
            }
        }
    }
}