extern crate num;
#[macro_use]
extern crate num_derive;

mod ethercat_controller;
mod epos_controller;

pub use ethercat_controller::EtherCatController;
pub use epos_controller::EposController;