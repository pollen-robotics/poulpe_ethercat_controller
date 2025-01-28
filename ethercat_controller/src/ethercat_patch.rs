// function not available in the ethercat-rs crate

use ethercat::{Master, SlavePos, SmInfo};
use std::io;

use std::{
    convert::TryFrom,
    ffi::CStr,
    fs::{OpenOptions}
};
use std::os::fd::AsRawFd;

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

use ethercat_sys as ec;
pub fn master_configure_sync(master: &mut Master, slave_pos: SlavePos, sm: SmInfo) -> Result<(),ethercat::Error> {
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