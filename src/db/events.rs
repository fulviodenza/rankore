use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::{mpsc::UnboundedReceiver, RwLock};

use super::users::User;

pub enum UserEvents {
    Joined(u64, String),
    Left(u64),
    SentText(u64, String),
}

#[async_trait]
pub trait UserObserver {
    async fn notify(
        mut rx: UnboundedReceiver<UserEvents>,
        user_lock: Arc<RwLock<HashMap<u64, User>>>,
    );
}
