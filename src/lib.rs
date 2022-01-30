extern crate sdl2;

#[macro_use]
extern crate bitflags;

mod apu;
mod bus;
pub mod cartridge;
mod common;
mod controller;
mod cpu;
mod debug;
mod memory;
pub mod ppu;
pub mod sdl_interface;

use bus::*;
use cartridge::*;
use cpu::*;

pub mod graphics {
    pub use super::sdl_interface::graphics::Renderer;
    pub use super::sdl_interface::SDL2Intrf;
    pub const FRAME_RATE_NS: u32 =
        1_000_000_000 / 60 / super::sdl_interface::graphics::NES_SCREEN_HEIGHT;
}

#[derive(Debug)]
pub enum NesError {
    Stub,
}

pub struct VNES {
    cpu: cpu::CPU<bus::NesBus>,
}

impl VNES {
    pub fn new(rom: &str) -> std::io::Result<Self> {
        let game = Cartridge::load(rom)?;
        let bus = NesBus::with_cartridge(game);
        Ok(VNES { cpu: CPU::new(bus) })
    }

    pub fn enable_logging(&mut self, log: bool) {
        self.cpu.enable_logging(log);
    }

    // TODO: Error handling. library types should not panic
    pub fn play(&mut self) -> Result<(), NesError> {
        self.cpu.bypass_reset(0xC000);
        loop {
            if self.cpu.clock() {
                break;
            }
        }

        Ok(())
    }
}

pub mod audio {}

pub mod input {}
