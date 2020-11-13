use crate::bus::*;
use crate::cartridge::*;
use crate::common::*;
use crate::cpu::*;
use crate::ppu::*;

struct Clock {}

impl Clock {
    pub fn tick(&mut self) -> (bool, bool) {
        (true, true)
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
        self.cpu.connect(&mut self.bus).init();
        Ok(0)
    }

    pub fn play(&mut self) {
        while !self.cpu.done() {
            let (cpu_tick, ppu_tick) = self.clock.tick();

            if cpu_tick {
                println!("-- Clocking CPU!");
                self.cpu.clock(&mut self.bus.connect(&mut self.ppu));
            }

            if ppu_tick {
                println!("-- Clocking PPU!");
                self.ppu.clock(&mut self.bus);
            }
        }
    }
}
