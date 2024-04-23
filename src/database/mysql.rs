use crate::core::{CreateUserParams, DatabaseClient, DatabaseTransaction, User};
use crate::database::Configuration;
use anyhow::{Context, Result};
use async_trait::async_trait;
use mysql_async::TxOpts;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

pub struct Client {
    pool: mysql_async::Pool,
}

impl Client {
    pub fn new(config: Configuration) -> Self {
        let builder = mysql_async::OptsBuilder::default()
            .ip_or_hostname(config.host)
            .tcp_port(config.port)
            .user(Some(config.username))
            .pass(Some(config.password))
            .db_name(Some(config.name))
            .tcp_keepalive(Some(10000_u32))
            .conn_ttl(Some(Duration::from_secs(60)))
            .wait_timeout(Some(60 * 10));
        let opts = mysql_async::Opts::from(builder);
        let pool = mysql_async::Pool::new(opts);

        Self { pool }
    }
}

#[derive(Debug)]
struct Transaction<'a> {
    handle: mysql_async::Transaction<'a>,
}

const MYSQL_DEADLOCK_ERROR_CODE: u16 = 1213;
const MAX_DEADLOCK_RETRY: usize = 5;

fn get_mysql_error_code(err: &anyhow::Error) -> Option<u16> {
    let mysql_err = err.downcast_ref::<mysql_async::Error>();
    if mysql_err.is_none() {
        return None;
    }
    if let mysql_async::Error::Server(server_err) = mysql_err.unwrap() {
        return Some(server_err.code);
    }
    None
}

#[async_trait]
impl DatabaseClient for Client {
    async fn invoke(
        &self,
        mut callback: Box<
            dyn FnMut(
                    Arc<Box<dyn DatabaseTransaction + Send + Sync>>,
                ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>
                + Send,
        >,
    ) -> Result<()> {
        let mut deadlock_count: usize = 0;

        loop {
            let tx = self
                .pool
                .start_transaction(TxOpts::default())
                .await
                .context("failed to start a database transaction")?;
            let tx = Transaction { handle: tx };
            let tx = Arc::new(Box::new(tx) as Box<(dyn DatabaseTransaction + Send + Sync)>);

            let result = callback(tx.clone()).await;

            let tx: Box<dyn Any> = Box::new(tx);
            let tx = tx.downcast::<Arc<Box<Transaction>>>().expect("expect Transaction type");
            let tx = Arc::try_unwrap(*tx).unwrap();

            if result.is_ok() {
                tx.handle
                    .commit()
                    .await
                    .context("failed to commit a database transaction")?;
                return Ok(());
            }

            let err = result.unwrap_err();
            let err_code = get_mysql_error_code(&err);
            if err_code.is_some()
                && err_code.unwrap() == MYSQL_DEADLOCK_ERROR_CODE
                && deadlock_count < MAX_DEADLOCK_RETRY
            {
                deadlock_count += 1;
                tx.handle
                    .rollback()
                    .await
                    .context("failed to rollback a database transaction")?;
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }

            return Err(err.context("failed to call the callback handler"));
        }
    }
}

#[async_trait]
impl DatabaseTransaction for Transaction<'_> {
    async fn create_user(&self, params: CreateUserParams) -> Result<User> {
        log::debug!("create_user: params = {:?}", params);
        // TODO: DB query.
        Ok(User {
            id: 0,
            username: String::from(""),
            password: String::from(""),
            age: 0,
            address: String::from(""),
        })
    }

    async fn remove_user(&self, id: u64) -> Result<()> {
        log::debug!("remove_user: id = {id}");
        // TODO: DB query.
        Ok(())
    }
}
