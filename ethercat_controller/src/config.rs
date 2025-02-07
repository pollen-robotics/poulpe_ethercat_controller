use std::{error::Error, fs};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub ethercat: EthercatConfig,
}

/// Configuration for the Ethercat master
///
/// The master id is the id of the master in the Ethercat network
/// The cycle time is the time in microseconds between each cycle
/// The command drop time is the time in microseconds to wait for the command to be dropped
/// The watchdog timeout is the time in milliseconds to wait for the watchdog to be updated
/// The mailbox wait time is the time in milliseconds to wait for the mailbox to be updated
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

/// Configuration for the Poulpe slave
///
/// The id is the id of the slave in the Ethercat network
/// The orbita type is the type of the orbita
/// The name is the name of the slave
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PoulpeKind {
    pub id: u16,
    pub orbita_type: u32,
    pub name: String,
}

impl Config {
    /// Load the configuration from a YAML file
    ///     
    /// # Arguments
    ///
    /// * `path` - The path to the YAML file
    ///
    /// # Returns
    ///
    /// * `Result<Self, Box<dyn Error>>` - The result of the operation
    pub fn from_yaml(path: &str) -> Result<Self, Box<dyn Error>> {
        let yaml = fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&yaml)?)
    }
}
