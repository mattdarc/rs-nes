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
use std::cell::RefCell;
use std::sync::{atomic::AtomicBool, Arc};

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
    StopRequested(i32),
    ExitInterrupt, // TODO: Temporary. Used to exit nestest
    ExitError(String),
}

pub type CpuTask<'a> = Box<dyn FnMut(&mut dyn CpuInterface) + 'a>;
type TaskList<'a> = RefCell<Vec<CpuTask<'a>>>;

pub struct VNES<'a> {
    cpu: cpu::CPU<bus::NesBus>,
    pre_execute_tasks: TaskList<'a>,
    post_execute_tasks: TaskList<'a>,
    headless: bool,
}

type NesResult = Result<(), String>;

unsafe impl<'a> Send for VNES<'a> {}

impl<'a> VNES<'a> {
    pub fn new(rom: &str) -> std::io::Result<Self> {
        let game = load_cartridge(rom)?;
        let bus = NesBus::new(game, Box::new(graphics::sdl2::SDLRenderer::new()));
        Ok(VNES {
            cpu: CPU::new(bus),
            pre_execute_tasks: TaskList::new(Vec::new()),
            post_execute_tasks: TaskList::new(Vec::new()),
            headless: false,
        })
    }

    pub fn new_headless(rom: &str) -> std::io::Result<Self> {
        let game = load_cartridge(rom)?;
        let bus = NesBus::new(game, Box::new(graphics::nop::NOPRenderer::new()));
        Ok(VNES {
            cpu: CPU::new(bus),
            pre_execute_tasks: TaskList::new(Vec::new()),
            post_execute_tasks: TaskList::new(Vec::new()),
            headless: true,
        })
    }

    pub fn add_pre_execute_task(&mut self, task: CpuTask<'a>) {
        self.pre_execute_tasks.borrow_mut().push(task);
    }

    pub fn add_post_execute_task(&mut self, task: CpuTask<'a>) {
        self.post_execute_tasks.borrow_mut().push(task);
    }

    fn run_pre_execute_tasks(&mut self) {
        for task in self.post_execute_tasks.borrow_mut().iter_mut() {
            task(&mut self.cpu);
        }
    }

    fn run_post_execute_tasks(&mut self) {
        for task in self.post_execute_tasks.borrow_mut().iter_mut() {
            task(&mut self.cpu);
        }
    }

    pub fn nestest_reset_override(&mut self, pc: u16) {
        self.cpu.nestest_reset_override(pc);
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
    }

    pub fn run_once(&mut self) -> ExitStatus {
        self.run_pre_execute_tasks();
        let status = self.cpu.clock();
        self.run_post_execute_tasks();

        status
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

    fn sdl_loop(stop_token: Arc<AtomicBool>) {
        use graphics::sdl2::SDL2Intrf;
        use sdl2::{event::Event, keyboard::Keycode, keyboard::Mod};

        let mut event_pump = SDL2Intrf::context().event_pump().unwrap();

        while !stop_token.load(std::sync::atomic::Ordering::Acquire) {
            let timeout_ms = 200;
            let event = event_pump.wait_event_timeout(timeout_ms);
            if event.is_none() {
                continue;
            }

            match event.unwrap() {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                }
                | Event::KeyDown {
                    keycode: Some(Keycode::C),
                    keymod: Mod::LCTRLMOD,
                    ..
                } => {
                    stop_token.store(true, std::sync::atomic::Ordering::Release);
                    return;
                }
                ev => println!("Unhandled event {:?}", ev),
            }
        }
    }

    fn cpu_loop(&mut self, stop_token: Arc<AtomicBool>) -> Result<(), String> {
        let mut inner_loop = || {
            while !stop_token.load(std::sync::atomic::Ordering::Acquire) {
                match self.run_once() {
                    ExitStatus::Continue => {}
                    ExitStatus::ExitError(e) => return Err(e),

                    ExitStatus::StopRequested(code) => {
                        if code == 0 {
                            return Ok(());
                        }

                        return Err(format!("StopRequested: {}", code));
                    }

                    // FIXME: Need to figure out the proper way to handle breakpoints
                    ExitStatus::Breakpoint(_)
                    | ExitStatus::ExitSuccess
                    | ExitStatus::ExitInterrupt => {
                        return Ok(());
                    }
                }
            }

            Ok(())
        };

        let ret = inner_loop();
        stop_token.store(true, std::sync::atomic::Ordering::Release);
        ret
    }

    pub fn play(&mut self) -> Result<(), String> {
        let stop_token_cpu = Arc::new(AtomicBool::new(false));
        if self.headless {
            return self.cpu_loop(stop_token_cpu);
        }

        scope(|scope| {
            use std::panic;
            let stop_token_sdl = stop_token_cpu.clone();

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
                .spawn(|_| self.cpu_loop(stop_token_cpu))
                .unwrap();

            VNES::sdl_loop(stop_token_sdl);
            cpu_thread.join().unwrap()
        })
        .unwrap()
    }
}

pub mod input {}
