use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::{mpsc::UnboundedReceiver, RwLock};

use super::users::User;

pub enum UserEvents {
    Joined(u64),
    Left(u64),
    _SentText(u64),
}

#[async_trait]
pub trait UserObserver {
    async fn notify(
        mut rx: UnboundedReceiver<UserEvents>,
        user_lock: Arc<RwLock<HashMap<u64, User>>>,
    );
}
