use rust_base::configuration;
use rust_base::core::{Controller, DatabaseTransaction};
use rust_base::database;
use rust_base::database::dummy::Dummy;
use rust_base::database::mysql;
use rust_base::logger;
use rust_base::server::http;

use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let config = configuration::load().context("failed to load the configuration")?;
    init_logger(config.log.level);
    init_http_server(config).await?;
    Ok(())
}

fn init_logger(level: log::LevelFilter) {
    log::set_boxed_logger(Box::new(logger::Logger)).unwrap();
    log::set_max_level(level);
}

fn init_mysql(config: configuration::Database) -> impl DatabaseTransaction + Send + Sync {
    let c = database::Configuration {
        host: config.host,
        port: config.port,
        username: config.username,
        password: config.password,
        name: config.name,
    };

    mysql::Client::new(c)
}

fn init_dummy() -> impl DatabaseTransaction + Send + Sync {
    Dummy {}
}

async fn init_http_server(config: configuration::Configuration) -> Result<()> {
    log::debug!("starting HTTP server...");

    if config.database.driver.to_lowercase() == "mysql" {
        http::serve(
            Controller::new(init_mysql(config.database)),
            config.http.port,
            &config.http.tls_cert_file,
            &config.http.tls_key_file,
        )
        .await
        .context("failed to serve HTTP service")?;
    } else {
        http::serve(
            Controller::new(init_dummy()),
            config.http.port,
            &config.http.tls_cert_file,
            &config.http.tls_key_file,
        )
        .await
        .context("failed to serve HTTP service")?;
    }

    Ok(())
}
