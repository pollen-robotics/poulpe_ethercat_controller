pub mod config;
pub use config::Config;

mod ethercat_controller;
pub use ethercat_controller::EtherCatController;

use ethercat::{Offset, PdoEntryIdx, SlavePos};
use std::collections::HashMap;

pub type PdoOffsets = HashMap<String, Vec<(PdoEntryIdx, u8, Offset)>>;
pub type SlaveOffsets = HashMap<SlavePos, PdoOffsets>;
pub type SlaveNames = HashMap<String, SlavePos>;
pub type SlaveSetup = HashMap<SlavePos, bool>;
pub type MailboxEntries = HashMap<SlavePos, Vec<String>>;

mod mailboxes;
mod watchdog;
