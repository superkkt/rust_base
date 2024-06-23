use std::fmt;
use std::fs::File;
use std::io::Read;

use anyhow::{Context, Result};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};

const DEFAULT_CONFIG_FILE_PATH: &str = "/etc/rust_base.yaml";

#[derive(Debug, Deserialize)]
pub struct Configuration {
    pub log: Log,
    pub database: Database,
    pub http: HTTP,
}

#[derive(Debug, Deserialize)]
pub struct Log {
    #[serde(deserialize_with = "deserialize_log_level")]
    pub level: log::LevelFilter,
}

#[derive(Debug, Deserialize)]
pub struct Database {
    pub driver: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct HTTP {
    pub port: u16,
    pub tls_cert_file: String,
    pub tls_key_file: String,
}

pub fn load() -> Result<Configuration> {
    let mut file = File::open(DEFAULT_CONFIG_FILE_PATH).context(format!(
        "failed to open the config file: {DEFAULT_CONFIG_FILE_PATH}"
    ))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).context(format!(
        "failed to load the config file: {DEFAULT_CONFIG_FILE_PATH}"
    ))?;
    Ok(serde_yaml::from_str(&contents)?)
}

struct LogLevelVisitor;

impl Visitor<'_> for LogLevelVisitor {
    type Value = log::LevelFilter;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string representing a valid log::LevelFilter variant")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value.to_ascii_lowercase().as_str() {
            "trace" => Ok(log::LevelFilter::Trace),
            "debug" => Ok(log::LevelFilter::Debug),
            "info" => Ok(log::LevelFilter::Info),
            "warning" => Ok(log::LevelFilter::Warn),
            "error" => Ok(log::LevelFilter::Error),
            _ => Err(E::custom(format!(
                "unknown log::LevelFilter variant: {value}"
            ))),
        }
    }
}

fn deserialize_log_level<'a, D>(deserializer: D) -> Result<log::LevelFilter, D::Error>
where
    D: Deserializer<'a>,
{
    deserializer.deserialize_str(LogLevelVisitor {})
}
