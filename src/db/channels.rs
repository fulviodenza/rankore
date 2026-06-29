use std::sync::Arc;

use async_trait::async_trait;
use sqlx::{Error, Pool, Postgres};

pub struct Channels {
    pub pool: Pool<Postgres>,
}

#[derive(Clone, Debug)]
pub struct ChannelMultiplier {
    pub channel_id: i64,
    pub text_multiplier: Option<i64>,
    pub voice_multiplier: Option<i64>,
}

#[async_trait]
pub trait ChannelsRepo {
    async fn new(pool: &Pool<Postgres>) -> Arc<Self>;
    async fn set_multiplier(
        &self,
        guild_id: i64,
        channel_id: i64,
        text: Option<i64>,
        voice: Option<i64>,
    ) -> Result<(), Error>;
    async fn clear_multiplier(&self, guild_id: i64, channel_id: i64) -> Result<(), Error>;
    async fn list(&self, guild_id: i64) -> Result<Vec<ChannelMultiplier>, Error>;
    async fn get_text(&self, guild_id: i64, channel_id: i64) -> Option<i64>;
    async fn get_voice(&self, guild_id: i64, channel_id: i64) -> Option<i64>;
    async fn exclude(&self, guild_id: i64, channel_id: i64) -> Result<(), Error>;
    async fn include(&self, guild_id: i64, channel_id: i64) -> Result<(), Error>;
    async fn is_excluded(&self, guild_id: i64, channel_id: i64) -> bool;
    async fn list_excluded(&self, guild_id: i64) -> Result<Vec<i64>, Error>;
}

#[async_trait]
impl ChannelsRepo for Channels {
    async fn new(pool: &Pool<Postgres>) -> Arc<Self> {
        Arc::new(Channels { pool: pool.clone() })
    }

    async fn set_multiplier(
        &self,
        guild_id: i64,
        channel_id: i64,
        text: Option<i64>,
        voice: Option<i64>,
    ) -> Result<(), Error> {
        // COALESCE preserves the existing column when the caller passes None
        // for one side and Some for the other.
        sqlx::query!(
            "INSERT INTO channel_multipliers (guild_id, channel_id, text_multiplier, voice_multiplier) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (guild_id, channel_id) DO UPDATE SET \
                 text_multiplier = COALESCE(EXCLUDED.text_multiplier, channel_multipliers.text_multiplier), \
                 voice_multiplier = COALESCE(EXCLUDED.voice_multiplier, channel_multipliers.voice_multiplier)",
            guild_id,
            channel_id,
            text,
            voice,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn clear_multiplier(&self, guild_id: i64, channel_id: i64) -> Result<(), Error> {
        sqlx::query!(
            "DELETE FROM channel_multipliers WHERE guild_id = $1 AND channel_id = $2",
            guild_id,
            channel_id,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn list(&self, guild_id: i64) -> Result<Vec<ChannelMultiplier>, Error> {
        let rows = sqlx::query!(
            "SELECT channel_id, text_multiplier, voice_multiplier \
             FROM channel_multipliers WHERE guild_id = $1 \
             ORDER BY channel_id",
            guild_id,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| ChannelMultiplier {
                channel_id: r.channel_id,
                text_multiplier: r.text_multiplier,
                voice_multiplier: r.voice_multiplier,
            })
            .collect())
    }

    async fn get_text(&self, guild_id: i64, channel_id: i64) -> Option<i64> {
        sqlx::query_scalar!(
            "SELECT text_multiplier FROM channel_multipliers \
             WHERE guild_id = $1 AND channel_id = $2",
            guild_id,
            channel_id,
        )
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .flatten()
    }

    async fn get_voice(&self, guild_id: i64, channel_id: i64) -> Option<i64> {
        sqlx::query_scalar!(
            "SELECT voice_multiplier FROM channel_multipliers \
             WHERE guild_id = $1 AND channel_id = $2",
            guild_id,
            channel_id,
        )
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .flatten()
    }

    async fn exclude(&self, guild_id: i64, channel_id: i64) -> Result<(), Error> {
        sqlx::query!(
            "INSERT INTO excluded_channels (guild_id, channel_id) VALUES ($1, $2) \
             ON CONFLICT DO NOTHING",
            guild_id,
            channel_id,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn include(&self, guild_id: i64, channel_id: i64) -> Result<(), Error> {
        sqlx::query!(
            "DELETE FROM excluded_channels WHERE guild_id = $1 AND channel_id = $2",
            guild_id,
            channel_id,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn is_excluded(&self, guild_id: i64, channel_id: i64) -> bool {
        sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM excluded_channels \
             WHERE guild_id = $1 AND channel_id = $2)",
            guild_id,
            channel_id,
        )
        .fetch_one(&self.pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(false)
    }

    async fn list_excluded(&self, guild_id: i64) -> Result<Vec<i64>, Error> {
        sqlx::query_scalar!(
            "SELECT channel_id FROM excluded_channels WHERE guild_id = $1 ORDER BY channel_id",
            guild_id,
        )
        .fetch_all(&self.pool)
        .await
    }
}
