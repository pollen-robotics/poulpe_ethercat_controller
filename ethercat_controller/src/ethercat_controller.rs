use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read},
    ops::Range,
    sync::{
        mpsc::{sync_channel, SyncSender},
        Arc, Condvar, Mutex, RwLock,
    },
    thread,
    time::Duration,
};

use ethercat::{
    AlState, DomainIdx, Master, MasterAccess, Offset, PdoCfg, PdoEntryIdx, PdoEntryInfo,
    PdoEntryPos, SlaveAddr, SlaveId, SlavePos, SmCfg,SmIdx, PdoPos, PdoIdx
};

#[derive(Debug)]
pub struct EtherCatController {
    offsets: SlaveOffsets,
    slave_names: SlaveNames,

    data_lock: Arc<RwLock<Option<Vec<u8>>>>,
    ready_condvar: Arc<(Mutex<bool>, Condvar)>,
    cycle_condvar: Arc<(Mutex<bool>, Condvar)>,
    slave_states_condvar: Arc<(Mutex<Vec<u8>>, Condvar)>,

    cmd_buff: SyncSender<(Range<usize>, Vec<u8>)>,
}

impl EtherCatController {
    pub fn open(
        master_id: u32,
        cycle_period: Duration,
    ) -> Result<Self, io::Error> {
        let (mut master, domain_idx, offsets, slave_names, mailbox_entries) = init_master(master_id)?;

        master.activate()?;

        for (s, o) in &offsets {
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

        let data_lock = Arc::new(RwLock::new(None));
        let write_data_lock = Arc::clone(&data_lock);

        let ready_condvar = Arc::new((Mutex::new(false), Condvar::new()));
        let write_ready_condvar = Arc::clone(&ready_condvar);

        let cycle_condvar = Arc::new((Mutex::new(false), Condvar::new()));
        let write_cycle_condvar = Arc::clone(&cycle_condvar);

        let slave_number = slave_names.len() as u32;

        let slave_states_condvar = Arc::new((Mutex::new(vec![0]), Condvar::new()));
        let sstate_condvar = Arc::clone(&slave_states_condvar);

        let (tx, rx) = sync_channel::<(Range<usize>, Vec<u8>)>(5); // TODO: make buffer size configurable

        let mut master_operational = false;

        //copy slave names to be used with the thread
        let snames = slave_names.clone(); 
        let slave_name_from_id = move |id: u16| -> String {
            snames.iter().find(|(_, sid)| u16::from(**sid) == id).unwrap().0.clone()
        };

        #[cfg(feature = "verify_mailboxes")]
        // find the number of axis offset for all slaves
        log::info!("Tracking mailbox last sent data: {:?}", mailbox_entries);
        #[cfg(feature = "verify_mailboxes")]
        let mut slave_mailbox_offsets= vec![];
        #[cfg(feature = "verify_mailboxes")]
        for i in 0..slave_number {
            let mut mailbox_offsets = vec![];
            for m in mailbox_entries.get(&SlavePos::from(i as u16)).unwrap() {
                mailbox_offsets.append(&mut get_reg_addr_ranges(offsets.clone(), i as u16, m));
            }
            slave_mailbox_offsets.push(mailbox_offsets);
        }

        #[cfg(feature = "verify_mailboxes")]
        // create a timestamp for each slave
        let mut slave_mailbox_timestamps = vec![std::time::Instant::now(); slave_number as usize];
        #[cfg(feature = "verify_mailboxes")]
        let mut slave_is_mailbox_responding = vec![true; slave_number as usize];
        #[cfg(feature = "verify_mailboxes")]
        let mut slave_mailbox_data_buffer: Vec<Vec<Vec<u8>>> = vec![vec![]; slave_number as usize];


        let mut last_call_timestamp = std::time::Instant::now();
        thread::spawn(move || loop {
            master.receive().unwrap();
            master.domain(domain_idx).process().unwrap();
            master.domain(domain_idx).queue().unwrap();

            let mut all_slaves_responding = true;

            let data = master.domain_data(domain_idx).unwrap();


            // verify that the poulpes are still writing by
            // checking the type value it should never be zero
            all_slaves_responding = true;

            #[cfg(feature = "verify_mailboxes")]
            {
                for i in 0..slave_number {
                    let offset  = match slave_mailbox_offsets.get(i as usize){
                        Some(offset) => offset,
                        None => {
                            log::error!("Slave {:?} (pos {:?}) seems to be connected after master init!", slave_name_from_id(i as u16), i);
                            continue; 
                        }
                    };
                    // load the data 
                    let mut mailbox_data = vec![];
                    for range in offset {
                        mailbox_data.push(data[range.clone()].to_vec());
                    }
                    log::debug!("{:?}", mailbox_data);
                    //check if all zeros
                    let mut is_all_zero = true;
                    for d in mailbox_data.iter(){
                        if d.iter().any(|&x| x != 0){
                            is_all_zero = false;
                            break;
                        }
                    }
                    if is_all_zero{
                        if slave_mailbox_timestamps[i as usize].elapsed().as_millis() > 1000 {
                            log::error!("Poulpe {:?} (pos: {:?}) not responding for more than 1s", slave_name_from_id(i as u16), i);
                            // set the ready flag to false
                            let (lock, cvar) = &*write_ready_condvar;
                            let mut ready = lock.lock().unwrap();
                            *ready = false;
                            cvar.notify_one();
                            all_slaves_responding = false;
                            slave_is_mailbox_responding[i as usize] = false;
                            // slave_mailbox_data_buffer[i as usize];
                        }
                    }else{
                        slave_mailbox_timestamps[i as usize] = std::time::Instant::now();
                        slave_is_mailbox_responding[i as usize] = true;
                        slave_mailbox_data_buffer[i as usize] = mailbox_data;
                    }
                    // save the new data to the buffer
                    if all_slaves_responding{
                        for (j, range) in offset.iter().enumerate(){
                            if slave_mailbox_data_buffer[i as usize].len() > j{
                                data[range.clone()].copy_from_slice(&slave_mailbox_data_buffer[i as usize][j]);
                            }
                        }
                    }
                }
            }

            if let Ok(mut write_guard) = write_data_lock.write() {
                *write_guard = Some(data.to_vec());
            }


            {
                let (lock, cvar) = &*write_cycle_condvar;
                let mut next_cycle = lock.lock().unwrap();
                *next_cycle = true;
                cvar.notify_one();
            }

            while let Ok((reg_addr_range, value)) = rx.try_recv() {
                data[reg_addr_range].copy_from_slice(&value);
            }

            master.send().unwrap();

            
            let m_state = master.state().unwrap();
            if !master_operational {
                if !all_slaves_responding {
                    continue;
                }
                log::debug!("Current state {:?}", m_state);
                if m_state.link_up && m_state.al_states == 8 { // 8 is operational
                    // notify the operational state to the master
                    let (lock, cvar) = &*write_ready_condvar;
                    let mut ready = lock.lock().unwrap();
                    *ready = true;
                    cvar.notify_one();
                    master_operational = true;
                    // notify the operational state to the slaves
                    let (lock, cvar) = &*sstate_condvar;
                    let mut sstate = lock.lock().unwrap();
                    *sstate = vec![AlState::Op as u8; slave_number as usize];
                    cvar.notify_one();

                    log::info!("Master and all slaves operational!");
                }
            }else{
                // check if all slaves are connected
                // thsis will fail if a lave is disconnected
                // as well as if we connect more slaves than expected
                if m_state.slaves_responding < slave_number {
                    log::error!("Not all slaves are connected! Expected: {}, Responding: {}", slave_number, m_state.slaves_responding);
                }
                if m_state.slaves_responding > slave_number {
                    log::error!("New slaves are connected! Inintially: {}, Now: {}", slave_number, m_state.slaves_responding);
                }
                if m_state.al_states != 8 || !all_slaves_responding{
                    if m_state.al_states != 8 {
                        log::error!("Master is not operational! State: {:?}", m_state.al_states);
                    }
                    if !all_slaves_responding {
                        log::error!("Not all slaves are responding!");
                    }
                    let mut slave_current_state : Vec<u8> = vec![];
                    // check which slaves are connected
                    for i in 0..slave_number {
                        let slave_pos = SlavePos::from(i as u16);
                        match master.get_slave_info(slave_pos){
                            Ok(info) =>{
                                // check if slave operational
                                let mut is_operational = info.al_state == AlState::Op;

                                #[cfg(feature = "verify_mailboxes")]
                                { is_operational = is_operational && slave_is_mailbox_responding[i as usize] == false; }
                                if is_operational{
                                    log::error!("Slave {:?} is not operational! State: {:?}", info.name, info.al_state);
                                    slave_current_state.push(0);
                                }else{
                                    slave_current_state.push(AlState::Op as u8);
                                }
                            },
                            Err(_) => {
                                log::error!("Failed to get slave info for slave {:?}, name: {:?}", slave_pos, slave_name_from_id(i as u16));
                                slave_current_state.push(255);
                            }
                        };                        
                    }
                    // set the ready flag to false
                    let (lock, cvar) = &*write_ready_condvar;
                    let mut ready = lock.lock().unwrap();
                    *ready = false;
                    cvar.notify_one();
                    master_operational = false;
                    // notify the operational state for the slaves
                    let (lock, cvar) = &*sstate_condvar;
                    let mut sstate = lock.lock().unwrap();
                    *sstate = slave_current_state;
                    cvar.notify_one();
                }
            }
            // sleep a small time
            thread::sleep(Duration::from_secs_f32(0.0001));
        });

        Ok(EtherCatController {
            offsets,
            slave_names,
            data_lock,
            ready_condvar,
            cycle_condvar,
            slave_states_condvar,
            cmd_buff: tx,
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
        get_reg_addr_range(self.offsets.clone(), slave_id, register, index)
    }

    fn get_reg_addr_ranges(&self, slave_id: u16, register: &String) -> Vec<Range<usize>> {
        get_reg_addr_ranges(self.offsets.clone(), slave_id, register)
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

pub fn get_reg_addr_range(offsets: SlaveOffsets, slave_id: u16, register: &String, index: usize) -> Range<usize> {
    let slave_pos = SlavePos::from(slave_id);

    let (_pdo_entry_idx, bit_len, offset) = offsets[&slave_pos][register][index];
    let addr = offset.byte;
    let bytes_len = (bit_len / 8) as usize;

    addr..addr + bytes_len
}

fn get_reg_addr_ranges(offsets: SlaveOffsets, slave_id: u16, register: &String) -> Vec<Range<usize>> {
    let slave_pos = SlavePos::from(slave_id);

    let mut ranges = vec![];
    for i in  0..offsets[&slave_pos][register].len(){
        ranges.push(get_reg_addr_range(offsets.clone(), slave_id, register, i));
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
