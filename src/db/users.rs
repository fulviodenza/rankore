use async_trait::async_trait;
use sqlx::{
    postgres::{PgPoolOptions, PgRow},
    Error, FromRow, Pool, Postgres, Row,
};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    select,
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot, RwLock,
    },
};

use crate::db;

use super::events::{UserEvents, UserObserver};

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
}

#[async_trait]
pub trait UsersRepo {
    async fn new(db_url: String) -> Arc<Self>;
    async fn get_user(&self, id: i64) -> User;
    async fn insert_user(&self, user: User);
    async fn update_user(pool: &Pool<Postgres>, id: User);
    async fn get_users(&self) -> Vec<User>;
}

#[async_trait]
impl UsersRepo for Users {
    async fn new(db_url: String) -> Arc<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await
            .unwrap();

        let (tx, rx) = mpsc::unbounded_channel::<UserEvents>();
        let users = Arc::new(Users { tx, pool });
        let users_clone = Arc::clone(&users);
        tokio::spawn(async move {
            users_clone.notify(rx).await;
        });
        users
    }
    async fn get_user(&self, id: i64) -> User {
        let temp_user: Result<User, Error> =
            sqlx::query_as!(User, "select * from users where id = $1", id)
                .fetch_one(&self.pool)
                .await;
        match temp_user {
            Err(_) => {
                let _ = sqlx::query!(
                    "INSERT into users(id, score, nick) values ($1, $2, $3)",
                    id as i64,
                    0,
                    ""
                )
                .execute(&self.pool)
                .await;
                return User {
                    id,
                    score: 0,
                    nick: "".to_string(),
                };
            }
            Ok(u) => {
                return User {
                    id: u.id,
                    score: u.score,
                    nick: u.nick,
                }
            }
        };
    }

    async fn insert_user(&self, user: User) {
        let _ = sqlx::query!(
            "INSERT into users(id, score, nick) values ($1, $2, $3)",
            user.id as i64,
            user.score as i64,
            user.nick
        )
        .execute(&self.pool)
        .await;
    }

    async fn update_user(pool: &Pool<Postgres>, user: User) {
        let temp_user: Result<User, Error> =
            sqlx::query_as!(User, "select * from users where id = $1", user.id as i64)
                .fetch_one(pool)
                .await;
        match temp_user {
            Ok(u) => {
                let _ = sqlx::query!(
                    "UPDATE users SET  score = $1, nick = $2 WHERE id = $3",
                    u.score + 1 as i64,
                    user.nick,
                    user.id as i64,
                )
                .execute(pool)
                .await;
            }
            Err(_) => {
                let _ = sqlx::query!(
                    "INSERT into users(id, score, nick) values ($1, $2, $3)",
                    user.id as i64,
                    0,
                    "",
                )
                .execute(pool)
                .await;
            }
        }
    }

    async fn get_users(&self) -> Vec<User> {
        let result: Vec<PgRow> = sqlx::query("select * from users")
            .fetch_all(&self.pool)
            .await
            .unwrap();

        let users_vec: Vec<User> = result
            .iter()
            .map(|row| {
                User {
                    id: row.get(0),    // 0 -> 'id' column
                    score: row.get(1), // 1 -> 'score' column
                    nick: row.get(2),  // 2 -> 'nick' column
                }
            })
            .collect();

        users_vec
    }
}

#[async_trait]
impl UserObserver for Users {
    async fn notify(&self, mut rx: UnboundedReceiver<UserEvents>) {
        let hashmap: Arc<RwLock<HashMap<i64, oneshot::Sender<()>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        while let Some(event) = rx.recv().await {
            let users_pool = self.pool.clone();

            match event {
                UserEvents::Joined(user_id, nick) => {
                    let (tx, mut rx) = oneshot::channel::<()>();
                    hashmap.write().await.insert(user_id, tx);
                    tokio::spawn(async move {
                        loop {
                            let user_pool_clone = users_pool.clone();

                            select! {
                                _ = tokio::time::sleep(tokio::time::Duration::from_secs(2)) => {
                                    db::users::Users::update_user(&user_pool_clone, User { id: user_id, score:0 , nick: nick.clone() })
                                    .await;
                                },
                                _ = &mut rx => {
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
                    Users::update_user(
                        &self.pool,
                        User {
                            id: user_id,
                            score: 0,
                            nick,
                        },
                    )
                    .await;
                }
            }
        }
    }
}
