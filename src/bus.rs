use crate::apu::*;
use crate::cartridge::*;
use crate::common::*;
use crate::controller::*;
use crate::instructions::*;
use crate::memory::*;
use crate::ppu::*;

#[derive(Clone)]
pub struct Bus<'a> {
    ram: RAM,
    ppu: PPU<'a>,
    apu: APU,
    controller_1: Controller,
    controller_2: Controller,
    cartridge: Option<&'a Cartridge>,
}

impl<'a> Bus<'a> {
    pub fn new() -> Bus<'a> {
        Bus {
            ram: RAM::new(0x800),
            ppu: PPU::new(),
            apu: APU::default(),
            controller_1: Controller::default(),
            controller_2: Controller::default(),
            cartridge: None,
        }
    }

    pub fn read(&mut self, addr: usize) -> u8 {
        let val = match addr {
            0..=0x1FFF => self.ram.read(addr % 0x800), // Mirroring
            0x2000..=0x3FFF => self.ppu.read(addr),
            0x4016 => self.controller_1.read(0), // TODO Remove arg
            0x4017 => self.controller_2.read(0),
            0x4018..=0xFFFF => self.cartridge.unwrap().prg_read(addr),
            _ => unreachable!("Invalid read from address {:#X}!", addr),
        };
        //println("-- Read value {:#X} from {:#X}", val, addr);
        val
    }

    pub fn read_n(&mut self, addr: usize, n: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(n);
        for idx in 0..n {
            v.push(self.read(addr + idx));
        }
        v
    }

    pub fn read16(&mut self, addr: usize) -> u16 {
        (self.read(addr) as u16) | ((self.read(addr + 1) as u16) << 8)
    }

    pub fn write(&mut self, addr: usize, val: u8) {
        //println("$$ Writing {:#X} to RAM {:#X}", val, addr);
        match addr {
            0..=0x1FFF => self.ram.write(addr % 0x800, val),
            0x2000..=0x3FFF => self.ppu.write(addr, val),
            0x4016 => self.controller_1.write(0, val),
            0x4017 => self.controller_2.write(0, val),
            0x4018..=0xFFFF => self.cartridge.unwrap().prg_write(addr, val),
            _ => unreachable!("Invalid write {} to address {}!", val, addr),
        }
    }

    pub fn init(
        &mut self, cartridge: &'a Cartridge,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.cartridge = Some(cartridge);
        // self.ppu.init(cartridge)
        Ok(())
    }
}

impl<'a> Clocked for Bus<'a> {
    fn clock(&mut self) {
        // self.ppu.clock();
    }
}
