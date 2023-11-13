#![allow(dead_code)]
#![feature(exclusive_range_pattern)]

extern crate sdl2;

#[macro_use]
extern crate bitflags;

pub mod apu;
pub mod audio;
pub mod cartridge;
pub mod cpu;
pub mod graphics;
pub mod ppu;

mod bus;
mod controller;
mod memory;

use cartridge::*;
use cpu::*;
use crossbeam::thread::scope;
use std::sync::{atomic::AtomicBool, Arc};

pub type NesBus = bus::NesBus;
pub type NesCPU<'a> = CPU<'a, NesBus>;

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

pub struct VNES<'a> {
    cpu: cpu::CPU<'a, bus::NesBus>,
}

type NesResult = Result<(), String>;

unsafe impl<'a> Send for VNES<'a> {}

impl<'a> VNES<'a> {
    pub fn new(rom: &str) -> std::io::Result<Self> {
        let game = load_cartridge(rom)?;
        let bus = NesBus::new(game, Box::new(graphics::sdl2::SDLRenderer::new()));
        Ok(VNES { cpu: CPU::new(bus) })
    }

    pub fn new_headless(rom: &str) -> std::io::Result<Self> {
        let game = load_cartridge(rom)?;
        let bus = NesBus::new(game, Box::new(graphics::nop::NOPRenderer::new()));
        Ok(VNES { cpu: CPU::new(bus) })
    }

    pub fn add_pre_execute_task(&mut self, task: CpuTask<'a>) {
        self.cpu.add_pre_execute_task(task);
    }

    pub fn add_post_execute_task(&mut self, task: CpuTask<'a>) {
        self.cpu.add_post_execute_task(task);
    }

    pub fn nestest_reset_override(&mut self, pc: u16) {
        self.cpu.nestest_reset_override(pc);
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
    }

    pub fn run_once(&mut self) -> ExitStatus {
        self.cpu.clock()
    }

    pub fn run_until(&mut self, pc: u16) -> ExitStatus {
        // FIXME: Set a SW breakpoint in the CPU instead of doing this
        while self.cpu.pc() < pc {
            match self.run_once() {
                ExitStatus::Continue => {}
                status => return status,
            }
        }

        ExitStatus::Breakpoint(self.cpu.pc())
    }

    fn wait_for_interrupt(mut event_pump: sdl2::EventPump, stop_token: Arc<AtomicBool>) {
        use sdl2::{event::Event, keyboard::Keycode};

        while !stop_token.load(std::sync::atomic::Ordering::Acquire) {
            let event = event_pump.wait_event();

            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    stop_token.store(true, std::sync::atomic::Ordering::Release);
                    return;
                }
                _ => {} // FIXME: Perhaps this should be recorded somewhere
            }
        }
    }

    pub fn play(&mut self) -> Result<(), String> {
        use graphics::sdl2::SDL2Intrf;
        use std::panic;

        let stop_token_cpu = Arc::new(AtomicBool::new(false));
        let event_pump = SDL2Intrf::context().event_pump().unwrap();

        scope(|scope| {
            let stop_token_main = stop_token_cpu.clone();

            // take_hook() returns the default hook in case when a custom one is not set
            let orig_hook = panic::take_hook();
            panic::set_hook(Box::new(move |panic_info| {
                // invoke the default handler and exit the process
                orig_hook(panic_info);
                std::process::exit(1);
            }));

            let cpu_thread = scope
                .builder()
                .name("cpu-thread".to_owned())
                .spawn(|_| {
                    while !stop_token_cpu.load(std::sync::atomic::Ordering::Acquire) {
                        match self.run_once() {
                            ExitStatus::Continue => {}
                            ExitStatus::ExitError(e) => return Err(e),

                            // FIXME: Need to figure out the proper way to handle breakpoints
                            ExitStatus::Breakpoint(_)
                            | ExitStatus::ExitSuccess
                            | ExitStatus::ExitInterrupt => {
                                stop_token_cpu.store(true, std::sync::atomic::Ordering::Release);
                                return Ok(());
                            }
                        }
                    }

                    Ok(())
                })
                .unwrap();

            VNES::wait_for_interrupt(event_pump, stop_token_main);
            cpu_thread.join().unwrap()
        })
        .unwrap()
    }
}

pub mod input {}
