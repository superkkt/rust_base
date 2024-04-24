use crate::core::{CreateUserParams, DatabaseClient, DatabaseTransaction, User};
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;

pub struct Controller {
    db: Box<dyn DatabaseClient + Send + Sync>,
}

impl Controller {
    pub fn new(db: Box<dyn DatabaseClient + Send + Sync>) -> Self {
        Self { db }
    }
}

fn constrain_callback<F>(f: F) -> F
where
    F: for<'a> FnMut(
            &'a mut (dyn DatabaseTransaction + Send + Sync),
        ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
        + Send,
{
    f
}

impl Controller {
    pub async fn create_user(&self, params: CreateUserParams) -> Result<User> {
        log::debug!("create_user invoked");
        let callback = constrain_callback(move |tx: &mut (dyn DatabaseTransaction + Send + Sync)| {
            log::debug!("callback invoked");
            let params = params.clone();

            let fut = async move {
                let _ = tx.create_user(params).await?;
                Ok(())
            };
            Box::pin(fut) as Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + Send>>
        });
        self.db.invoke(Box::new(callback)).await?;

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
