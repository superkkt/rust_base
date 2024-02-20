use anyhow::{Context, Result};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::fs::File;
use std::io::Read;

const DEFAULT_CONFIG_FILE_PATH: &str = "/etc/rust_base.yaml";

#[derive(Debug, Deserialize)]
pub struct Configuation {
    pub log: Log,
}

#[derive(Debug, Deserialize)]
pub struct Log {
    #[serde(deserialize_with = "deserialize_log_level")]
    pub level: log::LevelFilter,
}

pub fn load() -> Result<Configuation> {
    let mut file = File::open(DEFAULT_CONFIG_FILE_PATH)
        .context(format!("failed to open the config file: {}", DEFAULT_CONFIG_FILE_PATH))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .context(format!("failed to load the config file: {}", DEFAULT_CONFIG_FILE_PATH))?;
    Ok(serde_yaml::from_str(&contents)?)
}

struct LogLevelVisitor;

impl<'de> Visitor<'de> for LogLevelVisitor {
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
            _ => Err(E::custom(format!("unknown log::LevelFilter variant: {}", value))),
        }
    }
}

fn deserialize_log_level<'de, D>(deserializer: D) -> Result<log::LevelFilter, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(LogLevelVisitor)
}
