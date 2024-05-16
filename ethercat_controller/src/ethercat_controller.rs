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
    DomainIdx, Master, Offset, PdoCfg, PdoEntryIdx, PdoEntryInfo, PdoEntryPos, SlaveAddr, SlaveId,
    SlavePos, SmCfg,
};
use ethercat_esi::EtherCatInfo;

#[derive(Debug)]
pub struct EtherCatController {
    offsets: HashMap<SlavePos, HashMap<String, (PdoEntryIdx, u8, Offset)>>,

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
            for (name, (pdo, bit_len, offset)) in o {
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

                if m_state.link_up {
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

    pub fn get_pdo_register(&self, slave_id: u16, register: &String) -> Option<Vec<u8>> {
        let reg_addr_range = self.get_reg_addr_range(slave_id, register);

        (*self.data_lock.read().unwrap())
            .as_ref()
            .map(|data| data[reg_addr_range].to_vec())
    }

    pub fn set_pdo_register(&self, slave_id: u16, register: &String, value: Vec<u8>) {
        let reg_addr_range = self.get_reg_addr_range(slave_id, register);

        self.cmd_buff.send((reg_addr_range, value)).unwrap();
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

    fn get_reg_addr_range(&self, slave_id: u16, register: &String) -> Range<usize> {
        let slave_pos = SlavePos::from(slave_id);

        let (_pdo_entry_idx, bit_len, offset) = self.offsets[&slave_pos][register];
        let addr = offset.byte;
        let bytes_len = (bit_len / 8) as usize;

        addr..addr + bytes_len
    }
}

type SlaveOffsets = HashMap<SlavePos, HashMap<String, (PdoEntryIdx, u8, Offset)>>;

pub fn init_master(
    filename: &String,
    idx: u32,
) -> Result<(Master, DomainIdx, SlaveOffsets), io::Error> {
    let mut esi_file = File::open(filename)?;

    let mut esi_xml_str = String::new();
    esi_file.read_to_string(&mut esi_xml_str)?;

    let esi = EtherCatInfo::from_xml_str(&esi_xml_str)?;

    let mut master = Master::open(idx, ethercat::MasterAccess::ReadWrite)?;
    master.reserve()?;

    let domain_idx = master.create_domain()?;

    let mut offsets: HashMap<SlavePos, HashMap<String, (PdoEntryIdx, u8, Offset)>> = HashMap::new();

    for (dev_nr, dev) in esi.description.devices.iter().enumerate() {
        let slave_pos = SlavePos::from(dev_nr as u16);
        log::debug!("Request PreOp state for {:?}", slave_pos);

        master.request_state(slave_pos, ethercat::AlState::PreOp)?;

        let slave_info = master.get_slave_info(slave_pos)?;
        log::debug!("Found device {} : {:?}", dev.name, slave_info);

        let slave_addr = SlaveAddr::ByPos(dev_nr as u16);
        let slave_id = SlaveId {
            vendor_id: esi.vendor.id,
            product_code: dev.product_code,
        };

        let mut config = master.configure_slave(slave_addr, slave_id)?;
        let mut entry_offsets: HashMap<String, (PdoEntryIdx, u8, Offset)> = HashMap::new();

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

        let output = SmCfg::output(2.into());
        let input = SmCfg::input(3.into());

        config.config_sm_pdos(output, &rx_pdos)?;
        config.config_sm_pdos(input, &tx_pdos)?;

        for pdo in &rx_pdos {
            for entry in &pdo.entries {
                let offset = config.register_pdo_entry(entry.entry_idx, domain_idx)?;
                entry_offsets.insert(entry.name.clone(), (entry.entry_idx, entry.bit_len, offset));
            }
        }
        for pdo in &tx_pdos {
            for entry in &pdo.entries {
                let offset = config.register_pdo_entry(entry.entry_idx, domain_idx)?;
                entry_offsets.insert(entry.name.clone(), (entry.entry_idx, entry.bit_len, offset));
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
        offsets.insert(slave_pos, entry_offsets);
    }

    Ok((master, domain_idx, offsets))
}
