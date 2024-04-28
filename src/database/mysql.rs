use crate::core::entity::CreateUserParams;
use crate::core::{DatabaseTransaction, User};
use crate::database::Configuration;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::lock::Mutex;
use mysql_async::prelude::{Queryable, StatementLike};
use mysql_async::{params, Params, TxOpts};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[derive(Debug)]
pub struct Client {
    counter: AtomicU64,
    pool: mysql_async::Pool,
    map: Mutex<HashMap<u64, Transaction>>,
}

#[derive(Debug)]
struct Transaction {
    id: u64,
    tx: mysql_async::Transaction<'static>,
    deadlock: bool,
}

impl Transaction {
    async fn exec_drop<'a: 'b, 'b, S, P>(&'a mut self, stmt: S, params: P) -> Result<()>
    where
        S: StatementLike + 'b,
        P: Into<Params> + Send + 'b,
    {
        let v = self.tx.exec_drop(stmt, params).await;
        if v.is_ok() {
            self.deadlock = false;
            return Ok(v?);
        }
        self.process_error(v.unwrap_err())
    }

    fn process_error(&mut self, err: mysql_async::Error) -> Result<()> {
        let err_code = get_mysql_error_code(&err);
        if err_code.is_some() && err_code.unwrap() == MYSQL_DEADLOCK_ERROR_CODE {
            self.deadlock = true;
        } else {
            self.deadlock = false;
        }

        Err(err.into())
    }
}

impl Client {
    pub fn new(config: Configuration) -> Self {
        let counter = AtomicU64::new(0);
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
        let map = Mutex::new(HashMap::new());

        Self { counter, pool, map }
    }

    fn get_transaction(&self, tx_id: u64) -> Option<Transaction> {
        loop {
            match self.map.try_lock() {
                None => continue,
                Some(mut map) => return map.remove(&tx_id),
            }
        }
    }

    fn put_transaction(&self, tx: Transaction) {
        loop {
            match self.map.try_lock() {
                None => continue,
                Some(mut map) => {
                    map.insert(tx.id, tx);
                    return;
                }
            }
        }
    }
}

const MYSQL_DEADLOCK_ERROR_CODE: u16 = 1213;

fn get_mysql_error_code(err: &mysql_async::Error) -> Option<u16> {
    if let mysql_async::Error::Server(server_err) = err {
        return Some(server_err.code);
    }
    None
}

#[async_trait]
impl DatabaseTransaction for Client {
    async fn begin(&self) -> Result<u64> {
        let tx = self
            .pool
            .start_transaction(TxOpts::default())
            .await
            .context("failed to start a database transaction")?;
        let tx_id = self.counter.fetch_add(1, Ordering::SeqCst);
        self.put_transaction(Transaction {
            id: tx_id,
            tx,
            deadlock: false,
        });
        Ok(tx_id)
    }

    async fn commit(&self, tx_id: u64) -> Result<()> {
        match self.get_transaction(tx_id) {
            Some(tx) => Ok(tx.tx.commit().await?),
            None => Err(anyhow!("unknown transaction id: {}", tx_id)),
        }
    }

    async fn rollback(&self, tx_id: u64) -> Result<()> {
        match self.get_transaction(tx_id) {
            Some(tx) => Ok(tx.tx.rollback().await?),
            None => Err(anyhow!("unknown transaction id: {}", tx_id)),
        }
    }

    async fn is_deadlock(&self, tx_id: u64) -> Result<bool> {
        match self.get_transaction(tx_id) {
            Some(tx) => {
                let tx = scopeguard::guard(tx, |tx| {
                    self.put_transaction(tx);
                });
                let deadlock = tx.deadlock;
                Ok(deadlock)
            }
            None => Err(anyhow!("unknown transaction id: {}", tx_id)),
        }
    }

    async fn create_user<T>(&self, tx_id: u64, params: T) -> Result<User>
    where
        T: Into<CreateUserParams> + Send,
    {
        let params = params.into();
        log::debug!("create_user: params = {:?}", params);

        match self.get_transaction(tx_id) {
            Some(tx) => {
                let mut tx = scopeguard::guard(tx, |tx| {
                    self.put_transaction(tx);
                });
                // TODO: DB query.
                let insert_query = r"INSERT INTO test (a, b) VALUES (:value1, :value2)";
                let params = params! {
                    "value1" => "Test Data 1",
                    "value2" => "Test Data 2",
                };
                tx.exec_drop(insert_query, params).await?;

                Ok(User {
                    id: 18,
                    username: String::from(""),
                    password: String::from(""),
                    age: 18,
                    address: String::from(""),
                })
            }
            None => Err(anyhow!("unknown transaction id: {}", tx_id)),
        }
    }

    async fn remove_user(&self, tx_id: u64, id: u64) -> Result<()> {
        log::debug!("remove_user: id = {id}");

        // TODO: get tx from the map.

        // TODO: DB query.
        Ok(())
    }
}
