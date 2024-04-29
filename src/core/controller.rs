use crate::core::entity::CreateUserParams as EntityCreateUserParams;
use crate::core::entity::GetUserParams as EntityGetUserParams;
use crate::core::{DatabaseTransaction, User};
use anyhow::{Context, Result};
use futures::future::BoxFuture;
use std::fmt::Debug;
use std::sync::mpsc;
use std::time::Duration;

pub struct Controller<T: DatabaseTransaction + Send + Sync> {
    db: T,
}

fn constrain_callback<F, T>(f: F) -> F
where
    F: for<'a> FnMut(u64, &'a T) -> BoxFuture<'a, Result<()>> + Send,
    T: DatabaseTransaction + Send + Sync,
{
    f
}

#[derive(Debug, Clone)]
pub struct CreateUserParams {
    pub username: String,
    pub password: String,
    pub age: u16,
    pub address: String,
}

impl Into<EntityCreateUserParams> for CreateUserParams {
    fn into(self) -> EntityCreateUserParams {
        EntityCreateUserParams {
            username: self.username,
            password: self.password,
            age: self.age,
            address: self.address,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GetUserParams {
    pub id: u64,
}

impl Into<EntityGetUserParams> for GetUserParams {
    fn into(self) -> EntityGetUserParams {
        EntityGetUserParams { id: self.id }
    }
}

const MAX_DEADLOCK_RETRY: usize = 5;

impl<T: DatabaseTransaction + Send + Sync> Controller<T> {
    pub fn new(db: T) -> Self {
        Self { db }
    }

    async fn invoke(
        &self,
        mut callback: Box<dyn for<'a> FnMut(u64, &'a T) -> BoxFuture<'a, Result<()>> + Send>,
    ) -> Result<()> {
        let mut deadlock_count: usize = 0;

        loop {
            let tx_id = self.db.begin().await?;

            match callback(tx_id, &self.db).await {
                Ok(_) => {
                    self.db
                        .commit(tx_id)
                        .await
                        .context("failed to commit a database transaction")?;
                    return Ok(());
                }
                Err(err) => {
                    let deadlock = self.db.is_deadlock(tx_id).await?;
                    self.db
                        .rollback(tx_id)
                        .await
                        .context("failed to rollback a database transaction")?;

                    if deadlock && deadlock_count < MAX_DEADLOCK_RETRY {
                        deadlock_count += 1;
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }

                    return Err(err.context("failed to call the callback handler"));
                }
            }
        }
    }

    pub async fn create_user<U>(&self, params: U) -> Result<User>
    where
        U: Into<CreateUserParams>,
    {
        let params = params.into();
        let (tx_chan, rx_chan) = mpsc::channel();

        let callback = constrain_callback(move |tx_id: u64, tx: &T| {
            log::debug!("callback invoked");
            // We need these clones because this callback is FnMut, which can be called
            // multiple times. Otherwise, only the very first call for this callback will
            // work.
            let params = params.clone();
            let tx_chan = tx_chan.clone();
            let fut = async move {
                let user = tx.create_user(tx_id, params).await?;
                tx_chan.send(user)?;
                Ok(())
            };
            Box::pin(fut) as BoxFuture<'_, Result<()>>
        });
        self.invoke(Box::new(callback)).await?;

        Ok(rx_chan.recv()?)
    }

    pub async fn get_user<U>(&self, params: U) -> Result<Option<User>>
    where
        U: Into<GetUserParams>,
    {
        let params = params.into();
        let (tx_chan, rx_chan) = mpsc::channel();

        let callback = constrain_callback(move |tx_id: u64, tx: &T| {
            log::debug!("callback invoked");
            // We need these clones because this callback is FnMut, which can be called
            // multiple times. Otherwise, only the very first call for this callback will
            // work.
            let params = params.clone();
            let tx_chan = tx_chan.clone();
            let fut = async move {
                let user = tx.get_user(tx_id, params).await?;
                tx_chan.send(user)?;
                Ok(())
            };
            Box::pin(fut) as BoxFuture<'_, Result<()>>
        });
        self.invoke(Box::new(callback)).await?;

        Ok(rx_chan.recv()?)
    }
}
