use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedReceiver;

pub enum UserEvents {
    Joined(i64, String, bool),
    Left(i64),
    SentText(i64, String, bool),
}

#[async_trait]
pub trait Observer {
    async fn notify(&self, mut rx: UnboundedReceiver<UserEvents>);
}
