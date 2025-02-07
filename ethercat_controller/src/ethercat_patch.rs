// function not available in the ethercat-rs crate

use ethercat::{Master, SlavePos, SmInfo};
use std::io;

use std::os::fd::AsRawFd;
use std::{convert::TryFrom, ffi::CStr, fs::OpenOptions};

use ethercat_sys as ec;

/// ioctl macro to call the ioctl function
/// 
/// Copied from the ethercat-rs crate
macro_rules! ioctl {
    ($m:expr, $f:expr) => { ioctl!($m, $f,) };
    ($m:expr, $f:expr, $($arg:tt)*) => {{
        let file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&"/dev/EtherCAT0")?;
        let res = unsafe { $f(file.as_raw_fd(), $($arg)*) };
        if res < 0 { Err(ethercat::Error::Io(io::Error::last_os_error())) } else { Ok(res) }
    }}
}


/// Function that configures the sync manager of the master
/// It is missing from the ethercat-rs crate
/// 
/// # Arguments
///
/// * `master` - A mutable reference to the master
/// * `slave_pos` - The slave position
/// * `sm` - The sync manager information
/// 
/// # Returns
/// 
/// * `Result<(), ethercat::Error>` - The result of the operation
/// 
pub fn master_configure_sync(
    master: &mut Master,
    slave_pos: SlavePos,
    sm: SmInfo,
) -> Result<(), ethercat::Error> {
    let mut sync = ec::ec_ioctl_slave_sync_t::default();
    sync.slave_position = u16::from(slave_pos);
    sync.sync_index = u8::from(sm.idx) as u32;
    sync.physical_start_address = sm.start_addr;
    sync.control_register = sm.control_register;
    sync.enable = if sm.enable { 1 } else { 0 };
    sync.pdo_count = sm.pdo_count;
    sync.default_size = sm.default_size;
    ioctl!(master, ec::ioctl::SLAVE_SYNC, &mut sync).map(|_| ())
}
