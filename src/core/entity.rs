use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[async_trait]
pub trait DatabaseClient {
    async fn invoke(
        &self,
        callback: Box<
            dyn FnMut(
                    Arc<Box<dyn DatabaseTransaction + Send + Sync>>,
                ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>
                + Send,
        >,
    ) -> Result<()>;
}

#[async_trait]
pub trait DatabaseTransaction {
    async fn create_user(&self, params: CreateUserParams) -> Result<User>;
    async fn remove_user(&self, id: u64) -> Result<()>;
}

#[derive(Debug, Deserialize, Clone)]
pub struct CreateUserParams {
    pub username: String,
    pub password: String,
    pub age: u16,
    pub address: String,
}

#[derive(Debug, Serialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub password: String,
    pub age: u16,
    pub address: String,
}
