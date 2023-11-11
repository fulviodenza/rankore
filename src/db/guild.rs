use std::sync::Arc;

use async_trait::async_trait;
use sqlx::{Error, FromRow, Pool, Postgres};

pub struct Guilds {
    pub pool: Pool<Postgres>,
}

#[derive(Debug, FromRow)]
pub struct Guild {
    #[sqlx(default)]
    id: Option<i64>,
    #[sqlx(default)]
    prefix: String,
    #[sqlx(default)]
    welcome_msg: Option<String>,
}

impl Default for Guild {
    fn default() -> Self {
        Self {
            id: Some(0),
            prefix: "!".to_string(),
            welcome_msg: Some("Welcome!".to_string()),
        }
    }
}

#[async_trait]
pub trait GuildRepo {
    async fn new(pool: &Pool<Postgres>) -> Arc<Self>;
    async fn set_prefix(&self, guild_id: i64, prefix: &str);
    async fn get_prefix(&self, guild_id: i64) -> String;
    async fn set_welcome_msg(&self, guild_id: i64, welcome_msg: &str);
}

#[async_trait]
impl GuildRepo for Guilds {
    async fn new(pool: &Pool<Postgres>) -> Arc<Self> {
        Arc::new(Guilds { pool: pool.clone() })
    }
    async fn set_prefix(&self, guild_id: i64, prefix: &str) {
        let result = sqlx::query!(
            "UPDATE guilds SET prefix = $1 WHERE id = $2",
            prefix,
            guild_id as i64
        )
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(_) => {}
            Err(_) => {
                let _ = sqlx::query!(
                    "INSERT into guilds(id, prefix, welcome_msg) values ($1, $2, $3)",
                    guild_id as i64,
                    prefix,
                    "",
                )
                .execute(&self.pool)
                .await;
            }
        }
    }
    async fn set_welcome_msg(&self, guild_id: i64, welcome_msg: &str) {
        let result = sqlx::query_as!(
            Guild,
            "UPDATE guilds SET welcome_msg = $1 WHERE id = $2",
            welcome_msg,
            guild_id as i64
        )
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(_) => {}
            Err(_) => {
                let _ = sqlx::query!(
                    "INSERT into guilds(id, prefix, welcome_msg) values ($1, $2, $3)",
                    guild_id as i64,
                    "!",
                    welcome_msg,
                )
                .execute(&self.pool)
                .await;
            }
        }
    }
    async fn get_prefix(&self, guild_id: i64) -> String {
        let guild: Result<Guild, Error> =
            sqlx::query_as!(Guild, "select * from guilds where id = $1", guild_id)
                .fetch_one(&self.pool)
                .await;
        match guild {
            Err(_) => {
                let _ = sqlx::query!(
                    "INSERT into guilds(id, prefix, welcome_msg) values ($1, $2, $3)",
                    guild_id as i64,
                    "!",
                    "Welcome!"
                )
                .execute(&self.pool)
                .await;
                return "!".to_string();
            }
            Ok(g) => return g.prefix.to_string(),
        };
    }
}
