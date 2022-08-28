#![allow(dead_code)]
#![feature(exclusive_range_pattern)]

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

pub type NesBus = bus::NesBus;
pub type NesCPU = CPU<NesBus>;

#[derive(Debug)]
pub enum NesError {
    Stub,
}

#[derive(Debug, Clone)]
pub enum ExitStatus {
    Continue,
    Breakpoint(u16),
    ExitSuccess,
    ExitInterrupt, // TODO: Temporary. Used to exit nestest
    ExitError(String),
}

pub struct VNES {
    cpu: cpu::CPU<bus::NesBus>,
}

type NesResult = Result<(), String>;

impl VNES {
    pub fn new(rom: &str) -> std::io::Result<Self> {
        let game = Cartridge::load(rom)?;
        let bus = NesBus::new(game, Box::new(graphics::sdl2::SDLRenderer::new()));
        Ok(VNES { cpu: CPU::new(bus) })
    }

    pub fn reset_override(&mut self, pc: u16) {
        self.cpu.reset_override(pc);
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
    }

    pub fn run_once(&mut self) -> ExitStatus {
        use graphics::sdl2::SDL2Intrf;
        use sdl2::{event::Event, keyboard::Keycode};
        use std::time::Duration;
        let mut event_pump = SDL2Intrf::context().event_pump().unwrap();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return ExitStatus::ExitInterrupt,
                _ => {}
            }
        }
        ::std::thread::sleep(Duration::new(0, graphics::constants::FRAME_RATE_NS));

        self.cpu.clock()
    }

    // FIXME: Set a SW breakpoint in the CPU instead of doing this
    pub fn run_until(&mut self, pc: u16) -> ExitStatus {
        while self.cpu.pc() < pc {
            match self.run_once() {
                ExitStatus::Continue => {}
                status => return status,
            }
        }

        ExitStatus::Breakpoint(self.cpu.pc())
    }

    pub fn play(&mut self) -> Result<(), String> {
        loop {
            match self.run_once() {
                ExitStatus::Continue => {}
                ExitStatus::ExitError(e) => return Err(e),
                ExitStatus::Breakpoint(_) | ExitStatus::ExitSuccess | ExitStatus::ExitInterrupt => {
                    return Ok(())
                }
            }
        }
    }
}

pub mod audio {}

pub mod input {}
