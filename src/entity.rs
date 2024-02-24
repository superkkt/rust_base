use anyhow::Result;
use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;

#[async_trait]
pub trait DatabaseClient {
    async fn invoke<C>(&self, callback: C) -> Result<()>
    where
        C: FnMut(&dyn DatabaseTransaction) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send;
}

#[async_trait]
pub trait DatabaseTransaction {
    async fn remove_user(&self, id: u64) -> Result<()>;
}
