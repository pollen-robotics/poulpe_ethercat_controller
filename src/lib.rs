use std::{
    collections::HashMap,
    convert::TryInto,
    fs::File,
    io::{self, Read}, thread, sync::mpsc,
};

use ethercat::{
    Idx, Master, Offset, PdoCfg, PdoEntryIdx, PdoEntryInfo, PdoEntryPos, PdoIdx, SlaveAddr,
    SlaveId, SlavePos, SmCfg, DomainIdx,
};
use ethercat_esi::EtherCatInfo;

pub struct EtherCatController {
}

pub enum Slave {
    Id0 = 0,
    Id1 = 1,
    Id2 = 2,
}

enum PdoRegister {
    ControlWord,
    ModeOfOperation,
    TargetPosition,
    VelocityOffset,
    TargetTorque,

    StatusWord,
    ModeOfOperationDisplay,
    PositionActualValue,
    VelocityActualValue,
    TorqueActualValue,
    ErrorCode,
}

impl PdoRegister {
    fn addr(&self) -> u16 {
        match *self {
            PdoRegister::ControlWord => todo!(),
            PdoRegister::ModeOfOperation => todo!(),
            PdoRegister::TargetPosition => todo!(),
            PdoRegister::VelocityOffset => todo!(),
            PdoRegister::TargetTorque => todo!(),
            PdoRegister::StatusWord => todo!(),
            PdoRegister::ModeOfOperationDisplay => todo!(),
            PdoRegister::PositionActualValue => todo!(),
            PdoRegister::VelocityActualValue => todo!(),
            PdoRegister::TorqueActualValue => todo!(),
            PdoRegister::ErrorCode => todo!(),
        }
    }
}



impl EtherCatController {
    pub fn open(filename: &String, master_id: u32) -> Result<Self, io::Error> {
        let (mut master, domain_idx, offsets) = init_master(filename, master_id)?;

        master.activate()?;

        for (s, o) in &offsets {
            log::info!("PDO offsets of Slave {}:", u16::from(*s));
            for (name, (pdo, bit_len, offset)) in o {
                log::info!(
                    " - \"{}\" : {:X}:{:X} - {:?}, bit length: {}",
                    name,
                    u16::from(pdo.idx),
                    u8::from(pdo.sub_idx),
                    offset,
                    bit_len
                );
            }
        }

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            loop {
                master.receive().unwrap();
                master.domain(domain_idx).process().unwrap();
                master.domain(domain_idx).queue().unwrap();
    
                let data = master.domain_data(domain_idx).unwrap();
                tx.send(data.clone()).unwrap();
                
                master.send().unwrap();   
            }
        });

        Ok(EtherCatController {})
    }

    pub fn get_pdo_position_actual_value(&self, slave_id: Slave) -> u32 {
        self.get_pdo_register(slave_id, PdoRegister::PositionActualValue)
    }

    fn get_pdo_register<T>(&self, slave_id: Slave, addr: PdoRegister) -> T {
        todo!()
    }

    fn set_pdo_register<T>(&self, slave_id: Slave, addr: PdoRegister, value: T) {
        todo!()
    }
}


fn init_master(
    filename: &String,
    idx: u32,
) -> Result<
    (
        Master,
        DomainIdx,
        HashMap<SlavePos, HashMap<String, (PdoEntryIdx, u8, Offset)>>,
    ),
    io::Error,
> {
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
        log::info!("Found device {} : {:?}", dev.name, slave_info);

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
                        name: e.name.clone().unwrap_or(String::new()),
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
                        name: e.name.clone().unwrap_or(String::new()),
                        pos: PdoEntryPos::from(i as u8),
                    })
                    .collect(),
            })
            .collect();

        let output = SmCfg::output(2.into());
        let input = SmCfg::input(3.into());

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

        config.config_sm_pdos(output, &rx_pdos)?;
        config.config_sm_pdos(input, &tx_pdos)?;

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