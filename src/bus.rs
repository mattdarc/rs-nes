use crate::apu::*;
use crate::cartridge::*;
use crate::controller::*;
use crate::debug::debug_print;
use crate::memory::*;
use crate::ppu::*;

pub trait Bus {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
    fn read16(&self, addr: u16) -> u16 {
        // bus reads do not cross pages, they wrap around page boundaries
        let next_addr = (addr & 0xFF00) | ((addr + 1) & 0xFF);
        (self.read(addr) as u16) | ((self.read(next_addr) as u16) << 8)
    }
    fn read_n(&mut self, addr: u16, n: u16) -> Vec<u8> {
        let mut v = Vec::with_capacity(n as usize);
        for idx in 0..n {
            v.push(self.read(addr + idx));
        }
        v
    }
    fn cycles(&self) -> usize;
    fn clock(&mut self, cycles: u8);
    fn get_nmi(&mut self) -> Option<u8>;
}

pub struct NesBus {
    game: Cartridge,
    _controller1: Controller,
    _controller2: Controller,
    ppu: PPU,
    _apu: APU,
    cpu_ram: RAM,
    cycles: usize,
    nmi: Option<u8>,

    cpu_test_enabled: bool,
}

impl NesBus {
    pub fn with_cartridge(game: Cartridge) -> Self {
        NesBus {
            game: game.clone(),
            _controller1: Controller::new(),
            _controller2: Controller::new(),
            ppu: PPU::new(game),
            _apu: APU::new(),
            cpu_ram: RAM::with_size(2048),
            nmi: None,

            cycles: 7,
            cpu_test_enabled: false,
        }
    }
}

impl Bus for NesBus {
    fn read(&self, addr: u16) -> u8 {
        let value = match addr {
            0x0..=0x1FFF => self.cpu_ram.read(addr % 0x800),
            0x2000..=0x3FFF => self.ppu.register_read(addr),
            // 0x4000..=0x4015 => self.apu.read_register(addr - 0x4000),
            // 0x4016 => self.controller1.read(),
            // 0x4017 => self.controller2.read(),
            // 0x4018..=0x401F => {
            //     if self.cpu_test_enabled {
            //         self.apu.read_test_register((addr - 0x4000) % 18);
            //     }
            // }
            0x4020..=0xFFFF => self.game.prg_read(addr),
            _ => (addr >> 8) as u8,
        };
        debug_print!(
            "--- CYC:{} Read value {:X} from addr {:X}",
            self.cycles(),
            value,
            addr
        );
        value
    }

    fn write(&mut self, addr: u16, val: u8) {
        debug_print!(
            "--- CYC:{} Writing value {:X} to addr {:X}",
            self.cycles(),
            val,
            addr
        );

        match addr {
            0x0..=0x1FFF => self.cpu_ram.write(addr % 0x800, val),
            0x4020..=0xFFFF => self.game.prg_write(addr, val),
            0x2000..=0x3FFF => self.ppu.register_write(addr, val),
            _ => {}
        }
    }

    fn cycles(&self) -> usize {
        self.cycles
    }

    fn clock(&mut self, cycles: u8) {
        self.cycles += cycles as usize;
        //self.ppu.clock();
        if self.ppu.generate_nmi() {
            self.nmi = Some(1);
        }
    }

    fn get_nmi(&mut self) -> Option<u8> {
        let nmi = self.nmi;
        self.nmi = None;
        nmi
    }
}
