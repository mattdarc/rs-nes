use super::*;
use crate::memory::*;

#[derive(Clone)]
pub struct Mapper0 {
    // for CPU
    prg_rom: ROM,
    prg_ram: RAM,

    // for PPU, "most emulators support ram"
    chr_ram: RAM,
}

impl Mapper0 {
    pub const ROM_START: u16 = 0x8000;

    pub fn empty() -> Self {
        Mapper0 {
            prg_rom: ROM::with_size(0),
            prg_ram: RAM::with_size(0),
            chr_ram: RAM::with_size(0),
        }
    }

    pub fn new(header: &Header, data: &[u8]) -> Self {
        Mapper0 {
            prg_rom: ROM::with_data_and_size(data, header.get_prg_rom_size()),
            prg_ram: RAM::with_size(header.get_prg_ram_size()),
            chr_ram: RAM::with_size(header.get_chr_ram_size()),
        }
    }
}

impl Mapper for Mapper0 {
    fn get_num(&self) -> u8 {
        0
    }

    fn box_clone(&self) -> Box<dyn Mapper> {
        Box::new(self.clone())
    }

    // PRG
    fn prg_read(&self, addr: usize) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram.read(addr - 0x6000),
            0x8000..=0xFFFF => self.prg_rom.read((addr - 0x8000) % 0x4000),
            _ => unreachable!("Invalid read of address {}!", addr),
        }
    }

    fn prg_write(&mut self, addr: usize, val: u8) {
        match addr {
            0x6000..=0x7FFF => self.prg_ram.write(addr - 0x6000, val),
            0x8000..=0xFFFF => unreachable!("Tried to overwrite ROM!"),
            _ => unreachable!("Invalid read of address {}!", addr),
        };
    }

    fn prg_size(&self) -> usize {
        self.prg_rom.len() + self.prg_ram.len()
    }

    // CHR
    fn chr_read(&self, addr: usize) -> u8 {
        self.chr_ram.read(addr)
    }

    fn chr_write(&mut self, addr: usize, val: u8) {
        self.chr_ram.write(addr, val)
    }

    fn chr_size(&self) -> usize {
        self.chr_ram.len()
    }
}
