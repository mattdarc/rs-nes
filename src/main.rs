#[macro_use] extern crate static_assertions;
#[macro_use] mod common;

mod cpu;
mod instructions;
mod memory;
mod mapper;
mod cartridge;
mod controller;
mod ppu;
mod apu;

use cpu::*;

fn main() {
    const_assert!(0 == 0);
    let mut proc = Ricoh2A03::new();
    proc.run();
}
