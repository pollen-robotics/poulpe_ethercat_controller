use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read},
    ops::Range,
    sync::{Arc, Condvar, Mutex, RwLock},
    thread,
    time::Duration,
};

use ethercat::{
    AlState, DataType, DomainIdx, Master, MasterAccess, Offset, PdoCfg, PdoEntryIdx, PdoEntryInfo,
    PdoEntryPos, PdoIdx, PdoPos, SdoData, SdoIdx, SdoPos, SlaveAddr, SlaveId, SlavePos, SmCfg,
    SmIdx, SmInfo, SubIdx,
};

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::{watchdog, MailboxPdoEntries, PdoOffsets, SlaveNames, SlaveOffsets, SlaveSetup};

// function not available in the ethercat-rs crate
use crate::ethercat_patch::master_configure_sync;

#[cfg(feature = "verify_mailbox_pdos")]
use crate::mailboxes::{init_mailbox_pdo_verification, verify_mailbox_pdos};
#[cfg(feature = "enable_watchdog")]
use crate::watchdog::{init_watchdog_settings, verify_watchdog};

use crate::mailboxes::mailbox_sdo_read;

#[derive(Debug)]
pub struct EtherCatController {
    offsets: SlaveOffsets,
    slave_names: SlaveNames,

    data_lock: Arc<RwLock<Option<Vec<u8>>>>,
    ready_condvar: Arc<(Mutex<bool>, Condvar)>,
    cycle_condvar: Arc<(Mutex<bool>, Condvar)>,
    slave_states_condvar: Arc<(Mutex<Vec<u8>>, Condvar)>,

    cmd_buff: Sender<(Range<usize>, Vec<u8>)>,

    // is poulpe setup
    setup_condvar: Arc<(Mutex<SlaveSetup>, Condvar)>,

    pub command_drop_time_us: u32,
}

impl EtherCatController {
    pub fn open(
        master_id: u32,
        cycle_period: Duration,
        command_drop_time_us: u32,
        watchdog_timeout_ms: u32,
        mailbox_wait_time_ms: u32,
    ) -> Result<Self, io::Error> {
        let (mut master, domain_idx, offsets, slave_names, mailbox_pdo_entries) =
            init_master(master_id)?;

        // read the slave info using SDOs
        // IMPORTANT !!!!!!!
        // must be done before master.activate()
        for slave_id in 0..slave_names.len() {
            let mut data = vec![0u8; 1];
            match mailbox_sdo_read(&master, slave_id as u16, 0x201, 0x1, &mut data) {
                Ok(_) => {
                    log::info!("Slave {}, DXL_ID: {:?}", slave_id, data[0]);
                }
                Err(_) => {
                    log::warn!("Slave {}, DXL_ID unknown!", slave_id);
                }
            }

            let mut data = vec![0u8; 40];
            match mailbox_sdo_read(&master, slave_id as u16, 0x200, 0x1, &mut data) {
                Ok(_) => {
                    log::info!(
                        "Slave {} firmware version: {:?}",
                        slave_id,
                        String::from_utf8(data).unwrap()
                    );
                }
                Err(_) => {
                    log::warn!("Slave {}, firmware version unknown!", slave_id);
                }
            }
        }

        master.activate()?;

        // log the pdo offsets (debug)
        log_pdo_offsets(&offsets);

        // create the synhronization variables
        // EtherCAT data mutex
        let data_lock = Arc::new(RwLock::new(None));
        let write_data_lock = Arc::clone(&data_lock);

        // ethercat master ready mutex
        let ready_condvar = Arc::new((Mutex::new(false), Condvar::new()));
        let write_ready_condvar = Arc::clone(&ready_condvar);

        // ethercat master cycle mutex
        let cycle_condvar = Arc::new((Mutex::new(false), Condvar::new()));
        let write_cycle_condvar = Arc::clone(&cycle_condvar);

        // ethercat slave states mutex
        let slave_states_condvar = Arc::new((Mutex::new(vec![0]), Condvar::new()));
        let sstate_condvar = Arc::clone(&slave_states_condvar);

        // ethercat slave is setup mutex
        let mut is_poulpe_setup: SlaveSetup = HashMap::new();
        for i in 0..slave_names.len() {
            is_poulpe_setup.insert(SlavePos::from(i as u16), false);
        }
        let slave_setup_condvar = Arc::new((Mutex::new(is_poulpe_setup), Condvar::new()));
        let setup_condvar = Arc::clone(&slave_setup_condvar);

        // get the slave number
        let slave_number = slave_names.len() as u32;
        // create a function to map slave id to slave name
        let slave_name_from_id = create_slave_name_mapper(slave_names.clone());

        // create a sync channel to send data to the master
        // crossbeam_channel is more efficient than std::sync::mcsp::SyncChannel
        let buffer_size = (slave_number * 20) as usize;
        let (tx, rx): (
            crossbeam_channel::Sender<(Range<usize>, Vec<u8>)>,
            Receiver<(Range<usize>, Vec<u8>)>,
        ) = bounded(buffer_size);

        #[cfg(feature = "verify_mailbox_pdos")]
        // initialize the mailbox verification
        let (
            mut slave_mailbox_pdo_offsets,
            mut slave_mailbox_pdo_timestamps,
            mut slave_is_mailbox_pdo_responding,
            mut slave_mailbox_pdo_data_buffer,
        ) = init_mailbox_pdo_verification(
            slave_number,
            &mailbox_pdo_entries,
            &offsets,
            &get_reg_addr_ranges,
        );

        #[cfg(feature = "enable_watchdog")]
        // initialize the watchdog settings
        let (
            slave_watchdog_control_offsets,
            slave_watchdog_status_offsets,
            mut slave_watchdog_timestamps,
            mut slave_is_watchdog_responding,
            mut slave_previous_watchdog_counter,
        ) = init_watchdog_settings(slave_number, &offsets, &get_reg_addr_ranges);

        let mut watchdog_counter = 0;

        thread::spawn(move || {
            // is master operational flag
            let mut master_operational = false;
            // timestamp to say from when the master is not operational
            let mut display_not_operational_timestamp = std::time::Instant::now();
            // timestap used to establish the loop period
            let mut loop_period_timestamp = std::time::Instant::now();
            let mut debug_loop_timestamp = std::time::Instant::now();
            let mut debug_loop_counter = 0;
            // spawn a thread to handle the master
            loop {
                // check the loop period
                // make it approximately equal to the cycle period
                // make sure that the subtraction does not return a negative value
                let dt_sleep =
                    cycle_period.as_secs_f32() - loop_period_timestamp.elapsed().as_secs_f32();
                if dt_sleep > 0.0 {
                    thread::sleep(Duration::from_secs_f32(dt_sleep));
                }
                // set the loop period timestamp
                loop_period_timestamp = std::time::Instant::now();

                // debugging output
                debug_loop_counter += 1;
                if debug_loop_timestamp.elapsed().as_secs_f32() > 10.0 {
                    log::info!(
                        "EtherCAT loop: {:.02} Hz",
                        debug_loop_counter as f32 / debug_loop_timestamp.elapsed().as_secs_f32()
                    );
                    debug_loop_timestamp = std::time::Instant::now();
                    debug_loop_counter = 0;
                }

                // get the master data
                master.receive().unwrap();
                master.domain(domain_idx).process().unwrap();
                master.domain(domain_idx).queue().unwrap();

                // get the domain data
                let mut data = master.domain_data(domain_idx).unwrap();

                // verify that the poulpes are still writing
                // for each slave check if the mailbox mailbox pdo entries are updated
                // the mailbox data is being written by slaves at arounf 10Hz
                // if the mailbox data is not updated for more than 1s
                // the slave is considered not as responding
                //
                // if at least one slave is not responding function will return false
                //
                // if the slaves are responding it will update the data buffer
                // with the mailbox data (which might have been read some time ago (but less than 1s ago))
                #[cfg(feature = "verify_mailbox_pdos")]
                let all_slaves_responding = verify_mailbox_pdos(
                    slave_number,
                    &mut data,
                    &mut slave_mailbox_pdo_offsets,
                    &mut slave_mailbox_pdo_timestamps,
                    &mut slave_is_mailbox_pdo_responding,
                    &mut slave_mailbox_pdo_data_buffer,
                    mailbox_wait_time_ms,
                );

                // write the data to the data lock
                if let Ok(mut write_guard) = write_data_lock.write() {
                    *write_guard = Some(data.to_vec());
                }

                // notify the next cycle
                notify_next_cycle(&write_cycle_condvar);

                // check if the master is operational
                // and only if operational update the data buffer with the new data to send to the slaves
                if master_operational {
                    // check if the RX buffer is getting full!!!
                    // if rx.len() > 40 {log::warn!("RX buffer almost full: {}/{}", rx.len(), buffer_size)}
                    // update the data buffer with the new data to send
                    while let Ok((reg_addr_range, value)) = rx.try_recv() {
                        data[reg_addr_range].copy_from_slice(&value);
                    }
                }

                #[cfg(feature = "enable_watchdog")]
                // verify the watchdog
                let all_slaves_have_watchdog = verify_watchdog(
                    slave_number,
                    &mut data,
                    watchdog_timeout_ms,
                    watchdog_counter,
                    &slave_watchdog_control_offsets,
                    &slave_watchdog_status_offsets,
                    &mut slave_watchdog_timestamps,
                    &mut slave_is_watchdog_responding,
                    &mut slave_previous_watchdog_counter,
                    &slave_name_from_id,
                );
                // update the watchdog counter
                watchdog_counter = (watchdog_counter + 1) % 8;

                // send the data to the slaves
                master.send().unwrap();

                // get the master state
                let m_state = master.state().unwrap();
                #[cfg(not(feature = "verify_mailbox_pdos"))]
                // get the slave states without mailbox verification
                let all_slaves_responding = m_state.slaves_responding == slave_number;
                #[cfg(not(feature = "enable_watchdog"))]
                let all_slaves_have_watchdog = m_state.slaves_responding == slave_number;

                // master opration state machine
                // if the master is not operational
                //  - check if all slaves are responding
                //  - check if the link is up
                //  - check if the master is in operational state
                //  - check if the number of slaves responding is equal to the number of slaves connected (no disconnected or newly connected slaves)
                //  -> if all the conditions are met notify the operational state to the master and the slaves
                //
                // if the master is operational
                //  - check if the master is still operational
                //  - check if all slaves are connected
                //  - check if all slaves are responding
                //  - check if the master is in operational state
                //  - check if the number of slaves responding is equal to the number of slaves connected (no disconnected or newly connected slaves)
                //  -> if any of the conditions are not met notify the operational state to the slaves and set the ready flag to false
                if !master_operational {
                    // master is not operational

                    // To go to the operational state
                    // - if all slaves are responding
                    // - if all slaves have watchdog
                    // - if the link is up
                    // - if the master is in operational state
                    // - if the number of slaves responding is equal to the number of slaves connected (no disconnected or newly connected slaves)
                    if all_slaves_responding
                        && all_slaves_have_watchdog
                        && m_state.link_up
                        && m_state.al_states == AlState::Op as u8 // OP = 8 is operational
                        && m_state.slaves_responding == slave_number
                    {
                        // notify the operational state to the master
                        set_ready_flag(&write_ready_condvar, true);
                        master_operational = true;
                        // notify the operational state to the slaves
                        notify_slave_state(
                            &sstate_condvar,
                            vec![AlState::Op as u8; slave_number as usize],
                        );
                        log::info!("Master and all slaves operational!");
                    } else {
                        // check each second
                        if display_not_operational_timestamp.elapsed().as_secs() > 1 {
                            display_not_operational_timestamp = std::time::Instant::now();
                            log::warn!("Master cannot go to operational!");
                            // display the master state
                            // if the master is not operational
                            log_master_state(
                                &master,
                                slave_number,
                                #[cfg(feature = "verify_mailbox_pdos")]
                                mailbox_wait_time_ms,
                                #[cfg(feature = "enable_watchdog")]
                                watchdog_timeout_ms,
                                &slave_name_from_id,
                                #[cfg(feature = "verify_mailbox_pdos")]
                                &slave_is_mailbox_pdo_responding,
                                #[cfg(feature = "enable_watchdog")]
                                &slave_is_watchdog_responding,
                            );

                            // kill the master if error recovery not supported
                            #[cfg(feature = "stop_opeation_on_error")]
                            std::process::exit(10);
                        }
                    }
                } else {
                    // check if the master is still operational

                    // check if all slaves are connected
                    // thsis will fail if a slave is disconnected
                    // or if a new slave is connected
                    if m_state.slaves_responding != slave_number {
                        match m_state.slaves_responding {
                            0 => log::error!("No slaves are connected!"),
                            _ if m_state.slaves_responding < slave_number => log::error!(
                                "Not all slaves are connected! Expected: {}, Responding: {}",
                                slave_number,
                                m_state.slaves_responding
                            ),
                            _ if m_state.slaves_responding > slave_number => log::error!(
                                "New slaves are connected! Inintially: {}, Now: {}",
                                slave_number,
                                m_state.slaves_responding
                            ),
                            _ => {}
                        }

                        // update the slave states
                        let slave_current_state = (0..slave_number)
                            .map(|i| {
                                get_slave_current_state(
                                    &master,
                                    SlavePos::from(i as u16),
                                    &slave_name_from_id,
                                    #[cfg(feature = "verify_mailbox_pdos")]
                                    slave_is_mailbox_pdo_responding[i as usize],
                                    #[cfg(feature = "enable_watchdog")]
                                    slave_is_watchdog_responding[i as usize],
                                )
                            })
                            .collect::<Vec<_>>();

                        // notify the operational state for the slaves
                        notify_slave_state(&sstate_condvar, slave_current_state);

                        set_ready_flag(&write_ready_condvar, false);
                        master_operational = false;
                    }

                    // if master state has changed or not all slaves are responding
                    if m_state.al_states != AlState::Op as u8
                        || !all_slaves_responding
                        || !all_slaves_have_watchdog
                    {
                        // master state has changed
                        if m_state.al_states != AlState::Op as u8 {
                            log::error!(
                                "Master is not operational! State: {:?}",
                                m_state.al_states
                            );
                        }
                        if !all_slaves_responding {
                            // not all slaves are responding
                            log::error!("Not all slaves are responding!");
                        }
                        if !all_slaves_have_watchdog {
                            // not all slaves have watchdog
                            log::error!("Not all slaves have watchdog!");
                        }

                        // update the slave states
                        // with mailbox verification
                        // and watchdog verification
                        // if enabled
                        let slave_current_state = (0..slave_number)
                            .map(|i| {
                                get_slave_current_state(
                                    &master,
                                    SlavePos::from(i as u16),
                                    &slave_name_from_id,
                                    #[cfg(feature = "verify_mailbox_pdos")]
                                    slave_is_mailbox_pdo_responding[i as usize],
                                    #[cfg(feature = "enable_watchdog")]
                                    slave_is_watchdog_responding[i as usize],
                                )
                            })
                            .collect::<Vec<_>>();

                        // notify the operational state for the slaves
                        notify_slave_state(&sstate_condvar, slave_current_state);

                        // set the ready flag to false
                        set_ready_flag(&write_ready_condvar, false);
                        // master is not operational
                        master_operational = false;
                    }
                }
            }
        });

        Ok(EtherCatController {
            offsets,
            slave_names,
            data_lock,
            ready_condvar,
            cycle_condvar,
            slave_states_condvar,
            setup_condvar,
            cmd_buff: tx,
            command_drop_time_us,
        })
    }

    pub fn get_slave_ids(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = self
            .offsets
            .keys()
            .map(|slave_pos| u16::from(*slave_pos))
            .collect();
        ids.sort();
        ids
    }

    pub fn get_pdo_register(
        &self,
        slave_id: u16,
        register: &String,
        index: usize,
    ) -> Option<Vec<u8>> {
        let reg_addr_range = self.get_reg_addr_range(slave_id, register, index);

        (*self.data_lock.read().unwrap())
            .as_ref()
            .map(|data| data[reg_addr_range].to_vec())
    }

    pub fn set_pdo_register(&self, slave_id: u16, register: &String, index: usize, value: Vec<u8>) {
        let reg_addr_range = self.get_reg_addr_range(slave_id, register, index);

        self.cmd_buff.send((reg_addr_range, value)).unwrap();
    }

    pub fn get_pdo_registers(&self, slave_id: u16, register: &String) -> Option<Vec<Vec<u8>>> {
        let reg_addr_ranges = self.get_reg_addr_ranges(slave_id, register);

        let vals = reg_addr_ranges
            .iter()
            .map(|reg_addr_range| {
                (*self.data_lock.read().unwrap())
                    .as_ref()
                    .map(|data| data[reg_addr_range.clone()].to_vec())
            })
            .collect::<Option<Vec<Vec<u8>>>>()?;
        Some(vals)
    }

    pub fn set_pdo_registers(&self, slave_id: u16, register: &String, values: Vec<Vec<u8>>) {
        let reg_addr_ranges = self.get_reg_addr_ranges(slave_id, register);

        if values.len() != reg_addr_ranges.len() {
            // log::error!("values: {:?}", values);
            log::warn!(
                "Values length does not match register count, using first {} elements!",
                reg_addr_ranges.len()
            );
        }

        for (reg_addr_range, v) in reg_addr_ranges.iter().zip(values) {
            self.cmd_buff.send((reg_addr_range.clone(), v)).unwrap();
        }
    }

    pub fn wait_for_next_cycle(&self) {
        let (lock, cvar) = &*self.cycle_condvar;
        let mut next_cycle = lock.lock().unwrap();

        *next_cycle = false;
        while !*next_cycle {
            next_cycle = cvar.wait(next_cycle).unwrap();
        }
    }

    // check if the master is in the operational state
    pub fn master_operational(self) -> bool {
        {
            let (lock, _cvar) = &*self.ready_condvar;
            let ready = lock.lock().unwrap();
            return *ready;
        }
    }

    // true if slave is in the operational state
    pub fn is_slave_ready(&self, slave_id: u16) -> bool {
        let states = self.get_slave_states();
        match states.get(slave_id as usize).map(|s| *s) {
            Some(state) => state == (AlState::Op as u8),
            None => return false,
        }
    }

    // get all the slave states connected/configured to the master
    pub fn get_slave_states(&self) -> Vec<u8> {
        {
            let (lock, _cvar) = &*self.slave_states_condvar;
            let states = lock.lock().unwrap();
            states.clone()
        }
    }

    pub fn wait_for_ready(self) -> Self {
        {
            let (lock, cvar) = &*self.ready_condvar;
            let mut ready = lock.lock().unwrap();

            *ready = false;
            while !*ready {
                ready = cvar.wait(ready).unwrap();
            }
        }
        self
    }

    pub fn get_slave_setup(&self, slave_id: u16) -> bool {
        {
            let (lock, _cvar) = &*self.setup_condvar;
            let setup = lock.lock().unwrap();
            *setup.get(&SlavePos::from(slave_id)).unwrap_or(&false)
        }
    }

    pub fn set_slave_setup(&self, slave_id: u16, setup: bool) {
        {
            let (lock, cvar) = &*self.setup_condvar;
            let mut setup_lock = lock.lock().unwrap();
            *setup_lock.get_mut(&SlavePos::from(slave_id)).unwrap() = setup;
        }
    }

    fn get_reg_addr_range(&self, slave_id: u16, register: &String, index: usize) -> Range<usize> {
        get_reg_addr_range(&self.offsets, slave_id, register, index)
    }

    fn get_reg_addr_ranges(&self, slave_id: u16, register: &String) -> Vec<Range<usize>> {
        get_reg_addr_ranges(&self.offsets, slave_id, register)
    }

    pub fn get_slave_name(&self, slave_id: u16) -> Option<String> {
        self.slave_names
            .iter()
            .find(|(_, id)| u16::from(**id) == slave_id)
            .map(|(name, _)| name.clone())
    }

    pub fn get_slave_id(&self, slave_name: &String) -> Option<u16> {
        self.slave_names.get(slave_name).map(|id| u16::from(*id))
    }

    pub fn get_slave_ids_and_names(&self) -> Vec<(u16, String)> {
        self.slave_names
            .iter()
            .map(|(name, id)| (u16::from(*id), name.clone()))
            .collect()
    }
}

pub fn get_reg_addr_range(
    offsets: &SlaveOffsets,
    slave_id: u16,
    register: &String,
    index: usize,
) -> Range<usize> {
    let slave_pos = SlavePos::from(slave_id);

    let (_pdo_entry_idx, bit_len, offset) = offsets[&slave_pos][register][index];
    let addr = offset.byte;
    let bytes_len = (bit_len / 8) as usize;

    addr..addr + bytes_len
}

fn get_reg_addr_ranges(
    offsets: &SlaveOffsets,
    slave_id: u16,
    register: &String,
) -> Vec<Range<usize>> {
    let slave_pos = SlavePos::from(slave_id);

    // Fetch data once to minimize locking time
    let register_data = &offsets[&slave_pos][register];

    let mut ranges = Vec::with_capacity(register_data.len());
    for i in 0..register_data.len() {
        ranges.push(get_reg_addr_range(offsets, slave_id, register, i));
    }
    ranges
}

pub fn init_master_for_foe(idx: u32) -> Result<Master, io::Error> {
    // try to open the master
    // if it fails return error
    let mut master = match Master::open(idx, MasterAccess::ReadWrite) {
        Ok(master) => master,
        Err(_) => {
            log::error!("Failed to connecitng to master! Is ethercat master started?");
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to connect to master",
            ));
        }
    };
    log::debug!("Reserve master");
    master.reserve()?;
    log::debug!("Create domain");
    let domain_idx = master.create_domain()?;

    let slave_num = master.get_info().unwrap().slave_count;
    log::info!("Found {:?} slaves", slave_num);

    // if there are no slaves connected return error
    if slave_num == 0 {
        log::error!("No slaves found, check slave connections!");
        return Err(io::Error::new(io::ErrorKind::Other, "No slaves found"));
    }

    for i in 0..slave_num {
        let slave_info = master.get_slave_info(SlavePos::from(i as u16)).unwrap();
        log::info!("Slave {:?} at position {:?}", slave_info.name, i);
        log::debug!("Found device {:?}", slave_info);
        log::debug!(
            "Vendor ID: {:X}, Product Code: {:X}, SM count {:?}",
            slave_info.id.vendor_id,
            slave_info.id.product_code,
            slave_info.sync_count
        );
        let slave_addr = SlaveAddr::ByPos(i as u16);
        let slave_id = SlaveId {
            vendor_id: slave_info.id.vendor_id,
            product_code: slave_info.id.product_code,
        };

        for j in 0..slave_info.sync_count {
            let sm_idx = SmIdx::new(j);
            let sm_info = master.get_sync(SlavePos::from(i as u16), sm_idx).unwrap();

            // sanity check
            if sm_info.pdo_count == 0 {
                log::debug!("No PDOs found for SM {:?}", sm_idx);
            }

            // check if second bit is set
            // if it is its in mailbox mode
            if sm_info.control_register & 0b10 != 0 {
                log::debug!("SM is in mailbox mode!");
            } else {
                log::debug!("SM is in buffered mode!");
                continue;
            }

            if sm_info.control_register & 0b100 != 0 {
                log::debug!("Input SM!");
            } else {
                log::debug!("Output SM!");
            }

            master_configure_sync(&mut master, SlavePos::from(i as u16), sm_info);
        }

        let mut config = master.configure_slave(slave_addr, slave_id)?;

        let cfg_index = config.index();

        let cfg_info = master.get_config_info(cfg_index)?;
        if cfg_info.slave_position.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Unable to configure slave",
            ));
        }
    }

    Ok(master)
}

pub fn init_master(
    idx: u32,
) -> Result<
    (
        Master,
        DomainIdx,
        SlaveOffsets,
        SlaveNames,
        MailboxPdoEntries,
    ),
    io::Error,
> {
    // try to open the master
    // if it fails return error
    let mut master = match Master::open(idx, MasterAccess::ReadWrite) {
        Ok(master) => master,
        Err(_) => {
            log::error!("Failed to connecitng to master! Is ethercat master started?");
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to connect to master",
            ));
        }
    };
    log::debug!("Reserve master");
    master.reserve()?;
    log::debug!("Create domain");
    let domain_idx = master.create_domain()?;
    let mut offsets: SlaveOffsets = HashMap::new();
    let mut slave_names: SlaveNames = HashMap::new();

    let mut mailbox_pdos: MailboxPdoEntries = HashMap::new();

    let slave_num = master.get_info().unwrap().slave_count;
    log::info!("Found {:?} slaves", slave_num);

    // if there are no slaves connected return error
    if slave_num == 0 {
        log::error!("No slaves found, check slave connections!");
        return Err(io::Error::new(io::ErrorKind::Other, "No slaves found"));
    }

    for i in 0..slave_num {
        let slave_info = master.get_slave_info(SlavePos::from(i as u16)).unwrap();
        log::info!("Slave {:?} at position {:?}", slave_info.name, i);
        slave_names.insert(slave_info.name.clone(), SlavePos::from(i as u16));
        log::debug!("Found device {:?}", slave_info);
        log::debug!(
            "Vendor ID: {:X}, Product Code: {:X}, SM count {:?}",
            slave_info.id.vendor_id,
            slave_info.id.product_code,
            slave_info.sync_count
        );
        let slave_addr = SlaveAddr::ByPos(i as u16);
        let slave_id = SlaveId {
            vendor_id: slave_info.id.vendor_id,
            product_code: slave_info.id.product_code,
        };

        let mut pdos: Vec<Vec<PdoCfg>> = vec![];
        let mut sms = vec![];
        let mut mailbox = vec![];
        let mut direction = vec![];
        let mut mailbox_entires = vec![];
        for j in 0..slave_info.sync_count {
            let sm_idx = SmIdx::new(j);
            let sm_info = master.get_sync(SlavePos::from(i as u16), sm_idx).unwrap();
            log::debug!("Found sm {:?}, pdo_count {:?}", sm_info, sm_info.pdo_count);

            // sanity check
            if sm_info.pdo_count == 0 {
                log::debug!("No pdo found in sync manager, skipping!");
                continue;
            }

            // check if second bit is set
            // if it is its in mailbox mode
            if sm_info.control_register & 0b10 != 0 {
                log::debug!("SM is in mailbox mode!");
                mailbox.push(true);
            } else {
                log::debug!("SM is in buffered mode!");
                mailbox.push(false);
            }

            if sm_info.control_register & 0b100 != 0 {
                log::debug!("Input pdos!");
                direction.push(1);
            } else {
                log::debug!("Output pdos!");
                direction.push(-1);
            }

            let mut pdo_cfgs = vec![];
            for pdo_ind in 0..sm_info.pdo_count {
                let pdo_cfg: PdoCfg = {
                    let pdo_info = master
                        .get_pdo(SlavePos::from(i as u16), sm_idx, PdoPos::new(pdo_ind))
                        .unwrap();
                    log::debug!(
                        "Found pdo {:?}, entry_count {:?}",
                        pdo_info,
                        pdo_info.entry_count
                    );

                    let pdo_entries = (0..pdo_info.entry_count)
                        .map(|e| {
                            let entry_info = master
                                .get_pdo_entry(
                                    SlavePos::from(i as u16),
                                    sm_idx,
                                    PdoPos::new(pdo_ind),
                                    PdoEntryPos::new(e),
                                )
                                .unwrap();
                            log::debug!(
                                "Found entry {:?}, bit_len {:?}",
                                entry_info,
                                entry_info.bit_len
                            );
                            PdoEntryInfo {
                                entry_idx: entry_info.entry_idx,
                                bit_len: entry_info.bit_len as u8,
                                name: entry_info.name.clone(),
                                pos: PdoEntryPos::from(e as u8),
                            }
                        })
                        .collect();
                    PdoCfg {
                        idx: PdoIdx::new(pdo_info.idx.into()),
                        entries: pdo_entries,
                    }
                };
                pdo_cfgs.push(pdo_cfg.clone());
            }
            pdos.push(pdo_cfgs.clone());
            sms.push(sm_info);
        }

        let mut config = master.configure_slave(slave_addr, slave_id)?;
        let mut entry_offsets: PdoOffsets = HashMap::new();

        for i in 0..sms.len() {
            let pds = pdos[i].clone();
            let sm = sms[i].clone();

            // check if second bit is set
            // if it is its in input mode
            if direction[i] > 0 {
                config.config_sm_pdos(SmCfg::output(sm.idx), &pds)?;
                // Positions of TX PDO
                for pdo in &pds {
                    log::debug!("Positions of TX PDO 0x{:X}:", u16::from(pdo.idx));
                }
            } else {
                config.config_sm_pdos(SmCfg::input(sm.idx), &pds)?;
                // Positions of RX PDO
                for pdo in &pds {
                    log::debug!("Positions of RX PDO 0x{:X}:", u16::from(pdo.idx));
                }
            }
            for pdo in pds {
                for entry in &pdo.entries {
                    let offset = config.register_pdo_entry(entry.entry_idx, domain_idx)?;
                    let name = entry.name.clone();
                    if entry_offsets.contains_key(&name) {
                        entry_offsets.get_mut(&name).unwrap().push((
                            entry.entry_idx,
                            entry.bit_len,
                            offset,
                        ));
                    } else {
                        entry_offsets.insert(name, vec![(entry.entry_idx, entry.bit_len, offset)]);
                    }
                    if mailbox[i] && direction[i] < 0 {
                        // add the input mailbox to the list
                        mailbox_entires.push(entry.name.clone());
                    }
                }
            }
        }

        let cfg_index = config.index();

        let cfg_info = master.get_config_info(cfg_index)?;
        log::debug!("Config info: {:#?}", cfg_info);
        if cfg_info.slave_position.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Unable to configure slave",
            ));
        }
        offsets.insert(SlavePos::new(i as u16), entry_offsets);
        mailbox_pdos.insert(SlavePos::new(i as u16), mailbox_entires);
    }

    Ok((master, domain_idx, offsets, slave_names, mailbox_pdos))
}

// log the pdo offsets
fn log_pdo_offsets(offsets: &SlaveOffsets) {
    for (s, o) in offsets {
        log::debug!("PDO offsets of Slave {}:", u16::from(*s));
        for (name, pdos) in o {
            for (pdo, bit_len, offset) in pdos {
                log::debug!(
                    " - \"{}\" : {:X}:{:X} - {:?}, bit length: {}",
                    name,
                    u16::from(pdo.idx),
                    u8::from(pdo.sub_idx),
                    offset,
                    bit_len
                );
            }
        }
    }
}

// create a function to map slave id to slave name
fn create_slave_name_mapper(slave_names: SlaveNames) -> impl Fn(u16) -> String {
    move |id: u16| -> String {
        slave_names
            .iter()
            .find(|(_, sid)| u16::from(**sid) == id)
            .unwrap()
            .0
            .clone()
    }
}

// set the ready flag with mutex
fn set_ready_flag(condvar: &Arc<(Mutex<bool>, Condvar)>, flag: bool) {
    let (lock, cvar) = &**condvar;
    let mut ready = lock.lock().unwrap();
    *ready = flag;
    cvar.notify_one();
}

// notify the slave state with mutex
fn notify_slave_state(condvar: &Arc<(Mutex<Vec<u8>>, Condvar)>, state: Vec<u8>) {
    let (lock, cvar) = &**condvar;
    let mut sstate = lock.lock().unwrap();
    *sstate = state;
    cvar.notify_one();
}

fn notify_next_cycle(condvar: &Arc<(Mutex<bool>, Condvar)>) {
    let (lock, cvar) = &**condvar;
    let mut next_cycle = lock.lock().unwrap();
    *next_cycle = true;
    cvar.notify_one();
}

// Function to get the current state of a slave
fn get_slave_current_state(
    master: &Master,
    slave_pos: SlavePos,
    slave_name_from_id: &impl Fn(u16) -> String,
    #[cfg(feature = "verify_mailbox_pdos")] slave_is_mailbox_pdo_responding: bool,
    #[cfg(feature = "enable_watchdog")] slave_is_watchdog_responding: bool,
) -> u8 {
    #[cfg(feature = "verify_mailbox_pdos")]
    if !slave_is_mailbox_pdo_responding {
        log::error!(
            "Slave {:?} (pos: {:?}) is not responding (mailbox check failed)!",
            slave_name_from_id(slave_pos.into()),
            slave_pos
        );
        return 0;
    }
    #[cfg(feature = "enable_watchdog")]
    if !slave_is_watchdog_responding {
        log::error!(
            "Slave {:?} (pos: {:?}) is not responding (watchdog check failed)!",
            slave_name_from_id(slave_pos.into()),
            slave_pos
        );
        return 0;
    }

    match master.get_slave_info(slave_pos) {
        Ok(info) => {
            if info.al_state != AlState::Op {
                log::error!(
                    "Slave {:?} is not operational! State: {:?}",
                    info.name,
                    info.al_state
                );
                0
            } else {
                AlState::Op as u8
            }
        }
        Err(_) => {
            log::error!(
                "Failed to get slave info for slave {:?}, name: {:?}",
                slave_pos,
                slave_name_from_id(slave_pos.into())
            );
            255
        }
    }
}

// Function that logs the current state of the master
fn log_master_state(
    master: &Master,
    slave_number: u32,
    #[cfg(feature = "verify_mailbox_pdos")] maibox_timeout_ms: u32,
    #[cfg(feature = "enable_watchdog")] watchdog_timeout_ms: u32,
    slave_name_from_id: &impl Fn(u16) -> String,
    #[cfg(feature = "verify_mailbox_pdos")] slave_is_mailbox_pdo_responding: &Vec<bool>,
    #[cfg(feature = "enable_watchdog")] slave_is_watchdog_responding: &Vec<bool>,
) {
    let m_state = master.state().unwrap();
    log::debug!(
        "Master State: {:?}, Link up: {}, Slaves connected: {} out of {}",
        m_state.al_states,
        m_state.link_up,
        m_state.slaves_responding,
        slave_number
    );
    if m_state.al_states != AlState::Op as u8 {
        log::error!("Master is not operational! State: {:?}", m_state.al_states);
    }
    if !m_state.link_up {
        log::error!("Link is not up!");
    }
    if m_state.slaves_responding < slave_number {
        log::error!(
            "Not all slaves are connected! Expected: {}, Responding: {}",
            slave_number,
            m_state.slaves_responding
        );
    }
    if m_state.slaves_responding > slave_number {
        log::error!(
            "New slaves are connected! Inintially: {}, Now: {}",
            slave_number,
            m_state.slaves_responding
        );
    }

    // print the state of each slave
    log::info!("Connected slaves:");
    for i in 0..slave_number {
        match master.get_slave_info(SlavePos::from(i as u16)) {
            Ok(info) => {
                if info.al_state == AlState::Op {
                    log::info!(
                        "Slave {:?} (id: {}) is connected and operational! State: {:?}",
                        info.name,
                        i,
                        info.al_state
                    );
                } else {
                    log::warn!(
                        "Slave {:?} (id: {}) is connected but not operational! State: {:?}",
                        info.name,
                        i,
                        info.al_state
                    );
                }
            }
            Err(_) => {
                log::error!("Slave {:?} not connected!", i);
            }
        }
    }

    // notify the operational state to the master
    #[cfg(feature = "verify_mailbox_pdos")]
    if !slave_is_mailbox_pdo_responding.iter().all(|&r| r) {
        log::error!("Not all slaves are responding!");
        for i in 0..slave_number {
            if !slave_is_mailbox_pdo_responding[i as usize] {
                log::error!(
                    "Poulpe {:?} (pos: {:?}) not responding for more than {}ms",
                    slave_name_from_id(i as u16),
                    i,
                    maibox_timeout_ms
                );
            }
        }
    }

    #[cfg(feature = "enable_watchdog")]
    if !slave_is_watchdog_responding.iter().all(|&r| r) {
        log::error!("Not all slaves have watchdog!");
        for i in 0..slave_number {
            if !slave_is_watchdog_responding[i as usize] {
                log::error!(
                    "Poulpe {:?} (pos: {:?}) watchdog not responding for more than {}ms",
                    slave_name_from_id(i as u16),
                    i,
                    watchdog_timeout_ms
                );
            }
        }
    }
}
