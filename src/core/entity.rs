use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;
use std::fmt::Debug;

#[async_trait]
pub trait DatabaseTransaction: Debug {
    async fn begin(&self) -> Result<u64>;
    async fn commit(&self, tx_id: u64) -> Result<()>;
    async fn rollback(&self, tx_id: u64) -> Result<()>;
    async fn is_deadlock(&self, tx_id: u64) -> Result<bool>;
    async fn create_user<T>(&self, tx_id: u64, params: T) -> Result<User>
    where
        T: Into<CreateUserParams> + Send;
    async fn remove_user(&self, tx_id: u64, id: u64) -> Result<()>;
}

#[derive(Debug)]
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
