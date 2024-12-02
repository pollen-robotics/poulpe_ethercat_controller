use std::{
    ops::Range,
};

use crate::{MailboxEntries, SlaveOffsets, SlavePos};

// init the necessary variables for mailbox verification
// offsets of the mailboxes data in the domain data
// last read timestamp of the mailbox data
// flag to check if the slave is responding
// buffer to store the mailbox data (that are read asynchronusly from the slaves)
pub fn init_mailbox_verification(
    slave_number: u32,
    mailbox_entries: &MailboxEntries,
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
    let mut slave_mailbox_offsets = vec![];
    // last read timestamp of the mailbox data
    let slave_mailbox_timestamps = vec![std::time::Instant::now(); slave_number as usize];
    // flag to check if the slave is responding
    let slave_is_mailbox_responding = vec![true; slave_number as usize];
    // buffer to store the mailbox data (that are read asynchronusly from the slaves)
    let slave_mailbox_data_buffer = vec![vec![]; slave_number as usize];

    // find the mailbox offsets for each slave
    for i in 0..slave_number {
        let mut mailbox_offsets = vec![];
        for m in mailbox_entries.get(&SlavePos::from(i as u16)).unwrap() {
            mailbox_offsets.append(&mut get_reg_addr_ranges(&offsets, i as u16, m));
        }
        slave_mailbox_offsets.push(mailbox_offsets);
    }

    (
        slave_mailbox_offsets,
        slave_mailbox_timestamps,
        slave_is_mailbox_responding,
        slave_mailbox_data_buffer,
    )
}

// verify the mailboxes of the slaves
// verify that the slaves are still writing
// checking if all the mailbox values are zero for more than 1s
// - if the values are not zero, update the timestamp
// - if the values are zero, check if the timestamp is more than 1s
//      - if the timestamp is more than 1s, set the slave as not responding
// - if the values are not zero, update the timestamp
//
// the mailbox data is buffered and copied to the domain data if the slave is responding
pub fn verify_mailboxes(
    slave_number: u32,
    data: &mut [u8],
    slave_mailbox_offsets: &mut Vec<Vec<Range<usize>>>,
    slave_mailbox_timestamps: &mut Vec<std::time::Instant>,
    slave_is_mailbox_responding: &mut Vec<bool>,
    slave_mailbox_data_buffer: &mut Vec<Vec<Vec<u8>>>,
    mailbox_wait_time_ms: u32,
) -> bool {
    // return if all slaves responding
    let mut all_slaves_responding = true;
    // check each slave
    for i in 0..slave_number {
        // get slave mailbox offset
        let offset = slave_mailbox_offsets[i as usize].clone();
        // get the mailbox data
        let mailbox_data = offset
            .iter()
            .map(|range| data[range.clone()].to_vec())
            .collect::<Vec<_>>();
        log::debug!("{:?}", mailbox_data);
        // check if all the values are zero
        let is_all_zeros = mailbox_data.iter().all(|d| d.iter().all(|&x| x == 0));

        // flag to check if slave is responding
        slave_is_mailbox_responding[i as usize] = true;
        // if all the values are zero for more than 1s
        if is_all_zeros {
            if slave_mailbox_timestamps[i as usize].elapsed().as_millis() as u32
                > mailbox_wait_time_ms
            {
                all_slaves_responding &= false; // set the all slaves responding flag to false
                slave_is_mailbox_responding[i as usize] = false;
            }
        } else {
            // if the values are not zero
            slave_mailbox_timestamps[i as usize] = std::time::Instant::now();
            slave_is_mailbox_responding[i as usize] = true;
            slave_mailbox_data_buffer[i as usize] = mailbox_data;
        }

        if slave_is_mailbox_responding[i as usize] {
            for (j, range) in offset.iter().enumerate() {
                if slave_mailbox_data_buffer[i as usize].len() > j {
                    data[range.clone()].copy_from_slice(&slave_mailbox_data_buffer[i as usize][j]);
                }
            }
        }
    }

    return all_slaves_responding;
}
