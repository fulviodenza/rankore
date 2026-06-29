use std::sync::Arc;

use serenity::all::{GuildId, Http, RoleId, UserId};
use sqlx::{Pool, Postgres};

use crate::db::roles::{Roles, RolesRepo};

pub struct RoleSyncer {
    pub http: Arc<Http>,
    pub pool: Pool<Postgres>,
    pub roles: Arc<Roles>,
}

impl RoleSyncer {
    pub fn new(http: Arc<Http>, pool: Pool<Postgres>, roles: Arc<Roles>) -> Arc<Self> {
        Arc::new(Self { http, pool, roles })
    }

    /// Look up the user's current score for the guild and grant any threshold
    /// roles they have earned but don't yet have. Additive only — roles are
    /// not removed when a score drops below a threshold (e.g. after reset).
    pub async fn sync(self: &Arc<Self>, user_id: i64, guild_id: i64) {
        let score = match sqlx::query_scalar!(
            "SELECT score FROM users WHERE id = $1 AND guild_id = $2",
            user_id,
            guild_id,
        )
        .fetch_optional(&self.pool)
        .await
        {
            Ok(Some(s)) => s,
            Ok(None) => return,
            Err(e) => {
                eprintln!("[role_sync] read score: {e}");
                return;
            }
        };

        let earned = match self.roles.earned_roles(guild_id, score).await {
            Ok(r) if !r.is_empty() => r,
            Ok(_) => return,
            Err(e) => {
                eprintln!("[role_sync] earned_roles: {e}");
                return;
            }
        };

        let gid = GuildId::new(guild_id as u64);
        let uid = UserId::new(user_id as u64);
        let member = match self.http.get_member(gid, uid).await {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[role_sync] get_member: {e}");
                return;
            }
        };
        let current: std::collections::HashSet<u64> =
            member.roles.iter().map(|r| r.get()).collect();
        for role_id in earned {
            let rid = role_id as u64;
            if current.contains(&rid) {
                continue;
            }
            if let Err(e) = self
                .http
                .add_member_role(gid, uid, RoleId::new(rid), Some("rankore score threshold"))
                .await
            {
                eprintln!("[role_sync] add_member_role({rid}): {e}");
            }
        }
    }
}
