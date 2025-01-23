use std::{io, ops::Range};

use ethercat::{SdoData, Master, SdoIdx};

use crate::{MailboxPdoEntries, SlaveOffsets, SlavePos};

// init the necessary variables for mailbox pdo verification
// offsets of the mailboxes data in the domain data
// last read timestamp of the mailbox data
// flag to check if the slave is responding
// buffer to store the mailbox data (that are read asynchronusly from the slaves)
// 
// NOTE: 
// - mailbox PDOs are different from the normal buffered PDOs and they are not always present
pub fn init_mailbox_pdo_verification(
    slave_number: u32,
    mailbox_pdo_entries: &MailboxPdoEntries,
    offsets: &SlaveOffsets,
    get_reg_addr_ranges: &impl Fn(&SlaveOffsets, u16, &String) -> Vec<Range<usize>>,
) -> (
    Vec<Vec<Range<usize>>>,
    Vec<std::time::Instant>,
    Vec<bool>,
    Vec<Vec<Vec<u8>>>,
) {
    // initialize the mailbox verification variables
    // offsets of the mailboxes data in the domain data
    let mut slave_mailbox_pdo_offsets = vec![];
    // last read timestamp of the mailbox data
    let slave_mailbox_pdo_timestamps = vec![std::time::Instant::now(); slave_number as usize];
    // flag to check if the slave is responding
    let slave_is_mailbox_pdo_responding = vec![true; slave_number as usize];
    // buffer to store the mailbox data (that are read asynchronusly from the slaves)
    let slave_mailbox_pdo_data_buffer = vec![vec![]; slave_number as usize];

    // find the mailbox offsets for each slave
    for i in 0..slave_number {
        let mut mailbox_offsets = vec![];
        for m in mailbox_pdo_entries.get(&SlavePos::from(i as u16)).unwrap() {
            mailbox_offsets.append(&mut get_reg_addr_ranges(&offsets, i as u16, m));
        }
        slave_mailbox_pdo_offsets.push(mailbox_offsets);
    }

    (
        slave_mailbox_pdo_offsets,
        slave_mailbox_pdo_timestamps,
        slave_is_mailbox_pdo_responding,
        slave_mailbox_pdo_data_buffer,
    )
}

// verify the mailboxe pdos of the slaves (if they are available)
// verify that the slaves are still writing
// checking if all the mailbox values are zero for more than 1s
// - if the values are not zero, update the timestamp
// - if the values are zero, check if the timestamp is more than 1s
//      - if the timestamp is more than 1s, set the slave as not responding
// - if the values are not zero, update the timestamp
//
// NOTE: 
//  - mailbox PDOs are different from the normal buffered PDOs as they are only updated once the slave writes to them
//    and if the slave is not writing to them, the values will be read as zero
//  - therefore this function is used to check if the slaves are still writing to the mailbox PDOs
//    and if they are the mailbox pdo data is buffered and copied to the domain data 
pub fn verify_mailbox_pdos(
    slave_number: u32,
    data: &mut [u8],
    slave_mailbox_pdo_offsets: &mut Vec<Vec<Range<usize>>>,
    slave_mailbox_pdo_timestamps: &mut Vec<std::time::Instant>,
    slave_is_mailbox_pdo_responding: &mut Vec<bool>,
    slave_mailbox_pdo_data_buffer: &mut Vec<Vec<Vec<u8>>>,
    mailbox_wait_time_ms: u32,
) -> bool {
    // return if all slaves responding
    let mut all_slaves_responding = true;

    // check each slave
    for i in 0..slave_number {
        // get slave mailbox offset
        let offset = slave_mailbox_pdo_offsets[i as usize].clone();
        
        if offset.is_empty() {
            // if there are no mailbox pdos for the slave, continue 
            continue;
        }
        // get the mailbox data
        let mailbox_data = offset
            .iter()
            .map(|range| data[range.clone()].to_vec())
            .collect::<Vec<_>>();
        log::debug!("{:?}", mailbox_data);
        // check if all the values are zero
        let is_all_zeros = mailbox_data.iter().all(|d| d.iter().all(|&x| x == 0));

        // flag to check if slave is responding
        slave_is_mailbox_pdo_responding[i as usize] = true;
        // if all the values are zero for more than 1s
        if is_all_zeros {
            if slave_mailbox_pdo_timestamps[i as usize].elapsed().as_millis() as u32
                > mailbox_wait_time_ms
            {
                all_slaves_responding &= false; // set the all slaves responding flag to false
                slave_is_mailbox_pdo_responding[i as usize] = false;
            }
        } else {
            // if the values are not zero
            slave_mailbox_pdo_timestamps[i as usize] = std::time::Instant::now();
            slave_is_mailbox_pdo_responding[i as usize] = true;
            slave_mailbox_pdo_data_buffer[i as usize] = mailbox_data;
        }

        if slave_is_mailbox_pdo_responding[i as usize] {
            for (j, range) in offset.iter().enumerate() {
                if slave_mailbox_pdo_data_buffer[i as usize].len() > j {
                    data[range.clone()].copy_from_slice(&slave_mailbox_pdo_data_buffer[i as usize][j]);
                }
            }
        }
    }

    return all_slaves_responding;
}


// write to the mailbox sdo 
// - write the data to the mailbox sdo with the given index and subindex
// - the data size is determined automatically by the data type
pub fn mailbox_sdo_write<T: SdoData>(master: &mut Master, slave_id: u16, idx: u16, sub_idx: u8, data: &T) -> Result<(), io::Error> {
    let sdo_idx = SdoIdx::new(idx, sub_idx);
    let sdo_pos = SlavePos::from(slave_id);
    master.sdo_download(sdo_pos, sdo_idx, false, data)?;
    Ok(())
}

// read from the mailbox sdo
// - read the data from the mailbox sdo with the given index and subindex
// - the data size is determined automatically from the data vector's number of elements
pub fn mailbox_sdo_read(master: &Master, slave_id: u16, idx: u16, sub_idx: u8, data: &mut Vec<u8>) -> Result<(), io::Error> {
    let sdo_idx = SdoIdx::new(idx, sub_idx);
    let sdo_pos = SlavePos::from(slave_id);
    master.sdo_upload(sdo_pos, sdo_idx, false, data)?;
    Ok(())
}