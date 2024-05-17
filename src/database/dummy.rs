use crate::core::entity::{CreateUserParams, DatabaseTransaction, GetUserParams, User};

use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug)]
pub struct Dummy;

#[async_trait]
impl DatabaseTransaction for Dummy {
    async fn begin(&self) -> Result<u64> {
        Ok(0)
    }

    async fn commit(&self, _tx_id: u64) -> Result<()> {
        Ok(())
    }

    async fn rollback(&self, _tx_id: u64) -> Result<()> {
        Ok(())
    }

    async fn is_deadlock(&self, _tx_id: u64) -> Result<bool> {
        Ok(true)
    }

    async fn create_user<T>(&self, _tx_id: u64, _params: T) -> Result<User>
    where
        T: Into<CreateUserParams> + Send,
    {
        Ok(User {
            id: 0,
            username: String::from(""),
            password: String::from(""),
            address: String::from(""),
            age: 10,
        })
    }

    async fn get_user<T>(&self, _tx_id: u64, _id: T) -> Result<Option<User>>
    where
        T: Into<GetUserParams> + Send,
    {
        Ok(Some(User {
            id: 0,
            username: String::from(""),
            password: String::from(""),
            address: String::from(""),
            age: 10,
        }))
    }
}
