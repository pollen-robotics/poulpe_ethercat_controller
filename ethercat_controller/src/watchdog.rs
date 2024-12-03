use crate::SlaveOffsets;
use std::ops::Range;

// watchdog is added to the manufcturer specific data of the statusword
// bits 8, 14 and 15
// parse the 3bit watchdog counter from the statusword
fn parse_watchdog_from_status(statusword: Vec<u8>) -> u8 {
    // bit 8
    let mut watchdog_counter = (statusword[1] & 0b0000_0001);
    // bits 14 and 15
    watchdog_counter = watchdog_counter | ((statusword[1] & 0b1100_0000) >> 5);
    // return the counter
    watchdog_counter
}

// write the watchdog counter to the controlword
// to the bits 11-15 which are manufacturer specific
fn write_watchdog_to_control(control_word: Vec<u8>, watchdog_counter: u8) -> Vec<u8> {
    let mut control_word = control_word;
    // clear the bits 11-15
    control_word[1] &= 0b0000_0111;
    // write the watchdog counter to the controlword
    control_word[1] |= watchdog_counter << 3;
    // return the controlword
    control_word
}

// verify the watchdog of the slaves
// verify that the slaves are still writing
// checking if the watchdog counter is the same as the previous cycle
// - if the counter is the same, check for how long has it been the same
//      - if it is the same for more than 1s the slave is considered not responding
// - if the counter is different, update the timestamp
// - write the watchdog counter to the controlword
pub fn verify_watchdog(
    slave_number: u32,
    data: &mut [u8],
    watchdog_timeout_ms: u32,
    watchdog_counter: u8,
    slave_watchdog_control_offsets: &Vec<Vec<Range<usize>>>,
    slave_watchdog_status_offsets: &Vec<Vec<Range<usize>>>,
    slave_watchdog_timestamps: &mut Vec<std::time::Instant>,
    slave_is_watchdog_responding: &mut Vec<bool>,
    slave_previous_watchdog_counter: &mut Vec<u8>,
    slave_name_from_id: &impl Fn(u16) -> String,
) -> bool {
    // return if all slaves responding
    let mut all_slaves_responding = true;
    // check each slave

    for i in 0..slave_number {
        // get slave watchdog control offset
        let status_offset = slave_watchdog_status_offsets[i as usize].clone();
        // get the watchdog control data
        let status_data = status_offset
            .iter()
            .map(|range| data[range.clone()].to_vec())
            .collect::<Vec<_>>();

        // doutput the watchdog status in binary
        let counter = parse_watchdog_from_status(status_data[0].clone());
        log::debug!(
            "Slave {} ({})| Watchdog counter received : {} ({:08b}), sent: {} ({:08b})",
            i,
            slave_name_from_id(i as u16),
            counter,
            counter,
            watchdog_counter,
            watchdog_counter
        );

        // check if the watchdog counter is the same as the one in the previous cycle
        // if it is the same check for how long has it been the same
        // if it is the same for more than 1s the slave is considered not responding
        if slave_previous_watchdog_counter[i as usize] == counter {
            if slave_watchdog_timestamps[i as usize].elapsed().as_millis() as u32
                > watchdog_timeout_ms
            {
                all_slaves_responding &= false;
                slave_is_watchdog_responding[i as usize] = false;
            }
        } else {
            // if the watchdog counter is different
            // update the timestamp
            slave_watchdog_timestamps[i as usize] = std::time::Instant::now();
            slave_is_watchdog_responding[i as usize] = true;
            slave_previous_watchdog_counter[i as usize] = counter;
        }

        // write the counter to the controlword
        let control_offset = slave_watchdog_control_offsets[i as usize].clone();
        for (j, range) in control_offset.iter().enumerate() {
            let control_word = data[range.clone()].to_vec();
            data[range.clone()]
                .copy_from_slice(&write_watchdog_to_control(control_word, watchdog_counter));
        }
    }
    all_slaves_responding
}

// initialize the watchdog settings
// find the offsets of the controlword and statusword data in the domain data
// initialize the timestamp, flag and buffer for the watchdog data
pub fn init_watchdog_settings(
    slave_number: u32,
    offsets: &SlaveOffsets,
    get_reg_addr_ranges: &impl Fn(&SlaveOffsets, u16, &String) -> Vec<Range<usize>>,
) -> (
    Vec<Vec<Range<usize>>>,
    Vec<Vec<Range<usize>>>,
    Vec<std::time::Instant>,
    Vec<bool>,
    Vec<u8>,
) {
    // initialize the watchdog variables
    // offsets of the statusword data in the domain data
    let mut slave_watchdog_status_offsets = vec![];
    // offsets of the controlword data in the domain data
    let mut slave_watchdog_control_offsets = vec![];
    // last read timestamp of the watchdog data
    let slave_watchdog_timestamps = vec![std::time::Instant::now(); slave_number as usize];
    // flag to check if the slave is responding
    let slave_is_watchdog_responding = vec![true; slave_number as usize];
    // buffer to store the watchdog data (that are read asynchronusly from the slaves)
    let slave_previous_watchdog_counter = vec![0u8; slave_number as usize];

    // find the watchdog offsets for each slave
    for i in 0..slave_number {
        let mut watchdog_offsets = vec![];
        watchdog_offsets.append(&mut get_reg_addr_ranges(
            &offsets,
            i as u16,
            &"controlword".to_string(),
        ));
        slave_watchdog_control_offsets.push(watchdog_offsets);
    }
    for i in 0..slave_number {
        let mut watchdog_offsets = vec![];
        watchdog_offsets.append(&mut get_reg_addr_ranges(
            &offsets,
            i as u16,
            &"statusword".to_string(),
        ));
        slave_watchdog_status_offsets.push(watchdog_offsets);
    }

    (
        slave_watchdog_control_offsets,
        slave_watchdog_status_offsets,
        slave_watchdog_timestamps,
        slave_is_watchdog_responding,
        slave_previous_watchdog_counter,
    )
}
