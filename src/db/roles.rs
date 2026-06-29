use std::sync::Arc;

use async_trait::async_trait;
use sqlx::{Error, Pool, Postgres};

pub struct Roles {
    pub pool: Pool<Postgres>,
}

#[derive(Clone, Debug)]
pub struct RoleThreshold {
    pub role_id: i64,
    pub score: i64,
}

#[async_trait]
pub trait RolesRepo {
    async fn new(pool: &Pool<Postgres>) -> Arc<Self>;
    async fn set_threshold(
        &self,
        guild_id: i64,
        role_id: i64,
        score: i64,
    ) -> Result<(), Error>;
    async fn remove_threshold(&self, guild_id: i64, role_id: i64) -> Result<(), Error>;
    async fn list_thresholds(&self, guild_id: i64) -> Result<Vec<RoleThreshold>, Error>;
    async fn earned_roles(
        &self,
        guild_id: i64,
        score: i64,
    ) -> Result<Vec<i64>, Error>;
}

#[async_trait]
impl RolesRepo for Roles {
    async fn new(pool: &Pool<Postgres>) -> Arc<Self> {
        Arc::new(Roles { pool: pool.clone() })
    }

    async fn set_threshold(
        &self,
        guild_id: i64,
        role_id: i64,
        score: i64,
    ) -> Result<(), Error> {
        sqlx::query!(
            "INSERT INTO role_thresholds (guild_id, role_id, score) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (guild_id, role_id) DO UPDATE SET score = EXCLUDED.score",
            guild_id,
            role_id,
            score,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn remove_threshold(&self, guild_id: i64, role_id: i64) -> Result<(), Error> {
        sqlx::query!(
            "DELETE FROM role_thresholds WHERE guild_id = $1 AND role_id = $2",
            guild_id,
            role_id,
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn list_thresholds(&self, guild_id: i64) -> Result<Vec<RoleThreshold>, Error> {
        let rows = sqlx::query!(
            "SELECT role_id, score FROM role_thresholds \
             WHERE guild_id = $1 ORDER BY score ASC",
            guild_id,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| RoleThreshold {
                role_id: r.role_id,
                score: r.score,
            })
            .collect())
    }

    async fn earned_roles(
        &self,
        guild_id: i64,
        score: i64,
    ) -> Result<Vec<i64>, Error> {
        let rows = sqlx::query!(
            "SELECT role_id FROM role_thresholds \
             WHERE guild_id = $1 AND score <= $2",
            guild_id,
            score,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.role_id).collect())
    }
}
