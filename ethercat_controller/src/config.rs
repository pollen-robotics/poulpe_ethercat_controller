use std::{error::Error, fs};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub ethercat: EthercatConfig,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct EthercatConfig {
    pub master_id: u32,
    pub cycle_time_us: u32,
    pub command_drop_time_us: u32,
    pub watchdog_timeout_ms: u32,
    pub mailbox_wait_time_ms: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SlaveConfig {
    Poulpe(PoulpeKind),
    Unknown,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PoulpeKind {
    pub id: u16,
    pub orbita_type: u32,
    pub name: String,
}

impl Config {
    pub fn from_yaml(path: &str) -> Result<Self, Box<dyn Error>> {
        let yaml = fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&yaml)?)
    }
}
