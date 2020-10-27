// PPU implementation for rs-nes

mod nametable;
mod sdl_interface;

use crate::cartridge::*;
use crate::common::*;
use crate::memory::*;
use crate::ppu::sdl_interface::*;

pub const PPU_NUM_FRAMES: usize = 256;
pub const PPU_NUM_SCANLINES: usize = 0;

#[derive(Clone)]
pub struct PPU {
    vram: RAM,
    registers: [u8; 8],
    sdl: SDL2Intrf,
}

#[derive(Copy, Clone, Debug)]
enum SpriteSize {
    P8x8,
    P8x16,
}

#[derive(Copy, Clone, Debug)]
enum EXTPins {
    ReadBackdrop,
    WriteColor,
}

struct Scroll(u32, u32);

impl PPU {
    const PPUCTRL: usize = 0;
    const PPUMASK: usize = 1;
    const PPUSTATUS: usize = 2;

    pub fn new() -> PPU {
        PPU {
            vram: RAM::new(PPU_NUM_FRAMES * PPU_NUM_SCANLINES),
            registers: [0; 8],
            sdl: SDL2Intrf::new(),
        }
    }

    pub fn init(
        &mut self,
        cartridge: &Cartridge,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.sdl.init(cartridge)
    }

    fn base_nametable_addr(&self) -> usize {
        // After power/reset, writes to this register are ignored for about
        // 30,000 cycles.
        (self.registers[PPU::PPUCTRL] & 0x3) as usize
    }

    fn vram_increment(&self) -> usize {
        if self.registers[PPU::PPUCTRL] & 0x4 != 0 {
            32
        } else {
            1
        }
    }

    fn sprite_table_addr(&self) -> usize {
        if self.registers[PPU::PPUCTRL] & 0x8 != 0 {
            0x1000
        } else {
            0x0000
        }
    }

    fn bg_table_addr(&self) -> usize {
        if self.registers[PPU::PPUCTRL] & 0x10 != 0 {
            0x1000
        } else {
            0x0000
        }
    }

    fn sprite_size(&self) -> SpriteSize {
        if self.registers[PPU::PPUCTRL] & 0x20 != 0 {
            SpriteSize::P8x16
        } else {
            SpriteSize::P8x8
        }
    }

    fn master_slave_sel(&self) -> EXTPins {
        if self.registers[PPU::PPUCTRL] & 0x40 != 0 {
            EXTPins::WriteColor
        } else {
            EXTPins::ReadBackdrop
        }
    }

    fn gen_nmi(&self) -> bool {
        self.registers[PPU::PPUCTRL] & 0x80 != 0
    }

    fn scroll_pos(&self) -> Scroll {
        let mut scroll = Scroll(0, 0);
        if self.registers[PPU::PPUCTRL] & 0x1 != 0 {
            scroll.0 = 256;
        } else if self.registers[PPU::PPUCTRL] & 0x2 != 0 {
            scroll.1 = 240;
        }
        scroll
    }

    fn grayscale(&self) -> bool {
        self.registers[PPU::PPUMASK] & 0x1 != 0
    }

    fn show_leftmost_bg(&self) -> bool {
        self.registers[PPU::PPUMASK] & 0x2 != 0
    }

    fn show_leftmost_sprites(&self) -> bool {
        self.registers[PPU::PPUMASK] & 0x4 != 0
    }

    fn show_bg(&self) -> bool {
        self.registers[PPU::PPUMASK] & 0x8 != 0
    }

    fn show_sprites(&self) -> bool {
        self.registers[PPU::PPUMASK] & 0x10 != 0
    }

    fn emph_red(&self) -> bool {
        self.registers[PPU::PPUMASK] & 0x20 != 0
    }

    fn emph_green(&self) -> bool {
        self.registers[PPU::PPUMASK] & 0x40 != 0
    }

    fn emph_blue(&self) -> bool {
        self.registers[PPU::PPUMASK] & 0x80 != 0
    }
}

impl Writeable for PPU {
    fn write(&mut self, addr: usize, val: u8) {
	let num_regs = self.registers.len();
        self.registers[(addr - 0x2000) % num_regs] = val;
    }
}

impl Readable for PPU {
    fn read(&self, addr: usize) -> u8 {
	let num_regs = self.registers.len();
        self.registers[(addr - 0x2000) % num_regs]
    }
}

impl Clocked for PPU {
    fn clock(&mut self) {}
}
