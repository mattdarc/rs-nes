mod header;
mod mapper;

use header::Header;
use mapper::*;
use std::cell::RefCell;
use std::io::Read;

pub type Cartridge = RefCell<CartridgeImpl>;

pub trait CartridgeInterface {
    fn get_name(&self) -> String;
    fn load(filename: &str) -> std::io::Result<Cartridge>;
    fn prg_read(&self, addr: usize) -> u8;
    fn prg_write(&self, addr: usize, val: u8);
    fn chr_read(&self, addr: usize) -> u8;
    fn chr_write(&self, addr: usize, val: u8);
}

#[derive(Debug, Default, Clone)]
pub struct CartridgeImpl {
    name: String,

    // This may not need to be a box - we can instantiate a new type for each mapper fine
    mapper: Box<dyn Mapper>,
}

impl CartridgeInterface for Cartridge {
    fn get_name(&self) -> String {
        self.borrow().name.to_owned()
    }

    fn load(filename: &str) -> Result<Cartridge, std::io::Error> {
        let mut fh = std::fs::File::open(filename)?;
        let mut header: [u8; 16] = [0; 16];
        fh.read_exact(&mut header)?;
        let header = Header::from(&header);
        println!("Header: {:?}", &header);

        let mut data = vec![0; header.get_prg_rom_size()];
        fh.read_exact(&mut data)?;

        let mapper = create_mapper(&header, &data);
        Ok(RefCell::new(CartridgeImpl {
            name: filename.to_owned(),
            mapper,
        }))
    }

    fn prg_read(&self, addr: usize) -> u8 {
        self.borrow().mapper.prg_read(addr)
    }

    fn prg_write(&self, addr: usize, val: u8) {
        self.borrow_mut().mapper.prg_write(addr, val);
    }

    fn chr_read(&self, addr: usize) -> u8 {
        self.borrow().mapper.chr_read(addr)
    }

    fn chr_write(&self, addr: usize, val: u8) {
        self.borrow_mut().mapper.chr_write(addr, val);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn load_none() {
        let rom = Cartridge::load("NoFile.nes");
        assert!(rom.is_err());
        assert!(match rom.err() {
            Some(e) => e.kind() == ErrorKind::NotFound,
            None => false,
        });
    }

    #[test]
    fn load_some() {
        let exp_name = "nes-test-roms/cpu_dummy_reads/cpu_dummy_reads.nes";
        let cart = match Cartridge::load(exp_name) {
            Ok(cart) => cart,
            Err(e) => unreachable!("Error {:?}", e),
        };
        assert!(cart.borrow().mapper.prg_size() > 0);
        assert!(cart.borrow().mapper.chr_size() > 0);
        assert_eq!(cart.get_name(), exp_name);
    }
}
