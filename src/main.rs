mod configuration;
mod database;
mod entity;
mod logger;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let config = configuration::load().context("failed to load the configuration")?;
    init_logger(config.log.level);
    log::debug!("Hello World!");
    Ok(())
}

fn init_logger(level: log::LevelFilter) {
    log::set_boxed_logger(Box::new(logger::Logger)).unwrap();
    log::set_max_level(level);
}
