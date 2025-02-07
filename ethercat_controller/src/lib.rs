pub mod config;
pub use config::Config;

pub mod ethercat_controller;
pub use ethercat_controller::EtherCatController;

use ethercat::{Offset, PdoEntryIdx, SlavePos};
use std::collections::HashMap;

/// Type alias for the PDO offsets
///
/// The PDO offsets are stored in a hashmap with the name of the PDO as the key
/// and a vector of tuples with the index of the PDO entry, the subindex of the PDO entry and the offset
/// as the value
pub type PdoOffsets = HashMap<String, Vec<(PdoEntryIdx, u8, Offset)>>;
/// Type alias for the slave offsets
///
/// The slave offsets are stored in a hashmap with the slave position as the key
/// and the PDO offsets as the value
pub type SlaveOffsets = HashMap<SlavePos, PdoOffsets>;
/// Type alias for the slave names
pub type SlaveNames = HashMap<String, SlavePos>;
/// Type alias for the slave setup
///
/// The slave setup is stored in a hashmap with the slave position as the key
pub type SlaveSetup = HashMap<SlavePos, bool>;
/// Type alias for the mailbox PDO entries
///   
/// The mailbox PDO entries are stored in a hashmap with the slave position as the key
/// and a vector of strings as the value
pub type MailboxPdoEntries = HashMap<SlavePos, Vec<String>>;

pub mod mailboxes;
pub mod watchdog;

pub mod ethercat_patch;
