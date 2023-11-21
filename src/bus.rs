use crate::apu::*;
use crate::cartridge::*;
use crate::controller::*;
use crate::graphics::Renderer;
use crate::memory::*;
use crate::ppu::*;
use crate::timer;
use tracing::{event, Level};

pub const NTSC_CLOCK_MHZ: usize = 1_789_773;
pub const PAL_CLOCK_MHZ: usize = 1_662_607;

pub trait Bus {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
    fn read16(&mut self, addr: u16) -> u16 {
        // Bus reads do not cross pages, they wrap around page boundaries
        let next_addr = (addr & 0xFF00) | ((addr + 1) & 0xFF);
        (self.read(addr) as u16) | ((self.read(next_addr) as u16) << 8)
    }
    fn cycles(&self) -> usize;
    fn clock(&mut self, cycles: usize);
    fn pop_nmi(&mut self) -> Option<u8>;
    fn ppu_state(&self) -> (i16, i16) {
        (0, 0)
    }
}

pub struct NesBus {
    game: Cartridge,
    _controller1: Controller,
    _controller2: Controller,
    ppu: PPU,
    apu: APU,
    cpu_ram: RAM,
    nmi: Option<u8>,

    total_cycles: usize,
    cycles_last_sync: usize,
    last_sync: timer::FastInstant,
}

impl NesBus {
    pub fn new(game: Cartridge, renderer: Box<dyn Renderer>) -> Self {
        NesBus {
            _controller1: Controller::new(),
            _controller2: Controller::new(),
            ppu: PPU::new(&game, renderer),
            apu: APU::new(&game),
            game,
            cpu_ram: RAM::with_size(0x800),
            nmi: None,

            total_cycles: 0,
            cycles_last_sync: 0,
            last_sync: timer::FastInstant::now(),
        }
    }

    fn dump_access(&self, ty: &str, addr: u16, value: u8) {
        event!(
            Level::DEBUG,
            "CYC:{} {} value 0x{:X} @ addr 0x{:X}",
            self.cycles(),
            ty,
            value,
            addr
        );
    }

    fn throttle_to_ntsc(&mut self) {
        const FREERUN_CYCLES: usize = 20_000;
        if self.cycles_last_sync < FREERUN_CYCLES {
            return;
        }

        const SLEEP_OVERHEAD_US: u64 = 400;
        const SYNC_RESOLUTION_US: u64 = (1_000_000 * FREERUN_CYCLES / NTSC_CLOCK_MHZ) as u64;
        const SIMULATED_DURATION: timer::Duration =
            timer::Duration::from_micros(SYNC_RESOLUTION_US - SLEEP_OVERHEAD_US);

        let real_duration = self.last_sync.elapsed();
        if let Some(delta) = SIMULATED_DURATION.checked_sub(real_duration) {
            timer::timed!("sleep", { std::thread::sleep(delta) });
        }

        self.last_sync = timer::FastInstant::now();
        self.cycles_last_sync = 0;
    }
}

impl Bus for NesBus {
    #[tracing::instrument(target = "bus", level = Level::DEBUG, skip(self))]
    fn read(&mut self, addr: u16) -> u8 {
        let value = match addr {
            0x0..=0x1FFF => self.cpu_ram[addr as usize & 0x7FF],
            0x2000..=0x3FFF => self.ppu.register_read(addr - 0x2000),
            0x4000..=0x4015 => self.apu.register_read(addr - 0x4000),
            0x4016 => {
                event!(Level::DEBUG, "read from controller 1");
                0
            }
            0x4017 => {
                event!(Level::DEBUG, "read from controller 2");
                0
            }
            0x4018..=0x401F => {
                event!(Level::DEBUG, "read from APU.test");
                0
            }
            // NOTE: Cartridges use absolute addresses
            0x4020..=0xFFFF => self.game.prg_read(addr),
        };
        self.dump_access("read", addr, value);

        value
    }

    #[tracing::instrument(target = "bus", level = Level::DEBUG, skip(self))]
    fn write(&mut self, addr: u16, val: u8) {
        self.dump_access("write", addr, val);

        match addr {
            0x0..=0x1FFF => self.cpu_ram[addr as usize & 0x7FF] = val,
            0x2000..=0x3FFF => self.ppu.register_write(addr - 0x2000, val),
            0x4000..0x4014 | 0x4015 => self.apu.register_write(addr - 0x4000, val),
            // NOTE: Controllers can be written to to enable strobe mode
            0x4016 => event!(Level::DEBUG, "write to controller 1"),
            0x4017 => event!(Level::DEBUG, "write to controller 2"),
            0x4014 => {
                event!(
                    Level::DEBUG,
                    "CYC:{} OAMDMA from 0x{:04X}",
                    self.cycles(),
                    (val as u16) << 8
                );

                // Writing $XX will upload 256 bytes of data from CPU page $XX00-$XXFF to the
                // internal PPU OAM. This page is typically located in internal RAM, commonly
                // $0200-$02FF, but cartridge RAM or ROM can be used as well.
                //
                // https://www.nesdev.org/wiki/PPU_registers#OAMDATA
                const PAGE_SIZE: usize = 256;
                if val < 0x20 {
                    let page = ((val as usize) << 8) & 0x7FF;
                    self.ppu.oam_dma(&self.cpu_ram[page..(page + PAGE_SIZE)]);
                    return;
                }

                let dma_buffer = (0..PAGE_SIZE as u16)
                    .map(|lo| self.read((val as u16) << 8 | lo))
                    .collect::<Vec<_>>();
                self.ppu.oam_dma(dma_buffer.as_slice());
            }
            // NOTE: Cartridges use absolute addresses
            0x4020..=0xFFFF => self.game.prg_write(addr, val),
            _ => unreachable!(),
        }
    }

    fn cycles(&self) -> usize {
        self.total_cycles
    }

    fn clock(&mut self, cycles: usize) {
        self.total_cycles += cycles;
        self.cycles_last_sync += cycles;

        const PPU_CYCLES_PER: usize = 3;
        timer::timed!("ppu", { self.ppu.clock(PPU_CYCLES_PER * cycles) });

        if self.ppu.generate_nmi() {
            self.nmi = Some(1);
        }

        self.throttle_to_ntsc();
    }

    fn ppu_state(&self) -> (i16, i16) {
        (self.ppu.scanline(), self.ppu.cycle())
    }

    fn pop_nmi(&mut self) -> Option<u8> {
        let nmi = self.nmi;
        self.nmi = None;
        nmi
    }
}
