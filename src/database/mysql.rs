use crate::core::entity::{CreateUserParams, DatabaseTransaction, GetUserParams, User};
use crate::database::Configuration;

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::lock::Mutex;
use mysql_async::prelude::{FromRow, Queryable, StatementLike};
use mysql_async::{params, Params, Row, TxOpts};
use scopeguard::ScopeGuard;

#[derive(Debug)]
pub struct Client {
    counter: AtomicU64,
    pool: mysql_async::Pool,
    map: Mutex<HashMap<u64, Transaction>>,
}

#[derive(Debug)]
struct Transaction {
    id: u64,
    handle: mysql_async::Transaction<'static>,
    deadlock: bool,
}

impl Transaction {
    async fn exec_drop<'a: 'b, 'b, S, P>(&'a mut self, stmt: S, params: P) -> Result<()>
    where
        S: StatementLike + 'b,
        P: Into<Params> + Send + 'b,
    {
        let v = self.handle.exec_drop(stmt, params).await;
        if v.is_ok() {
            self.deadlock = false;
            return Ok(v?);
        }
        Ok(self.process_error(v)?)
    }

    async fn exec_map<'a: 'b, 'b, T, S, P, U, F>(
        &'a mut self,
        stmt: S,
        params: P,
        f: F,
    ) -> Result<Vec<U>>
    where
        S: StatementLike + 'b,
        P: Into<Params> + Send + 'b,
        T: FromRow + Send + 'static,
        F: FnMut(T) -> U + Send + 'a,
        U: Send + 'a,
    {
        let v = self.handle.exec_map(stmt, params, f).await;
        if v.is_ok() {
            self.deadlock = false;
            return Ok(v?);
        }
        Ok(self.process_error(v)?)
    }

    fn process_error<T>(
        &mut self,
        result: Result<T, mysql_async::Error>,
    ) -> Result<T, mysql_async::Error> {
        let err = result.err().unwrap();
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
        log::debug!("get_transaction invoked: tx_id = {tx_id}");

        loop {
            match self.map.try_lock() {
                None => std::thread::yield_now(),
                Some(mut map) => return map.remove(&tx_id),
            }
        }
    }

    fn get_transaction_guard(
        &self,
        tx_id: u64,
    ) -> Result<ScopeGuard<Transaction, Box<dyn FnOnce(Transaction) + Send + '_>>> {
        log::debug!("get_transaction_guard invoked: tx_id = {tx_id}");

        match self.get_transaction(tx_id) {
            None => Err(anyhow!("unknown transaction id: {tx_id}")),
            Some(tx) => Ok(scopeguard::guard(
                tx,
                Box::new(move |tx| {
                    log::debug!("scopeguard invoked: tx_id = {tx_id}");
                    self.put_transaction(tx);
                }),
            )),
        }
    }

    fn put_transaction(&self, tx: Transaction) {
        log::debug!("put_transaction invoked: tx_id = {}", tx.id);

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
        log::debug!("begin invoked");
        let tx = self
            .pool
            .start_transaction(TxOpts::default())
            .await
            .context("failed to start a database transaction")?;
        let tx_id = self.counter.fetch_add(1, Ordering::SeqCst);
        self.put_transaction(Transaction {
            id: tx_id,
            handle: tx,
            deadlock: false,
        });
        Ok(tx_id)
    }

    async fn commit(&self, tx_id: u64) -> Result<()> {
        log::debug!("commit invoked: tx_id = {tx_id}");
        match self.get_transaction(tx_id) {
            Some(tx) => Ok(tx.handle.commit().await?),
            None => Err(anyhow!("unknown transaction id: {tx_id}")),
        }
    }

    async fn rollback(&self, tx_id: u64) -> Result<()> {
        log::debug!("rollback invoked: tx_id = {tx_id}");
        match self.get_transaction(tx_id) {
            Some(tx) => Ok(tx.handle.rollback().await?),
            None => Err(anyhow!("unknown transaction id: {tx_id}")),
        }
    }

    async fn is_deadlock(&self, tx_id: u64) -> Result<bool> {
        log::debug!("is_deadlock invoked: tx_id = {tx_id}");
        match self.get_transaction(tx_id) {
            Some(tx) => {
                let tx = scopeguard::guard(tx, |tx| {
                    self.put_transaction(tx);
                });
                let deadlock = tx.deadlock;
                Ok(deadlock)
            }
            None => Err(anyhow!("unknown transaction id: {tx_id}")),
        }
    }

    async fn create_user<T>(&self, tx_id: u64, params: T) -> Result<User>
    where
        T: Into<CreateUserParams> + Send,
    {
        let params = params.into();
        log::debug!("create_user: tx_id = {tx_id}, params = {params:?}");

        let mut tx = self.get_transaction_guard(tx_id)?;
        let query = "INSERT INTO `users` (`username`, `password`, `age`, `address`) \
                     VALUES (:username, :password, :age, :address)";
        tx.exec_drop(
            query,
            params! {
                "username" => &params.username,
                "password" => &params.password,
                "age" => params.age,
                "address" => &params.address,
            },
        )
        .await?;
        let id = tx
            .handle
            .last_insert_id()
            .expect("AUTO-INCREMENTed last inserted ID should exist");

        Ok(User {
            id,
            username: params.username,
            password: params.password,
            age: params.age,
            address: params.address,
        })
    }

    async fn get_user<T>(&self, tx_id: u64, params: T) -> Result<Option<User>>
    where
        T: Into<GetUserParams> + Send,
    {
        let params = params.into();
        log::debug!("get_user: tx_id = {}, id = {}", tx_id, params.id);

        let mut tx = self.get_transaction_guard(tx_id)?;
        let query =
            r"SELECT `id`, `username`, `password`, `age`, `address` FROM `users` WHERE `id` = :id";
        let mut users = tx
            .exec_map(
                query,
                params! {
                    "id" => params.id,
                },
                |row: Row| User {
                    id: row.get("id").unwrap(),
                    username: row.get("username").unwrap(),
                    password: row.get("password").unwrap(),
                    age: row.get("age").unwrap(),
                    address: row.get("address").unwrap(),
                },
            )
            .await?;
        if users.is_empty() {
            Ok(None)
        } else {
            Ok(Some(users.remove(0)))
        }
    }
}
