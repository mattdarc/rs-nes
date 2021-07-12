use super::*;
use crate::memory::*;

#[derive(Clone)]
pub struct Mapper3 {
    prg_rom: ROM, // for CPU
    chr_ram: RAM, // for PPU, "most emulators support ram"
}

impl Mapper3 {
    pub const ROM_START: u16 = 0x8000;

    pub fn new(header: &Header, data: &[u8]) -> Self {
        Mapper3 {
            chr_ram: RAM::with_size(header.get_chr_ram_size()),
            prg_rom: ROM::with_data_and_size(data, header.get_prg_rom_size()),
        }
    }
}

impl Mapper for Mapper3 {
    fn get_num(&self) -> u8 {
        3
    }

    fn box_clone(&self) -> Box<dyn Mapper> {
        Box::new(self.clone())
    }

    // PRG
    fn prg_read(&self, addr: usize) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.prg_rom.read(addr - 0x8000),
            _ => unreachable!("Invalid read of address {}!", addr),
        }
    }

    fn prg_write(&mut self, addr: usize, _val: u8) {
        match addr {
            0x6000..=0x7FFF => unreachable!("Tried to overwrite ROM!"),
            0x8000..=0xFFFF => unreachable!("Tried to overwrite ROM!"),
            _ => unreachable!("Invalid read of address {}!", addr),
        };
    }

    fn prg_size(&self) -> usize {
        self.prg_rom.len()
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
