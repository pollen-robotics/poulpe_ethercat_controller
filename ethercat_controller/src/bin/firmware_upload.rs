use ethercat::{Master, MasterAccess, SlaveId, SlavePos, SmIdx};
use ethercat_controller::ethercat_controller::init_master_for_foe;
use ethercat_controller::mailboxes::{mailbox_sdo_read, mailbox_sdo_write};
use log;
use std::{thread, time::Duration};

fn main() -> Result<(), std::io::Error> {
    env_logger::init();

    let mut master = init_master_for_foe(0)?;

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        log::error!("Usage: firmware_upload <slave-position> <file>");
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Not enough arguments",
        ));
    }

    let idx: u16 = args[1]
        .parse::<u16>()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        .into();
    let slave_pos = SlavePos::from(idx);
    let file = &args[2];
    let file_name = std::path::Path::new(file)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    let file_size = std::fs::metadata(file)?.len();
    let buf = std::fs::read(file)?;

    let mut data = vec![0; 40];
    match mailbox_sdo_read(&mut master, idx, 0x200, 0x1, &mut data) {
        Ok(_) => {
            log::info!("Firmware version: {:?}", String::from_utf8(data).unwrap());
        }
        Err(e) => {
            log::error!("Error: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
    }

    log::info!(
        "Writing firmware to slave {:?} from file {}, with size {}",
        slave_pos,
        file_name,
        file_size
    );

    match master.foe_write(slave_pos, file_name, &buf) {
        Ok(_) => log::info!("Firmware written"),
        Err(e) => {
            log::error!("Error: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
    }

    log::info!("Verfiying the number of bytes written");
    let mut data: Vec<u8> = vec![0; 4];
    match mailbox_sdo_read(&mut master, idx, 0x100, 0x1, &mut data) {
        Ok(_) => {
            let bytes_written = u32::from_le_bytes(data.as_slice().try_into().unwrap());
            if bytes_written == file_size as u32 {
                log::info!(
                    "Firmware written successfully, {} bytes written",
                    bytes_written
                );
            } else {
                log::error!(
                    "Error: Firmware not written successfully, {} bytes written, expected {}",
                    bytes_written,
                    file_size
                );
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Firmware not written successfully",
                ));
            }
        }
        Err(e) => {
            log::error!("Error: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
    }

    log::info!("Resetting the slave");
    thread::sleep(Duration::from_secs(1));
    match mailbox_sdo_write(&mut master, idx, 0x100, 0x1, &(file_size as u32)) {
        Ok(_) => log::info!("Slave reset request sent"),
        Err(e) => {
            log::error!("Error: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
    }

    Ok(())
}
