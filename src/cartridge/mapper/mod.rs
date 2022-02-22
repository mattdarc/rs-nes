// The mapper controls read/write to and from memory. A catridge should have a mapper and memory,
// then the memory should only be accessed using the mapper. The mapper defines where the RAM
// ROM PPU APU all are in memory AFAIK, and defines the mirroring

mod mapper0;
mod mapper1;
mod mapper3;

use super::header::Header;
use mapper0::Mapper0;
use mapper1::Mapper1;
use mapper3::Mapper3;

use std::fmt;

pub const RESET_VECTOR_START: u16 = 0xC004;

fn dump_game(header: &Header, game: &[u8]) {
    let (prg, chr) = game.split_at(header.get_prg_rom_size() as usize);
    println!("PRG:");
    for (addr, instr) in prg.iter().enumerate() {
        println!(" 0x{:?}: {:?}", addr, instr);
    }

    println!("\nCHR:");
    for (addr, data) in chr.iter().enumerate() {
        println!(" 0x{:?}: {:?}", addr, data);
    }
}

pub trait Mapper {
    fn get_num(&self) -> u8;
    fn rom_start(&self) -> u16 {
        0x8000
    }

    fn box_clone(&self) -> Box<dyn Mapper>;

    // PRG
    fn prg_read(&self, addr: u16) -> u8;
    fn prg_write(&mut self, addr: u16, val: u8);
    fn prg_size(&self) -> u16;

    // CHR
    fn chr_read(&self, addr: u16) -> u8;
    fn chr_write(&mut self, addr: u16, val: u8);
    fn chr_size(&self) -> u16;
}

impl Clone for Box<dyn Mapper> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

impl fmt::Debug for Box<dyn Mapper> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mapper")
            .field("mapper_num", &self.get_num())
            .field("prg_size", &self.prg_size())
            .field("chr_size", &self.chr_size())
            .finish()
    }
}

impl Default for Box<dyn Mapper> {
    fn default() -> Box<dyn Mapper> {
        Box::new(Mapper0::empty())
    }
}

pub fn create_mapper(header: &Header, data: &[u8]) -> Box<dyn Mapper> {
    match header.get_mapper_num() {
        0 => Box::new(Mapper0::new(header, data)),
        1 => Box::new(Mapper1::new(header, data)),
        3 => Box::new(Mapper3::new(header, data)),
        n => unreachable!("Unimplemented mapper {}!", n),
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    // Create a test mapper, setting the reset vector to the first instruction
    pub fn mapper_with(data: &[u8], reset_vector: u16) -> Box<dyn Mapper> {
        let header = Header::default();
        let mut rom = vec![0; header.get_prg_rom_size() as usize];

        println!("-- Cloning data {:?} into ROM", data);
        rom[0..data.len()].clone_from_slice(data);
        let low = (reset_vector & 0xFF) as u8;
        let high = (reset_vector >> 8) as u8;
        let reset_loc = (RESET_VECTOR_START - reset_vector) as usize;

        println!(
            "-- Initializing ROM reset vector at {:#X} to {:#X}",
            reset_loc, reset_vector
        );
        rom[reset_loc] = low;
        rom[reset_loc + 1] = high;
        super::create_mapper(&header, rom.as_slice())
    }
}
