use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

#[async_trait]
pub trait DatabaseClient {
    async fn invoke(
        &self,
        callback: Box<
            dyn for<'a> FnMut(
                    &'a mut (dyn DatabaseTransaction + Send + Sync),
                ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
                + Send,
        >,
    ) -> Result<()>;
}

#[async_trait]
pub trait DatabaseTransaction: Debug {
    async fn create_user(&mut self, params: CreateUserParams) -> Result<User>;
    async fn remove_user(&mut self, id: u64) -> Result<()>;
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
