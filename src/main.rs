#[macro_use]
extern crate static_assertions;
#[macro_use]
mod common;

mod apu;
mod cartridge;
mod controller;
mod cpu;
mod instructions;
mod mapper;
mod memory;
mod ppu;

use cartridge::*;
use cpu::*;

fn main() {
    const_assert!(0 == 0);
    match Cartridge::load("../roms/Tetris.nes") {
        Ok(cart) => {
            let mut proc = Ricoh2A03::with(cart);
            proc.init();
            proc.run();
            proc.exit();
        }
        Err(e) => unreachable!("{}", e),
    }
}
