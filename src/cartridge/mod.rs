pub mod header;
mod mapper;

use crate::memory::ROM;
use header::Header;
use mapper::*;
use std::io::Read;
use tracing::{event, Level};

pub trait CartridgeInterface {
    fn get_name(&self) -> String;
    fn prg_read(&self, addr: u16) -> u8;
    fn prg_write(&mut self, addr: u16, val: u8);
    fn header(&self) -> Header;
    fn dpcm(&self) -> ROM;
    fn chr(&self) -> ROM;
}

#[derive(Debug, Default)]
pub struct Cartridge {
    name: String,
    header: Header,

    // This may not need to be a box - we can instantiate a new type for each mapper fine
    mapper: Box<dyn Mapper>,
}

impl CartridgeInterface for Cartridge {
    fn get_name(&self) -> String {
        self.name.to_owned()
    }

    fn prg_read(&self, addr: u16) -> u8 {
        self.mapper.prg_read(addr)
    }

    fn prg_write(&mut self, addr: u16, val: u8) {
        // dpcm_read assumes that these bytes never change. If this happens we have to update how
        // we pass the samples to the APU
        assert!(addr <= 0xC000);

        self.mapper.prg_write(addr, val);
    }

    fn header(&self) -> Header {
        self.header.clone()
    }

    fn dpcm(&self) -> ROM {
        self.mapper.dpcm()
    }

    fn chr(&self) -> ROM {
        self.mapper.chr()
    }
}

pub fn load_cartridge(filename: &str) -> Result<Cartridge, std::io::Error> {
    event!(Level::INFO, "Loading ROM: {:?}", filename);

    let mut fh = std::fs::File::open(filename)?;
    let mut header: [u8; 16] = [0; 16];
    fh.read_exact(&mut header)?;
    let header = Header::from(&header);
    let data_size = header.get_prg_rom_size() + header.get_chr_ram_size();
    let mut data = vec![0; data_size as usize];
    fh.read_exact(&mut data)?;

    let mapper = create_mapper(&header, &data);
    Ok(Cartridge {
        header,
        name: filename.to_owned(),
        mapper,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn load_none() {
        let rom = load_cartridge("NoFile.nes");
        assert!(rom.is_err());
        assert!(match rom.err() {
            Some(e) => e.kind() == ErrorKind::NotFound,
            None => false,
        });
    }

    #[ignore = "unimplemented mapper3"]
    #[test]
    fn load_some() {
        let exp_name = "nes-test-roms/cpu_dummy_reads/cpu_dummy_reads.nes";
        let cart = match load_cartridge(exp_name) {
            Ok(cart) => cart,
            Err(e) => unreachable!("Error {:?}", e),
        };
        assert_eq!(cart.get_name(), exp_name);
    }
}
