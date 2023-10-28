use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedReceiver;

pub enum UserEvents {
    Joined(i64, String),
    Left(i64),
    SentText(i64, String),
}

#[async_trait]
pub trait UserObserver {
    async fn notify(&self, mut rx: UnboundedReceiver<UserEvents>);
}
