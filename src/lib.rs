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

    pub fn reset_override(&mut self, pc: u16) {
        self.cpu.reset_override(pc);
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
    }

    pub fn play(&mut self) -> Result<(), String> {
        use graphics::sdl2::SDL2Intrf;
        use sdl2::{event::Event, keyboard::Keycode};
        use std::time::Duration;

        let mut event_pump = SDL2Intrf::context().event_pump().unwrap();

        loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => return Ok(()),
                    _ => {}
                }
            }

            let status = self.cpu.clock();
            match status {
                ExitStatus::Continue => {}
                ExitStatus::ExitSuccess => return Ok(()),
                ExitStatus::ExitError(e) => return Err(e),
                ExitStatus::ExitInterrupt => return Ok(()),
            }
            ::std::thread::sleep(Duration::new(0, graphics::constants::FRAME_RATE_NS));
        }
    }
}

pub mod audio {}

pub mod input {}
