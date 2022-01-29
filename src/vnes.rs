use crate::bus::*;
use crate::cartridge::*;
use crate::common::*;
use crate::cpu::*;
use crate::ppu::*;
use crate::sdl_interface::{Event, Keycode, SDL2Intrf};
use std::time::Duration;

const MASTER_CLOCK: u32 = 1_000_000_000 / 21_477_272 / 240;

struct Clock {}

impl Clock {
    pub fn tick(&mut self) -> (bool, bool) {
        (true, true)
    }

    pub fn sleep(&self) {
        // NES master clock speed is 21.477272 MHz. Changing this clock should be how we introduce a "fast-forward" mode
        ::std::thread::sleep(Duration::new(0, MASTER_CLOCK));
    }
}

impl Clock {
    fn new() -> Clock {
        Clock {}
    }
}

pub struct VNES {
    cpu: Ricoh2A03,
    ppu: PPU,
    bus: NesBus,
    clock: Clock,
}

impl VNES {
    pub fn new() -> VNES {
        VNES {
            cpu: Ricoh2A03::new(),
            ppu: PPU::new(),
            bus: NesBus::new(),
            clock: Clock::new(),
        }
    }

    pub fn load(&mut self, filename: &str) -> Result<u8, std::io::Error> {
        let cartridge = Cartridge::load(filename)?;
        self.bus.init(cartridge).expect("Error loading cartridge");
        self.ppu.init().expect("Error initializing PPU");
        self.cpu.connect(&mut self.bus).init();
        Ok(0)
    }

    pub fn play(&mut self) {
        'running: loop {
            // Handle clock to see what should be executing
            let (cpu_tick, ppu_tick) = self.clock.tick();

            // Tick CPU if running
            if cpu_tick {
                // println!("-- Clocking CPU!");
                self.cpu.clock(&mut self.bus.connect(&mut self.ppu));
            }

            // Tick PPU if running
            if ppu_tick {
                // println!("-- Clocking PPU!");
                self.ppu.clock(&mut self.bus);
            }

            // Handle any events that happened
            for event in SDL2Intrf::context()
                .event_pump()
                .expect("There is only one instance")
                .poll_iter()
            {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => break 'running,
                    _ => {}
                }
            }
            self.clock.sleep();
        }
    }
}
