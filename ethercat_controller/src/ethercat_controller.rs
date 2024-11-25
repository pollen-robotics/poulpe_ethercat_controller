use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read},
    ops::Range,
    sync::{
        Arc, Condvar, Mutex, RwLock,
    },
    thread,
    time::Duration,
};

use ethercat::{
    AlState, DomainIdx, Master, MasterAccess, Offset, PdoCfg, PdoEntryIdx, PdoEntryInfo,
    PdoEntryPos, SlaveAddr, SlaveId, SlavePos, SmCfg,SmIdx, PdoPos, PdoIdx
};

use crossbeam_channel::{bounded, Sender, Receiver};

#[derive(Debug)]
pub struct EtherCatController {
    offsets: SlaveOffsets,
    slave_names: SlaveNames,

    data_lock: Arc<RwLock<Option<Vec<u8>>>>,
    ready_condvar: Arc<(Mutex<bool>, Condvar)>,
    cycle_condvar: Arc<(Mutex<bool>, Condvar)>,
    slave_states_condvar: Arc<(Mutex<Vec<u8>>, Condvar)>,

    cmd_buff: Sender<(Range<usize>, Vec<u8>)>,

    pub command_drop_time_us: u32
}

impl EtherCatController {
    pub fn open(
        master_id: u32,
        cycle_period: Duration,
        command_drop_time_us: u32,
        mailbox_wait_time_ms: u32
    ) -> Result<Self, io::Error> {
        let (mut master, domain_idx, offsets, slave_names, mailbox_entries) = init_master(master_id)?;

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

        // get the slave number
        let slave_number = slave_names.len() as u32;
        // create a function to map slave id to slave name
        let slave_name_from_id = create_slave_name_mapper(slave_names.clone()); 

        // create a sync channel to send data to the master
        // crossbeam_channel is more efficient than std::sync::mcsp::SyncChannel
        let buffer_size = (slave_number*20) as usize;
        let (tx, rx): (crossbeam_channel::Sender<(Range<usize>, Vec<u8>)>, Receiver<(Range<usize>, Vec<u8>)>) = bounded(buffer_size);


        #[cfg(feature = "verify_mailboxes")]
        // initialize the mailbox verification 
        let (mut slave_mailbox_offsets, mut slave_mailbox_timestamps, mut slave_is_mailbox_responding, mut slave_mailbox_data_buffer) = init_mailbox_verification(slave_number, &mailbox_entries, &offsets);


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
                let dt_sleep = cycle_period.as_secs_f32() - loop_period_timestamp.elapsed().as_secs_f32();
                if dt_sleep > 0.0 {
                    thread::sleep(Duration::from_secs_f32(dt_sleep));
                }
                // set the loop period timestamp
                loop_period_timestamp = std::time::Instant::now();
                
                // debugging output
                debug_loop_counter += 1;
                if debug_loop_timestamp.elapsed().as_secs_f32() > 10.0 {
                    log::info!("EtherCAT loop: {:.02} Hz", debug_loop_counter as f32 / debug_loop_timestamp.elapsed().as_secs_f32());
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
                // for each slave check if the mailbox mailbox entries are updated 
                // the mailbox data is being written by slaves at arounf 10Hz
                // if the mailbox data is not updated for more than 1s
                // the slave is considered not as responding
                // 
                // if at least one slave is not responding function will return false
                //
                // if the slaves are responding it will update the data buffer
                // with the mailbox data (which might have been read some time ago (but less than 1s ago))
                #[cfg(feature = "verify_mailboxes")]
                let all_slaves_responding = verify_mailboxes(
                    slave_number,
                    &mut data,
                    &mut slave_mailbox_offsets,
                    &mut slave_mailbox_timestamps,
                    &mut slave_is_mailbox_responding,
                    &mut slave_mailbox_data_buffer,
                    &write_ready_condvar,
                    &slave_name_from_id,
                    mailbox_wait_time_ms
                );

                // write the data to the data lock
                if let Ok(mut write_guard) = write_data_lock.write() {
                    *write_guard = Some(data.to_vec());
                }

                // notify the next cycle
                notify_next_cycle(&write_cycle_condvar);

                // check if the master is operational
                // and only if operational update the data buffer with the new data to send to the slaves
                if master_operational{
                    // check if the RX buffer is getting full!!!
                    // if rx.len() > 40 {log::warn!("RX buffer almost full: {}/{}", rx.len(), buffer_size)}
                    // update the data buffer with the new data to send 
                    while let Ok((reg_addr_range, value)) = rx.try_recv() {
                        data[reg_addr_range].copy_from_slice(&value);
                    }
                }

                // send the data to the slaves
                master.send().unwrap();

                // get the master state
                let m_state = master.state().unwrap();
                #[cfg(not(feature = "verify_mailboxes"))]
                // get the slave states without mailbox verification
                let all_slaves_responding = m_state.slaves_responding == slave_number;


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
                if !master_operational { // master is not operational 

                    // To go to the operational state 
                    // - if all slaves are responding
                    // - if the link is up 
                    // - if the master is in operational state
                    // - if the number of slaves responding is equal to the number of slaves connected (no disconnected or newly connected slaves)
                    if all_slaves_responding 
                        && m_state.link_up 
                        && m_state.al_states == AlState::Op as u8 // OP = 8 is operational
                        && m_state.slaves_responding == slave_number { 
                        // notify the operational state to the master
                        set_ready_flag(&write_ready_condvar, true);
                        master_operational = true;
                        // notify the operational state to the slaves
                        notify_slave_state(&sstate_condvar, vec![AlState::Op as u8; slave_number as usize]);
                        log::info!("Master and all slaves operational!");
                    }else{
                        // check each second
                        if display_not_operational_timestamp.elapsed().as_secs() > 1 {
                            display_not_operational_timestamp = std::time::Instant::now();
                            log::warn!("Master cannot go to operational!");
                            // display the master state
                            // if the master is not operational
                            #[cfg(feature = "verify_mailboxes")]
                            log_master_state(&master, slave_number, &slave_name_from_id, &slave_is_mailbox_responding);
                            #[cfg(not(feature = "verify_mailboxes"))]
                            log_master_state(&master, slave_number, &slave_name_from_id);
                            
                            // kill the master if error recovery not supported
                            #[cfg(not(feature = "recover_from_error"))]
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
                            _ if m_state.slaves_responding < slave_number => log::error!("Not all slaves are connected! Expected: {}, Responding: {}", slave_number, m_state.slaves_responding),
                            _ if m_state.slaves_responding > slave_number => log::error!("New slaves are connected! Inintially: {}, Now: {}", slave_number, m_state.slaves_responding),
                            _ => {}
                        }

                        // update the slave states 
                        // with mailbox verification
                        #[cfg(feature = "verify_mailboxes")]
                        let slave_current_state = (0..slave_number)
                        .map(|i| get_slave_current_state(&master, SlavePos::from(i as u16), &slave_name_from_id, slave_is_mailbox_responding[i as usize]))
                        .collect::<Vec<_>>();
                        // without mailbox verification
                        #[cfg(not(feature = "verify_mailboxes"))]
                        let slave_current_state = (0..slave_number)
                        .map(|i| get_slave_current_state(&master, SlavePos::from(i as u16), &slave_name_from_id))
                        .collect::<Vec<_>>();

                        // notify the operational state for the slaves
                        notify_slave_state(&sstate_condvar, slave_current_state);

                        set_ready_flag(&write_ready_condvar, false);
                        master_operational = false;
                    }

                    // if master state has changed or not all slaves are responding
                    if m_state.al_states != AlState::Op as u8 || !all_slaves_responding {
                        // master state has changed
                        if m_state.al_states != AlState::Op as u8 {
                            log::error!("Master is not operational! State: {:?}", m_state.al_states);
                        }
                        if !all_slaves_responding {
                            // not all slaves are responding
                            log::error!("Not all slaves are responding!");
                        }

                        // update the slave states 
                        // with mailbox verification
                        #[cfg(feature = "verify_mailboxes")]
                        let slave_current_state = (0..slave_number)
                        .map(|i| get_slave_current_state(&master, SlavePos::from(i as u16), &slave_name_from_id, slave_is_mailbox_responding[i as usize]))
                        .collect::<Vec<_>>();
                        // without mailbox verification
                        #[cfg(not(feature = "verify_mailboxes"))]
                        let slave_current_state = (0..slave_number)
                        .map(|i| get_slave_current_state(&master, SlavePos::from(i as u16), &slave_name_from_id))
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
            cmd_buff: tx,
            command_drop_time_us
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
        match states.get(slave_id as usize).map(|s| *s){
            Some(state) => state == (AlState::Op as u8), 
            None => return false
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
}

pub fn get_reg_addr_range(offsets: &SlaveOffsets, slave_id: u16, register: &String, index: usize) -> Range<usize> {
    let slave_pos = SlavePos::from(slave_id);

    let (_pdo_entry_idx, bit_len, offset) = offsets[&slave_pos][register][index];
    let addr = offset.byte;
    let bytes_len = (bit_len / 8) as usize;

    addr..addr + bytes_len
}

fn get_reg_addr_ranges(offsets: &SlaveOffsets, slave_id: u16, register: &String) -> Vec<Range<usize>> {
    let slave_pos = SlavePos::from(slave_id);

    // Fetch data once to minimize locking time
    let register_data = &offsets[&slave_pos][register];

    let mut ranges = Vec::with_capacity(register_data.len());
    for i in 0..register_data.len() {
        ranges.push(get_reg_addr_range(offsets, slave_id, register, i));
    }
    ranges
}

type PdoOffsets = HashMap<String, Vec<(PdoEntryIdx, u8, Offset)>>;
type SlaveOffsets = HashMap<SlavePos, PdoOffsets>;
type SlaveNames = HashMap<String, SlavePos>;
type MailboxEntries = HashMap<SlavePos, Vec<String>>;

pub fn init_master(
    idx: u32,
) -> Result<(Master, DomainIdx, SlaveOffsets, SlaveNames, MailboxEntries), io::Error> {

    // try to open the master
    // if it fails return error
    let mut master = match Master::open(idx, MasterAccess::ReadWrite){
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
    let mut slave_names:SlaveNames = HashMap::new();


    let mut mailboxes: MailboxEntries = HashMap::new();

    let slave_num = master.get_info().unwrap().slave_count;
    log::info!("Found {:?} slaves", slave_num);

    // if there are no slaves connected return error
    if slave_num == 0 {
        log::error!("No slaves found, check slave connections!");
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "No slaves found",
        ));
    }

    for i in 0..slave_num {
        let slave_info = master.get_slave_info(SlavePos::from(i as u16)).unwrap();
        log::info!("Slave {:?} at position {:?}", slave_info.name, i);
        slave_names.insert(slave_info.name.clone(), SlavePos::from(i as u16));
        log::debug!("Found device {:?}", slave_info);
        log::debug!("Vendor ID: {:X}, Product Code: {:X}, SM count {:?}", slave_info.id.vendor_id, slave_info.id.product_code, slave_info.sync_count);
        let slave_addr = SlaveAddr::ByPos(i as u16);
        let slave_id = SlaveId {
            vendor_id: slave_info.id.vendor_id,
            product_code: slave_info.id.product_code,
        };

        let mut pdos  = vec![];
        let mut sms = vec![];
        let mut mailbox = vec![];
        let mut direction = vec![];
        let mut mailbox_entires = vec![];
        for j in  0..slave_info.sync_count{
            let sm_idx = SmIdx::new(j);
            let sm_info = master.get_sync(SlavePos::from(i as u16), sm_idx).unwrap();
            log::debug!("Found sm {:?}, pdo_count {:?}", sm_info, sm_info.pdo_count);

            // sanity check
            if sm_info.pdo_count > 1 {
                log::error!("Only support 1 pdo per sync manager, treating as 1 pdo!");
            }else if sm_info.pdo_count == 0 {
                log::error!("No pdo found in sync manager");
                continue;
            }

            // check if second bit is set
            // if it is its in mailbox mode
            if sm_info.control_register & 0b10 != 0 {
                log::debug!("SM is in mailbox mode!");
                mailbox.push(true);
            }else{
                log::debug!("SM is in buffered mode!");
                mailbox.push(false);
            }

            if sm_info.control_register & 0b100 != 0 {
                log::debug!("Input pdos!");
                direction.push(1);
            }else{
                log::debug!("Output pdos!");
                direction.push(-1);
            }


            let pdo_cfg: PdoCfg = {
                let pdo_info = master.get_pdo(SlavePos::from(i as u16), sm_idx, PdoPos::new(0)).unwrap();
                log::debug!("Found pdo {:?}, entry_count {:?}", pdo_info, pdo_info.entry_count);

                let pdo_entries = (0..pdo_info.entry_count).map(|e| {
                    let entry_info = master.get_pdo_entry(SlavePos::from(i as u16), sm_idx, PdoPos::new(0), PdoEntryPos::new(e)).unwrap();
                    log::debug!("Found entry {:?}, bit_len {:?}", entry_info, entry_info.bit_len);
                    PdoEntryInfo {
                        entry_idx: entry_info.entry_idx,
                        bit_len: entry_info.bit_len as u8,
                        name: entry_info.name.clone(),
                        pos: PdoEntryPos::from(e as u8),
                    }
                }).collect();
                PdoCfg {
                    idx: PdoIdx::new(pdo_info.idx.into()),
                    entries: pdo_entries,
                }
            };
            pdos.push(pdo_cfg);
            sms.push(sm_info)
        }

        let mut config = master.configure_slave(slave_addr, slave_id)?;
        let mut entry_offsets: PdoOffsets = HashMap::new();


        for i in 0..pdos.len() {
            let pdo = pdos[i].clone();
            let sm = sms[i].clone();

            // check if second bit is set
            // if it is its in input mode
            if direction[i] > 0 {
                config.config_sm_pdos(SmCfg::output(sm.idx), &[pdo.clone()])?;
                // Positions of TX PDO
                log::debug!("Positions of TX PDO 0x{:X}:", u16::from(pdo.idx));
            }else{
                config.config_sm_pdos(SmCfg::input(sm.idx), &[pdo.clone()])?;
                // Positions of RX PDO
                log::debug!("Positions of RX PDO 0x{:X}:", u16::from(pdo.idx));
            }
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
                if mailbox[i] &&  direction[i] < 0 {
                    // add the input mailbox to the list
                    mailbox_entires.push(entry.name.clone());
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
        mailboxes.insert(SlavePos::new(i as u16), mailbox_entires);
    }

    Ok((master, domain_idx, offsets, slave_names, mailboxes))
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
            slave_names.iter().find(|(_, sid)| u16::from(**sid) == id).unwrap().0.clone()
        }
    }
    
    #[cfg(feature = "verify_mailboxes")]
    fn init_mailbox_verification(
        slave_number: u32,
        mailbox_entries: &MailboxEntries,
        offsets: &SlaveOffsets,
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
    #[cfg(feature = "verify_mailboxes")]
    fn verify_mailboxes(
        slave_number: u32,
        data: &mut [u8],
        slave_mailbox_offsets: &mut Vec<Vec<Range<usize>>>,
        slave_mailbox_timestamps: &mut Vec<std::time::Instant>,
        slave_is_mailbox_responding: &mut Vec<bool>,
        slave_mailbox_data_buffer: &mut Vec<Vec<Vec<u8>>>,
        write_ready_condvar: &Arc<(Mutex<bool>, Condvar)>,
        slave_name_from_id: &impl Fn(u16) -> String,
        mailbox_wait_time_ms: u32
    ) -> bool{
        // return if all slaves responding 
        let mut all_slaves_responding = true;
        // check each slave
        for i in 0..slave_number {
            // get slave mailbox offset
            let offset = slave_mailbox_offsets[i as usize].clone();
            // get the mailbox data
            let mailbox_data = offset.iter().map(|range| data[range.clone()].to_vec()).collect::<Vec<_>>();
            log::debug!("{:?}", mailbox_data);
            // check if all the values are zero
            let is_all_zeros = mailbox_data.iter().all(|d| d.iter().all(|&x| x == 0));

            // flag to check if slave is responding
            slave_is_mailbox_responding[i as usize] = true;
            // if all the values are zero for more than 1s
            if is_all_zeros{ 
                if slave_mailbox_timestamps[i as usize].elapsed().as_millis() as u32 > mailbox_wait_time_ms {
                    all_slaves_responding &= false; // set the all slaves responding flag to false
                    slave_is_mailbox_responding[i as usize] = false;
                }
            } else {  // if the values are not zero
                slave_mailbox_timestamps[i as usize] = std::time::Instant::now();
                slave_is_mailbox_responding[i as usize] = true;
                slave_mailbox_data_buffer[i as usize] = mailbox_data;
            }

            if  slave_is_mailbox_responding[i as usize] {
                for (j, range) in offset.iter().enumerate() {
                    if slave_mailbox_data_buffer[i as usize].len() > j {
                        data[range.clone()].copy_from_slice(&slave_mailbox_data_buffer[i as usize][j]);
                    }
                }
            }
        }


        return  all_slaves_responding;
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

    #[cfg(not(feature = "verify_mailboxes"))] 
    // Function to get the current state of a slave
    fn get_slave_current_state(
        master: &Master,
        slave_pos: SlavePos,
        slave_name_from_id: &impl Fn(u16) -> String,
    ) -> u8 {
        
        match master.get_slave_info(slave_pos) {
            Ok(info) => {
                if info.al_state != AlState::Op {
                    log::error!("Slave {:?} is not operational! State: {:?}", info.name, info.al_state);
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

    #[cfg(feature = "verify_mailboxes")] 
    // Function to get the current state of a slave
    fn get_slave_current_state(
        master: &Master,
        slave_pos: SlavePos,
        slave_name_from_id: &impl Fn(u16) -> String,
        slave_is_mailbox_responding: bool,
    ) -> u8 {
        
        if !slave_is_mailbox_responding {
            log::error!("Slave {:?} (pos: {:?}) is not responding (mailbox check failed)!", slave_name_from_id(slave_pos.into()), slave_pos);
            return 0;
        }
        match master.get_slave_info(slave_pos) {
            Ok(info) => {
                if info.al_state != AlState::Op {
                    log::error!("Slave {:?} is not operational! State: {:?}", info.name, info.al_state);
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
    fn log_master_state(master: &Master, 
        slave_number: u32, 
        slave_name_from_id: &impl Fn(u16) -> String,
        #[cfg(feature = "verify_mailboxes")] slave_is_mailbox_responding: &Vec<bool>,
    ) {
        let m_state = master.state().unwrap();
        log::debug!("Master State: {:?}, Link up: {}, Slaves connected: {} out of {}", m_state.al_states, m_state.link_up, m_state.slaves_responding, slave_number);
        if m_state.al_states != AlState::Op as u8 {
            log::error!("Master is not operational! State: {:?}", m_state.al_states);
        }
        if !m_state.link_up {
            log::error!("Link is not up!");
        }
        if m_state.slaves_responding < slave_number {
            log::error!("Not all slaves are connected! Expected: {}, Responding: {}", slave_number, m_state.slaves_responding);
        }
        if m_state.slaves_responding > slave_number {
            log::error!("New slaves are connected! Inintially: {}, Now: {}", slave_number, m_state.slaves_responding);
        }

        // notify the operational state to the master
        #[cfg(feature = "verify_mailboxes")]
        if !slave_is_mailbox_responding.iter().all(|&r| r) {
            log::error!("Not all slaves are responding!");
            for i in 0..slave_number {
                if !slave_is_mailbox_responding[i as usize] {
                    log::error!(
                        "Poulpe {:?} (pos: {:?}) not responding for more than 1s",
                        slave_name_from_id(i as u16),
                        i
                    );
                }
            }
        }
    }
