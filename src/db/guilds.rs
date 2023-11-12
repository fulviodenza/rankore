use std::sync::Arc;

use async_trait::async_trait;
use sqlx::{Error, FromRow, Pool, Postgres};

pub struct Guilds {
    pub pool: Pool<Postgres>,
}

#[derive(Debug, FromRow)]
pub struct Guild {
    #[sqlx(default)]
    pub id: Option<i64>,
    #[sqlx(default)]
    prefix: String,
    #[sqlx(default)]
    welcome_msg: Option<String>,
    #[sqlx(default)]
    voice_multiplier: i64,
    #[sqlx(default)]
    text_multiplier: i64,
}

impl Default for Guild {
    fn default() -> Self {
        Self {
            id: Some(0),
            prefix: "!".to_string(),
            welcome_msg: Some("Welcome!".to_string()),
            voice_multiplier: 1,
            text_multiplier: 1,
        }
    }
}

#[async_trait]
pub trait GuildRepo {
    async fn new(pool: &Pool<Postgres>) -> Arc<Self>;
    async fn set_prefix(&self, guild_id: i64, prefix: &str);
    async fn get_prefix(&self, guild_id: i64) -> String;
    async fn set_welcome_msg(&self, guild_id: i64, welcome_msg: &str);
    async fn set_voice_multiplier(
        &self,
        guild_id: i64,
        voice_multiplier: i64,
    ) -> Result<bool, Error>;
    async fn get_voice_multiplier(&self, guild_id: i64) -> Result<i64, Error>;
    async fn set_text_multiplier(&self, guild_id: i64, multiplier: i64) -> Result<bool, Error>;
    async fn get_text_multiplier(&self, guild_id: i64) -> Result<i64, Error>;
    async fn guilds(&self) -> Result<Vec<Guild>, Error>;
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
            guild_id
        )
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(_) => {}
            Err(_) => {
                let _ = sqlx::query!(
                    "INSERT into guilds(id, prefix, welcome_msg) values ($1, $2, $3)",
                    guild_id,
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
            guild_id
        )
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(_) => {}
            Err(_) => {
                let _ = sqlx::query!(
                    "INSERT into guilds(id, prefix, welcome_msg) values ($1, $2, $3)",
                    guild_id,
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
                    guild_id,
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
    async fn set_voice_multiplier(
        &self,
        guild_id: i64,
        voice_multiplier: i64,
    ) -> Result<bool, Error> {
        let result = sqlx::query!(
            "UPDATE guilds SET voice_multiplier = $1 WHERE id = $2",
            voice_multiplier,
            guild_id
        )
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => return Ok(true),
            Err(_) => return Ok(false),
        };
    }
    async fn get_voice_multiplier(&self, guild_id: i64) -> Result<i64, Error> {
        let result = sqlx::query_as!(Guild, "select * FROM guilds WHERE id = $1", guild_id)
            .fetch_one(&self.pool)
            .await;

        match result {
            Ok(guild) => {
                println!("got something");
                return Ok(guild.voice_multiplier);
            }
            Err(_) => {
                let _ = sqlx::query!(
                    "INSERT into guilds(id, prefix, welcome_msg, voice_multiplier) values ($1, $2, $3, $4)",
                    guild_id,
                    "!",
                    "",
                    1,
                )
                .execute(&self.pool)
                .await;
                return Ok(1);
            }
        };
    }

    async fn set_text_multiplier(&self, guild_id: i64, multiplier: i64) -> Result<bool, Error> {
        let result = sqlx::query!(
            "UPDATE guilds SET text_multiplier = $1 WHERE id = $2",
            multiplier,
            guild_id
        )
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => return Ok(true),
            Err(_) => return Ok(false),
        };
    }
    async fn get_text_multiplier(&self, guild_id: i64) -> Result<i64, Error> {
        let result = sqlx::query_as!(Guild, "select * FROM guilds WHERE id = $1", guild_id)
            .fetch_one(&self.pool)
            .await;

        match result {
            Ok(guild) => {
                return Ok(guild.text_multiplier);
            }
            Err(_) => {
                let _ = sqlx::query!(
                    "INSERT into guilds(id, prefix, welcome_msg, voice_multiplier, text_multiplier) values ($1, $2, $3, $4, $5)",
                    guild_id,
                    "!",
                    "",
                    1,
                    1,
                )
                .execute(&self.pool)
                .await;
                return Ok(1);
            }
        };
    }
    async fn guilds(&self) -> Result<Vec<Guild>, Error> {
        let result = sqlx::query_as!(Guild, "select * FROM guilds")
            .fetch_all(&self.pool)
            .await;

        match result {
            Ok(res) => {
                return Ok(res);
            }
            Err(e) => {
                return Err(e);
            }
        };
    }
}
