#[macro_use]
extern crate static_assertions;

#[macro_use]
mod common;

extern crate sdl2; 


mod apu;
mod cartridge;
mod controller;
mod cpu;
mod instructions;
mod mapper;
mod memory;
mod ppu;
mod vnes;

use vnes::*;

fn main() {
    const_assert!(0 == 0);
    let mut vnes = VNES::new();
    while let Err(e) = vnes.play("../roms/Tetris.nes") {
	println!("Error: {}", e)
    }
}
