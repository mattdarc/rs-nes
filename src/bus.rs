use crate::apu::*;
use crate::cartridge::*;
use crate::common::*;
use crate::controller::*;
use crate::memory::*;
use crate::ppu::*;

pub struct NesBus {
    ram: RAM,
    apu: APU,
    controller_1: Controller,
    controller_2: Controller,
    cartridge: Option<Cartridge>,
}

pub struct PPUAccessibleBus<'a> {
    bus: &'a mut NesBus,
    ppu: &'a mut PPU,
}

impl Bus for NesBus {
    fn read(&mut self, addr: usize) -> u8 {
        let val = match addr {
            0..=0x1FFF => self.ram.read(addr % 0x800), // Mirroring
            0x2000..=0x3FFF => panic!("No access to PPU!"),
            0x4016 => self.controller_1.read(0), // TODO Remove arg
            0x4017 => self.controller_2.read(0),
            0x4018..=0xFFFF => self.cartridge.as_ref().unwrap().prg_read(addr),
            _ => unreachable!("Invalid read from address {:#X}!", addr),
        };
        //println("-- Read value {:#X} from {:#X}", val, addr);
        val
    }

    fn write(&mut self, addr: usize, val: u8) {
        //println("$$ Writing {:#X} to RAM {:#X}", val, addr);
        match addr {
            0..=0x1FFF => self.ram.write(addr % 0x800, val),
            0x2000..=0x3FFF => panic!("No access to PPU!"),
            0x4016 => self.controller_1.write(0, val),
            0x4017 => self.controller_2.write(0, val),
            0x4018..=0xFFFF => self.cartridge.as_ref().unwrap().prg_write(addr, val),
            _ => unreachable!("Invalid write {} to address {}!", val, addr),
        }
    }
}

impl<'a> Bus for PPUAccessibleBus<'a> {
    fn read(&mut self, addr: usize) -> u8 {
        match addr {
            0x2000..=0x3FFF => self.ppu.read(addr),
            addr => self.bus.read(addr),
        }
    }

    fn write(&mut self, addr: usize, val: u8) {
        match addr {
            0x2000..=0x3FFF => {
                if self.ppu.is_ppu_data(addr) {
                    self.ppu.write(addr, val);
                } else {
                    self.write(self.ppu.vram_addr(), val);
                }
            }
            addr => self.bus.write(addr, val),
        }
    }
}

impl NesBus {
    pub fn new() -> NesBus {
        NesBus {
            ram: RAM::new(0x800),
            apu: APU::default(),
            controller_1: Controller::default(),
            controller_2: Controller::default(),
            cartridge: None,
        }
    }

    pub fn init(
        &mut self, cartridge: Cartridge,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.cartridge = Some(cartridge);
        Ok(())
    }

    pub fn connect<'a>(&'a mut self, ppu: &'a mut PPU) -> PPUAccessibleBus<'a> {
        PPUAccessibleBus { bus: self, ppu }
    }
}
