use std::sync::Arc;

use async_trait::async_trait;
use sqlx::{Error, FromRow, Pool, Postgres};

pub struct Guilds {
    pub pool: Pool<Postgres>,
}

#[allow(dead_code)]
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
            id: None,
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
    async fn set_prefix(&self, guild_id: i64, prefix: &str) -> Result<(), Error>;
    async fn get_prefix(&self, guild_id: i64) -> String;
    async fn set_welcome_msg(&self, guild_id: i64, welcome_msg: &str) -> Result<(), Error>;
    async fn get_welcome_msg(&self, guild_id: i64) -> Option<String>;
    async fn set_voice_multiplier(&self, guild_id: i64, multiplier: i64) -> Result<(), Error>;
    async fn get_voice_multiplier(&self, guild_id: i64) -> i64;
    async fn set_text_multiplier(&self, guild_id: i64, multiplier: i64) -> Result<(), Error>;
    async fn get_text_multiplier(&self, guild_id: i64) -> i64;
    async fn guilds(&self) -> Result<Vec<Guild>, Error>;
}

#[async_trait]
impl GuildRepo for Guilds {
    async fn new(pool: &Pool<Postgres>) -> Arc<Self> {
        Arc::new(Guilds { pool: pool.clone() })
    }

    async fn set_prefix(&self, guild_id: i64, prefix: &str) -> Result<(), Error> {
        sqlx::query!(
            "INSERT INTO guilds (id, prefix, welcome_msg, voice_multiplier, text_multiplier) \
             VALUES ($1, $2, '', 1, 1) \
             ON CONFLICT (id) DO UPDATE SET prefix = EXCLUDED.prefix",
            guild_id,
            prefix,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn get_prefix(&self, guild_id: i64) -> String {
        match sqlx::query_scalar!("SELECT prefix FROM guilds WHERE id = $1", guild_id)
            .fetch_optional(&self.pool)
            .await
        {
            Ok(Some(p)) => p,
            _ => "!".to_string(),
        }
    }

    async fn set_welcome_msg(&self, guild_id: i64, welcome_msg: &str) -> Result<(), Error> {
        sqlx::query!(
            "INSERT INTO guilds (id, prefix, welcome_msg, voice_multiplier, text_multiplier) \
             VALUES ($1, '!', $2, 1, 1) \
             ON CONFLICT (id) DO UPDATE SET welcome_msg = EXCLUDED.welcome_msg",
            guild_id,
            welcome_msg,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn get_welcome_msg(&self, guild_id: i64) -> Option<String> {
        match sqlx::query_scalar!("SELECT welcome_msg FROM guilds WHERE id = $1", guild_id)
            .fetch_optional(&self.pool)
            .await
        {
            Ok(Some(msg)) => msg,
            _ => None,
        }
    }

    async fn set_voice_multiplier(&self, guild_id: i64, multiplier: i64) -> Result<(), Error> {
        if multiplier <= 0 {
            return Err(Error::Protocol("multiplier must be positive".into()));
        }
        sqlx::query!(
            "INSERT INTO guilds (id, prefix, welcome_msg, voice_multiplier, text_multiplier) \
             VALUES ($1, '!', '', $2, 1) \
             ON CONFLICT (id) DO UPDATE SET voice_multiplier = EXCLUDED.voice_multiplier",
            guild_id,
            multiplier,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn get_voice_multiplier(&self, guild_id: i64) -> i64 {
        sqlx::query_scalar!("SELECT voice_multiplier FROM guilds WHERE id = $1", guild_id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
            .unwrap_or(1)
    }

    async fn set_text_multiplier(&self, guild_id: i64, multiplier: i64) -> Result<(), Error> {
        if multiplier <= 0 {
            return Err(Error::Protocol("multiplier must be positive".into()));
        }
        sqlx::query!(
            "INSERT INTO guilds (id, prefix, welcome_msg, voice_multiplier, text_multiplier) \
             VALUES ($1, '!', '', 1, $2) \
             ON CONFLICT (id) DO UPDATE SET text_multiplier = EXCLUDED.text_multiplier",
            guild_id,
            multiplier,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn get_text_multiplier(&self, guild_id: i64) -> i64 {
        sqlx::query_scalar!("SELECT text_multiplier FROM guilds WHERE id = $1", guild_id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
            .unwrap_or(1)
    }

    async fn guilds(&self) -> Result<Vec<Guild>, Error> {
        sqlx::query_as!(Guild, "SELECT * FROM guilds")
            .fetch_all(&self.pool)
            .await
    }
}
