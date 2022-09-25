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
use crossbeam::thread::scope;

pub type NesBus = bus::NesBus;
pub type NesCPU = CPU<NesBus>;

#[derive(Debug)]
pub enum NesError {
    Stub,
}

#[derive(Debug, Clone, Eq, PartialEq)]
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

unsafe impl Send for VNES {}

impl VNES {
    pub fn new(rom: &str) -> std::io::Result<Self> {
        let game = Cartridge::load(rom)?;
        let bus = NesBus::new(game, Box::new(graphics::sdl2::SDLRenderer::new()));
        Ok(VNES { cpu: CPU::new(bus) })
    }

    pub fn state(&self) -> &cpu::CpuState {
        self.cpu.state()
    }

    pub fn reset_override(&mut self, pc: u16) {
        self.cpu.reset_override(pc);
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
    }

    pub fn run_once(&mut self) -> ExitStatus {
        self.cpu.clock()
    }

    pub fn run_until(&mut self, pc: u16) -> ExitStatus {
        use std::time::Duration;

        // FIXME: Set a SW breakpoint in the CPU instead of doing this
        while self.cpu.pc() < pc {
            match self.run_once() {
                ExitStatus::Continue => {}
                status => return status,
            }
            // FIXME: The CPU probably should not be throttled like this
            ::std::thread::sleep(Duration::new(0, graphics::constants::FRAME_RATE_NS));
        }

        ExitStatus::Breakpoint(self.cpu.pc())
    }

    fn wait_for_interrupt(mut event_pump: sdl2::EventPump) {
        use sdl2::{event::Event, keyboard::Keycode};

        loop {
            let event = event_pump.wait_event();

            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return,
                _ => {}
            }
        }
    }

    pub fn play(&mut self) -> Result<(), String> {
        use graphics::sdl2::SDL2Intrf;
        use std::sync::atomic::AtomicBool;

        let stop_requested = AtomicBool::new(false);
        let event_pump = SDL2Intrf::context().event_pump().unwrap();
        scope(|scope| {
            let cpu_thread = scope.spawn(|_| {
                while !stop_requested.load(std::sync::atomic::Ordering::Relaxed) {
                    match self.run_once() {
                        ExitStatus::Continue => {}
                        ExitStatus::ExitError(e) => return Err(e),
                        ExitStatus::Breakpoint(_)
                        | ExitStatus::ExitSuccess
                        | ExitStatus::ExitInterrupt => return Ok(()),
                    }
                }

                Ok(())
            });

            VNES::wait_for_interrupt(event_pump);
            stop_requested.store(true, std::sync::atomic::Ordering::Relaxed);
            cpu_thread.join().unwrap()
        })
        .unwrap()
    }
}

pub mod audio {}

pub mod input {}
