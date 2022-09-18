use crate::apu::*;
use crate::cartridge::*;
use crate::controller::*;
use crate::graphics::Renderer;
use crate::memory::*;
use crate::ppu::*;
use tracing::{event, Level};

pub trait Bus {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
    fn read16(&mut self, addr: u16) -> u16 {
        // Bus reads do not cross pages, they wrap around page boundaries
        let next_addr = (addr & 0xFF00) | ((addr + 1) & 0xFF);
        (self.read(addr) as u16) | ((self.read(next_addr) as u16) << 8)
    }
    fn read_n(&mut self, addr: u16, n: u16) -> Vec<u8> {
        (0..n).map(|idx| self.read(addr + idx)).collect::<Vec<_>>()
    }
    fn cycles(&self) -> usize;
    fn clock(&mut self, cycles: u8);
    fn pop_nmi(&mut self) -> Option<u8>;
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
    pub fn new(game: Cartridge, renderer: Box<dyn Renderer>) -> Self {
        NesBus {
            game: game.clone(),
            _controller1: Controller::new(),
            _controller2: Controller::new(),
            ppu: PPU::new(game, renderer),
            _apu: APU::new(),
            cpu_ram: RAM::with_size(2048),
            nmi: None,

            cycles: 7,
            cpu_test_enabled: false,
        }
    }

    fn dump_instr(&self, ty: &str, addr: u16, value: u8) {
        event!(
            Level::DEBUG,
            "CYC:{} {} value 0x{:X} @ addr 0x{:X}",
            self.cycles(),
            ty,
            value,
            addr
        );
    }
}

impl Bus for NesBus {
    #[tracing::instrument(target = "bus", skip(self), ret)]
    fn read(&mut self, addr: u16) -> u8 {
        // FIXME: *Could* make each of these components conform to a common interface which has
        // read/write register, but the NES is fixed HW so I don't see the benefit ATM
        let value = match addr {
            0x0..=0x1FFF => self.cpu_ram.read(addr),
            0x2000..=0x3FFF => self.ppu.register_read(addr),
            0x4000..=0x4015 => {
                event!(Level::INFO, "read from APU");
                0
            }
            0x4016 => {
                event!(Level::INFO, "read from controller 1");
                0
            }
            0x4017 => {
                event!(Level::INFO, "read from controller 2");
                0
            }
            0x4018..=0x401F => {
                event!(Level::INFO, "read from APU.test");
                0
            }
            0x4020..=0xFFFF => self.game.prg_read(addr),
        };
        value
    }

    #[tracing::instrument(target = "bus", skip(self))]
    fn write(&mut self, addr: u16, val: u8) {
        self.dump_instr("write", addr, val);

        match addr {
            0x0..=0x1FFF => self.cpu_ram.write(addr % 0x800, val),
            0x4000..0x4014 | 0x4015 => {} // self.apu.write_register(addr - 0x4000, val),
            // NOTE: Controllers can be written to to enable strobe mode
            0x4016 => event!(Level::INFO, "write to controller 1"),
            0x4017 => event!(Level::INFO, "write to controller 2"),
            0x4020..=0xFFFF => self.game.prg_write(addr, val),
            0x2000..=0x3FFF => self.ppu.register_write(addr, val),
            0x4014 => {
                // FIXME: Could make this a direct access from the page and not a bunch of bus reads
                const PAGE_SIZE: u16 = 256;
                let dma_buffer = (0..PAGE_SIZE)
                    .map(|lo| self.read((val as u16) << 8 | lo))
                    .collect::<Vec<_>>();
                self.ppu.oam_dma(dma_buffer.as_slice());
            }
            _ => panic!(
                "Tried to write 0x{:X} to read-only address 0x{:X}",
                val, addr
            ),
        }
    }

    fn cycles(&self) -> usize {
        self.cycles
    }

    fn clock(&mut self, cycles: u8) {
        self.cycles += cycles as usize;
        self.ppu.clock(3 * cycles as i16);
        if self.ppu.generate_nmi() {
            self.nmi = Some(1);
        }
    }

    fn pop_nmi(&mut self) -> Option<u8> {
        let nmi = self.nmi;
        self.nmi = None;
        nmi
    }
}
