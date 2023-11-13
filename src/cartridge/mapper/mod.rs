// The mapper controls read/write to and from memory. A catridge should have a mapper and memory,
// then the memory should only be accessed using the mapper. The mapper defines where the RAM
// ROM PPU APU all are in memory AFAIK, and defines the mirroring

mod mapper0;
mod mapper1;

use super::header::Header;
use crate::memory::ROM;
use mapper0::Mapper0;
use mapper1::Mapper1;
use tracing;
use tracing::Level;

use std::fmt;

fn dump_game(header: &Header, game: &[u8]) {
    println!("Header:\n {:?}", header);
    let (prg, chr) = game.split_at(header.get_prg_rom_size() as usize);

    let print_data = |name, data: &[u8]| {
        tracing::debug!("{}:", name);
        for (addr, chunk) in data.chunks(16).enumerate() {
            tracing::debug!(
                " 0x{:<4x}| {}",
                addr * 16,
                chunk
                    .iter()
                    .map(|d| format!("{:0<2x}", d))
                    .fold(String::new(), |acc, b| acc + " " + &b)
            );
        }
        println!();
    };

    print_data("PRG", prg);
    print_data("CHR", chr);
}

#[track_caller]
fn unknown_address(addr: usize) -> ! {
    panic!("Invalid access of unknown address {:#X}", addr);
}

pub trait Mapper {
    fn number(&self) -> u8;
    fn prg_read(&self, addr: u16) -> u8;
    fn prg_write(&mut self, addr: u16, val: u8);
    fn chr(&self) -> ROM;
    fn dpcm(&self) -> ROM;
}

impl fmt::Debug for Box<dyn Mapper> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("Mapper{}", self.number())).finish()
    }
}

impl Default for Box<dyn Mapper> {
    fn default() -> Box<dyn Mapper> {
        Box::new(Mapper0::empty())
    }
}

pub fn create_mapper(header: &Header, data: &[u8]) -> Box<dyn Mapper> {
    if tracing::enabled!(Level::DEBUG) {
        dump_game(header, data);
    }

    match header.get_mapper_num() {
        0 => Box::new(Mapper0::new(header, data)),
        1 => Box::new(Mapper1::new(header, data)),
        n => panic!("Unimplemented mapper {}!", n),
    }
}
