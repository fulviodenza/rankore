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

#[derive(Clone, Copy, Debug, Default)]
pub struct AntiSpamSettings {
    pub min_msg_length: i32,
    pub cooldown_secs: i32,
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
    async fn set_decay_rate(&self, guild_id: i64, pct: i32) -> Result<(), Error>;
    async fn set_min_msg_length(&self, guild_id: i64, len: i32) -> Result<(), Error>;
    async fn set_msg_cooldown(&self, guild_id: i64, secs: i32) -> Result<(), Error>;
    async fn get_anti_spam(&self, guild_id: i64) -> AntiSpamSettings;
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

    async fn set_decay_rate(&self, guild_id: i64, pct: i32) -> Result<(), Error> {
        if !(0..=100).contains(&pct) {
            return Err(Error::Protocol("decay rate must be between 0 and 100".into()));
        }
        sqlx::query!(
            "INSERT INTO guilds (id, prefix, welcome_msg, voice_multiplier, text_multiplier, decay_per_day_pct) \
             VALUES ($1, '!', '', 1, 1, $2) \
             ON CONFLICT (id) DO UPDATE SET decay_per_day_pct = EXCLUDED.decay_per_day_pct",
            guild_id,
            pct,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn set_min_msg_length(&self, guild_id: i64, len: i32) -> Result<(), Error> {
        if len < 0 {
            return Err(Error::Protocol("min_msg_length cannot be negative".into()));
        }
        sqlx::query!(
            "INSERT INTO guilds (id, prefix, welcome_msg, voice_multiplier, text_multiplier, min_msg_length) \
             VALUES ($1, '!', '', 1, 1, $2) \
             ON CONFLICT (id) DO UPDATE SET min_msg_length = EXCLUDED.min_msg_length",
            guild_id,
            len,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn set_msg_cooldown(&self, guild_id: i64, secs: i32) -> Result<(), Error> {
        if secs < 0 {
            return Err(Error::Protocol("cooldown cannot be negative".into()));
        }
        sqlx::query!(
            "INSERT INTO guilds (id, prefix, welcome_msg, voice_multiplier, text_multiplier, msg_cooldown_secs) \
             VALUES ($1, '!', '', 1, 1, $2) \
             ON CONFLICT (id) DO UPDATE SET msg_cooldown_secs = EXCLUDED.msg_cooldown_secs",
            guild_id,
            secs,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn get_anti_spam(&self, guild_id: i64) -> AntiSpamSettings {
        match sqlx::query!(
            "SELECT min_msg_length, msg_cooldown_secs FROM guilds WHERE id = $1",
            guild_id
        )
        .fetch_optional(&self.pool)
        .await
        {
            Ok(Some(r)) => AntiSpamSettings {
                min_msg_length: r.min_msg_length,
                cooldown_secs: r.msg_cooldown_secs,
            },
            _ => AntiSpamSettings::default(),
        }
    }

    async fn guilds(&self) -> Result<Vec<Guild>, Error> {
        sqlx::query_as!(
            Guild,
            "SELECT id, prefix, welcome_msg, voice_multiplier, text_multiplier FROM guilds"
        )
        .fetch_all(&self.pool)
        .await
    }
}
