#![allow(dead_code)]

extern crate sdl2;

#[macro_use]
extern crate bitflags;

pub mod apu;
pub mod cartridge;
pub mod cpu;
pub mod graphics;
pub mod ppu;

mod bus;
mod common;
mod controller;
mod debug;
mod memory;

use cartridge::*;
use cpu::*;
use tracing::{event, Level};

pub type NesBus = bus::NesBus;
pub type NesCPU = CPU<NesBus>;

#[derive(Debug)]
pub enum NesError {
    Stub,
}

#[derive(Debug, Clone)]
pub enum ExitStatus {
    Continue,
    ExitSuccess,
    ExitInterrupt, // TODO: Temporary. Used to exit nestest
    ExitError(String),
}

pub struct VNES {
    cpu: cpu::CPU<bus::NesBus>,
}

impl VNES {
    pub fn new(rom: &str) -> std::io::Result<Self> {
        let game = Cartridge::load(rom)?;
        let bus = NesBus::new(game, Box::new(graphics::sdl2::SDLRenderer::new()));

        Ok(VNES {
            cpu: CPU::new(bus, RESET_VECTOR_START),
        })
    }

    pub fn init(&mut self) {
        self.cpu.init();
    }

    pub fn play(&mut self) -> Result<(), NesError> {
        loop {
            match self.cpu.clock() {
                ExitStatus::Continue => {}
                ExitStatus::ExitSuccess => return Ok(()),
                ExitStatus::ExitError(e) => {
                    event!(Level::ERROR, %e, "Exiting");
                    return Ok(());
                }
                ExitStatus::ExitInterrupt => {
                    event!(Level::INFO, "Exiting from software interrupt");
                    return Ok(());
                }
            }
        }
    }
}

pub mod audio {}

pub mod input {}
