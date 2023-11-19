use async_trait::async_trait;
use sqlx::{Error, FromRow, Pool, Postgres};
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

#[derive(Clone, Debug, FromRow)]
pub struct User {
    #[sqlx(default)]
    pub id: i64,
    #[sqlx(default)]
    pub score: i64,
    #[sqlx(default)]
    pub nick: String,
    #[sqlx(default)]
    pub is_bot: bool,
    #[sqlx(default)]
    pub guild_id: i64,
}

#[async_trait]
pub trait UsersRepo {
    async fn new(pool: &Pool<Postgres>) -> Arc<Self>;
    async fn insert_user(&self, user: User);
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

    async fn insert_user(&self, user: User) {
        let _ = sqlx::query!(
            "INSERT into users(id, score, nick, is_bot, guild_id) values ($1, $2, $3, $4, $5)",
            user.id,
            user.score,
            user.nick,
            user.is_bot,
            user.guild_id,
        )
        .execute(&self.pool)
        .await;
    }

    async fn update_user(pool: &Pool<Postgres>, user: User) {
        let temp_user: Result<User, Error> = sqlx::query_as!(
            User,
            "select * from users where id = $1 and guild_id = $2",
            user.id,
            user.guild_id
        )
        .fetch_one(pool)
        .await;
        match temp_user {
            Ok(u) => {
                let _ = sqlx::query!(
                    "UPDATE users SET score = $1, nick = $2, is_bot = $3 WHERE id = $5 and guild_id = $4",
                    u.score + 1,
                    user.nick,
                    user.is_bot,
                    user.guild_id,
                    user.id,
                )
                .execute(pool)
                .await;
            }
            Err(_) => {
                let _ = sqlx::query!(
                    "INSERT into users(id, score, nick, is_bot, guild_id) values ($1, $2, $3, $4, $5)",
                    user.id,
                    0,
                    "",
                    user.is_bot,
                    user.guild_id,
                )
                .execute(pool)
                .await;
            }
        }
    }

    async fn get_users(&self, guild_id: i64) -> Vec<User> {
        let result = sqlx::query_as!(User, "select * from users WHERE guild_id = $1", guild_id)
            .fetch_all(&self.pool)
            .await
            .unwrap();

        let users_vec: Vec<User> = result
            .iter()
            .map(|user| User {
                id: user.id,
                score: user.score,
                nick: user.nick.clone(),
                is_bot: user.is_bot,
                guild_id: user.guild_id,
            })
            .collect();

        users_vec
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
                                    println!("increasing with: {:?} multiplier", multiplier);
                                    db::users::Users::update_user(&user_pool_clone, User { id: user_id, score: 0, nick: nick.clone(), is_bot, guild_id})
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
                        },
                    )
                    .await;
                }
            }
        }
    }
}
