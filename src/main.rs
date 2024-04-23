mod configuration;
mod core;
mod database;
mod logger;
mod server;

use anyhow::{Context, Result};
use core::Controller;
use core::DatabaseClient;
use database::mysql;
use server::http;

#[tokio::main]
async fn main() -> Result<()> {
    let config = configuration::load().context("failed to load the configuration")?;
    init_logger(config.log.level);
    let db = init_database(config.database);
    let controller = Controller::new(db);
    http::serve(
        controller,
        config.http.port,
        &config.http.tls_cert_file,
        &config.http.tls_key_file,
    )
    .await
    .context("failed to serve HTTP")?;
    Ok(())
}

fn init_logger(level: log::LevelFilter) {
    log::set_boxed_logger(Box::new(logger::Logger)).unwrap();
    log::set_max_level(level);
}

fn init_database(config: configuration::Database) -> Box<dyn DatabaseClient + Send + Sync> {
    let c = database::Configuration {
        host: config.host,
        port: config.port,
        username: config.username,
        password: config.password,
        name: config.name,
    };

    Box::new(mysql::Client::new(c))
}
