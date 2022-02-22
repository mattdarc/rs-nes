use super::*;
use crate::memory::*;

#[derive(Clone)]
pub struct Mapper1 {
    prg_rom: ROM, // for CPU
    prg_ram: RAM, // for CPU
    chr_ram: RAM, // for PPU, "most emulators support ram"
}

impl Mapper1 {
    pub fn new(header: &Header, data: &[u8]) -> Self {
        //dump_game(header, data);

        let (prg, chr) = data.split_at(header.get_prg_rom_size() as usize);
        Mapper1 {
            prg_ram: RAM::with_size(header.get_prg_ram_size()),
            prg_rom: ROM::with_data_and_size(prg, header.get_prg_rom_size()),
            chr_ram: RAM::with_data_and_size(chr, header.get_chr_ram_size()),
        }
    }
}

impl Mapper for Mapper1 {
    fn get_num(&self) -> u8 {
        1
    }

    fn box_clone(&self) -> Box<dyn Mapper> {
        Box::new(self.clone())
    }

    // PRG
    fn prg_read(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram.read(addr - 0x6000),
            0x8000..=0xFFFF => self.prg_rom.read(addr - 0x8000),
            _ => unreachable!("Invalid read of address {:#X}!", addr),
        }
    }

    fn prg_write(&mut self, addr: u16, val: u8) {
        match addr {
            0x6000..=0x7FFF => self.prg_ram.write(addr - 0x6000, val),
            0x8000..=0xFFFF => unreachable!("Tried to write to ROM at address {}!", addr),
            _ => unreachable!("Invalid read of address {}!", addr),
        };
    }

    fn prg_size(&self) -> u16 {
        self.prg_rom.len() + self.prg_ram.len()
    }

    // CHR
    fn chr_read(&self, addr: u16) -> u8 {
        self.chr_ram.read(addr)
    }

    fn chr_write(&mut self, addr: u16, val: u8) {
        self.chr_ram.write(addr, val)
    }

    fn chr_size(&self) -> u16 {
        self.chr_ram.len()
    }
}
