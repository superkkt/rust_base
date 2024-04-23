use crate::core::{CreateUserParams, DatabaseClient, DatabaseTransaction, User};
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub struct Controller {
    db: Box<dyn DatabaseClient + Send + Sync>,
}

impl Controller {
    pub fn new(db: Box<dyn DatabaseClient + Send + Sync>) -> Self {
        Self { db }
    }
}

impl Controller {
    pub async fn create_user(&self, params: CreateUserParams) -> Result<User> {
        let callback = move |tx: Arc<Box<dyn DatabaseTransaction + Send + Sync>>| {
            let params = params.clone();

            let fut = async move {
                let _ = tx.create_user(params).await?;
                Ok(())
            };
            Box::pin(fut) as Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send>>
        };
        let callback = Box::new(callback);
        self.db.invoke(callback).await?;

        // TODO: receive and return.

        Ok(User {
            id: 0,
            username: String::from(""),
            password: String::from(""),
            address: String::from(""),
            age: 10,
        })
    }

    pub async fn remove_user(&self, id: u64) -> Result<()> {
        Ok(())
    }
}
