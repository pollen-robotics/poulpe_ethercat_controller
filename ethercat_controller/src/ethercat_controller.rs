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
    AlState, DomainIdx, Master, Offset, PdoCfg, PdoEntryIdx, MasterAccess, PdoEntryInfo, PdoEntryPos, SlaveAddr, SlaveId,
    SlavePos, SmCfg,
};
use ethercat_esi::EtherCatInfo;

#[derive(Debug)]
pub struct EtherCatController {
    offsets: SlaveOffsets,

    data_lock: Arc<RwLock<Option<Vec<u8>>>>,
    ready_condvar: Arc<(Mutex<bool>, Condvar)>,
    cycle_condvar: Arc<(Mutex<bool>, Condvar)>,

    cmd_buff: SyncSender<(Range<usize>, Vec<u8>)>,
}

impl EtherCatController {
    pub fn open(
        filename: &String,
        master_id: u32,
        cycle_period: Duration,
    ) -> Result<Self, io::Error> {
        let (mut master, domain_idx, offsets) = init_master(filename, master_id)?;

        master.activate()?;

        for (s, o) in &offsets {
            log::debug!("PDO offsets of Slave {}:", u16::from(*s));
            for (name, pdos    ) in o {
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

        let (tx, rx) = sync_channel::<(Range<usize>, Vec<u8>)>(5);

        let mut is_ready = false;

        thread::spawn(move || loop {
            master.receive().unwrap();
            master.domain(domain_idx).process().unwrap();
            master.domain(domain_idx).queue().unwrap();

            let data = master.domain_data(domain_idx).unwrap();

            log::debug!("{:?}", &data);

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

            if !is_ready {
                let m_state = master.state().unwrap();
                log::debug!("Current state {:?}", m_state);

                if m_state.link_up  && m_state.al_states == 8 { 
                    let (lock, cvar) = &*write_ready_condvar;
                    let mut ready = lock.lock().unwrap();
                    *ready = true;
                    cvar.notify_one();
                    is_ready = true;

                    log::info!("Master ready!");
                }
            }

            thread::sleep(cycle_period);
        });

        Ok(EtherCatController {
            offsets,
            data_lock,
            ready_condvar,
            cycle_condvar,
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

    pub fn get_pdo_register(&self, slave_id: u16, register: &String, index : usize) -> Option<Vec<u8>> {
        let reg_addr_range = self.get_reg_addr_range(slave_id, register, index);

        (*self.data_lock.read().unwrap())
            .as_ref()
            .map(|data| data[reg_addr_range].to_vec())
    }

    pub fn set_pdo_register(&self, slave_id: u16, register: &String, index : usize, value: Vec<u8> ) {
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

    pub fn set_pdo_registers(&self, slave_id: u16, register: &String, values: Vec<Vec<u8>> ) {
        let reg_addr_ranges = self.get_reg_addr_ranges(slave_id, register);

        if values.len() != reg_addr_ranges.len() {
            // log::error!("values: {:?}", values);
            log::warn!("Values length does not match register count, using first {} elements!",reg_addr_ranges.len());
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
        let slave_pos = SlavePos::from(slave_id);

        let (_pdo_entry_idx, bit_len, offset) = self.offsets[&slave_pos][register][index];
        let addr = offset.byte;
        let bytes_len = (bit_len / 8) as usize;

        addr..addr + bytes_len
    }

    fn get_reg_addr_ranges(&self, slave_id: u16, register: &String) -> Vec<Range<usize>> {
        let slave_pos = SlavePos::from(slave_id);

        let pdos = self.offsets[&slave_pos][register].clone();
        
        let mut ranges = Vec::new();
        for (pdo, bit_len, offset) in pdos {
            let addr = offset.byte;
            let bytes_len = (bit_len / 8) as usize;
            ranges.push(addr..addr + bytes_len);
        }
        ranges
    }
}

type PdoOffsets = HashMap<String, Vec<(PdoEntryIdx, u8, Offset)>>;
type SlaveOffsets = HashMap<SlavePos, PdoOffsets>;


pub fn init_master(
    filename: &String,
    idx: u32,
) -> Result<(Master, DomainIdx, SlaveOffsets), io::Error> {
    let mut esi_file = File::open(filename)?;

    let mut esi_xml_str = String::new();
    esi_file.read_to_string(&mut esi_xml_str)?;

    let esi = EtherCatInfo::from_xml_str(&esi_xml_str)?;

    let mut master = Master::open(idx, MasterAccess::ReadWrite)?;
    log::debug!("Reserve master");
    master.reserve()?;
    log::debug!("Create domain");
    let domain_idx = master.create_domain()?;
    let mut offsets: SlaveOffsets = HashMap::new();


    esi.description.devices.iter().for_each(|dev| {
        log::debug!("Device: {}, Sync managers {:?}", dev.name, dev.sm);
        dev.rx_pdo.iter().for_each(|pdo| {
            log::debug!("  RxPDO: {:?}, SM: {:?}", pdo.idx, pdo.sm);
            pdo.entries.iter().for_each(|entry| {
                log::debug!("    Entry: {:?} - {}", entry.entry_idx, entry.name.as_ref().unwrap_or(&"".to_string()));
            });
        });
        dev.tx_pdo.iter().for_each(|pdo| {
            log::debug!("  TxPDO: {:?}, SM: {:?}", pdo.idx, pdo.sm);
            pdo.entries.iter().for_each(|entry| {
                log::debug!("    Entry: {:?} - {}", entry.entry_idx, entry.name.as_ref().unwrap_or(&"".to_string()));
            });
        });
    });

    for (dev_nr, dev) in esi.description.devices.iter().enumerate() {
        let slave_pos = SlavePos::from(dev_nr as u16);
        log::debug!("Request PreOp state for {:?}", slave_pos);
        master.request_state(slave_pos, AlState::PreOp)?;
        let slave_info = master.get_slave_info(slave_pos)?;
        log::info!("Found device {}:{:?}", dev.name, slave_info);
        let slave_addr = SlaveAddr::ByPos(dev_nr as u16);
        let slave_id = SlaveId {
            vendor_id: esi.vendor.id,
            product_code: dev.product_code,
        };
        let mut config = master.configure_slave(slave_addr, slave_id)?;
        let mut entry_offsets: PdoOffsets = HashMap::new();

        // display syncs 
        log::debug!("Device: {}, Sync managers {:?}", dev.name, dev.sm);

        log::debug!("no rx_pdo: {}", dev.rx_pdo.len());
        log::debug!("no tx_pdo: {}", dev.tx_pdo.len());
        
        let rx_smidxs = dev.rx_pdo.iter().map(|pdo| pdo.sm).collect::<Vec<_>>();
        let tx_smidxs = dev.tx_pdo.iter().map(|pdo| pdo.sm).collect::<Vec<_>>();
        log::debug!("RX SMIDX: {:?}", rx_smidxs);
        log::debug!("TX SMIDX: {:?}", tx_smidxs);
        

        let rx_pdos: Vec<PdoCfg> = dev
            .rx_pdo
            .iter()
            .map(|pdo| PdoCfg {
                idx: pdo.idx,
                entries: pdo
                    .entries
                    .iter()
                    .enumerate()
                    .map(|(i, e)| PdoEntryInfo {
                        entry_idx: e.entry_idx,
                        bit_len: e.bit_len as u8,
                        name: e.name.clone().unwrap_or_default(),
                        pos: PdoEntryPos::from(i as u8),
                    })
                    .collect(),
            })
            .collect();

        let tx_pdos: Vec<PdoCfg> = dev
            .tx_pdo
            .iter()
            .map(|pdo| PdoCfg {
                idx: pdo.idx,
                entries: pdo
                    .entries
                    .iter()
                    .enumerate()
                    .map(|(i, e)| PdoEntryInfo {
                        entry_idx: e.entry_idx,
                        bit_len: e.bit_len as u8,
                        name: e.name.clone().unwrap_or_default(),
                        pos: PdoEntryPos::from(i as u8),
                    })
                    .collect(),
            })
            .collect();

        let mut i = 0;
        for  pdo in &rx_pdos {
            config.config_sm_pdos(SmCfg::output(rx_smidxs[i]), &[pdo.clone()])?;
            i += 1;
            // Positions of RX PDO  
            log::debug!("Positions of RX PDO 0x{:X}:", u16::from(pdo.idx));
            for entry in &pdo.entries {
                let offset = config.register_pdo_entry(entry.entry_idx, domain_idx)?;
                let name = entry.name.clone();
                if entry_offsets.contains_key(&name){
                    entry_offsets.get_mut(&name).unwrap().push((entry.entry_idx, entry.bit_len, offset));
                }else{
                    entry_offsets.insert(name, vec!((entry.entry_idx, entry.bit_len, offset)));
                }
            }
        }
        i = 0;
        for pdo in &tx_pdos {
            config.config_sm_pdos(SmCfg::input(tx_smidxs[i]), &[pdo.clone()])?;
            i += 1;
            // Positions of TX PDO
            log::debug!("Positions of TX PDO 0x{:X}:", u16::from(pdo.idx));
            for entry in &pdo.entries {
                let offset = config.register_pdo_entry(entry.entry_idx, domain_idx)?;
                let name = entry.name.clone();
                if entry_offsets.contains_key(&name){
                    entry_offsets.get_mut(&name).unwrap().push((entry.entry_idx, entry.bit_len, offset));
                }else{
                    entry_offsets.insert(name, vec!((entry.entry_idx, entry.bit_len, offset)));
                }
            }
        }

        let cfg_index = config.index();



        let cfg_info = master.get_config_info(cfg_index)?;
        log::info!("Config info: {:#?}", cfg_info);
        if cfg_info.slave_position.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Unable to configure slave",
            ));
        }
        offsets.insert(slave_pos, entry_offsets);
    }
    Ok((master, domain_idx, offsets))
}