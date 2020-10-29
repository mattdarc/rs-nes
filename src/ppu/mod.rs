// PPU implementation for rs-nes

#![allow(dead_code)] // TODO: remove this once

mod registers;

use crate::cartridge::*;
use crate::common::*;
use crate::memory::*;
use crate::sdl_interface::*;
use crate::graphics::*;
use registers::*;

use std::cell::RefCell;

pub const PPU_NUM_FRAMES: usize = 256;
pub const PPU_NUM_SCANLINES: usize = 0;

#[derive(Clone)]
struct Internal {
    status: Status,
    addr_latch: bool,
}

impl Internal {
    fn new() -> RefCell<Internal> {
	RefCell::new(Internal {
	    status: Status(0),
	    addr_latch: false,
	})
    }
}

#[derive(Clone)]
pub struct PPU<'a> {
    renderer: Option<Renderer>,
    cartridge: Option<&'a Cartridge>,

    control: Control,
    mask: Mask,
    state: RefCell<Internal>,
    oam_addr: u8,
    oam_data: [u8; 0x100],
    v_addr: VRAMAddr,
    t_addr: VRAMAddr,
    x_scroll: u8,
    w: bool,
    
    cycle: u32,
    scanline: u32,
}

impl<'a> PPU<'a> {
    const NUM_REGS: usize = 9;

    pub fn new() -> PPU<'a> {
        PPU {
            renderer: None,
            cartridge: None,
	    control: Control(0),
	    mask: Mask(0),
	    state: Internal::new(),
	    oam_addr: 0,
	    oam_data: [0; 256],
	    v_addr: VRAMAddr(0),
	    t_addr: VRAMAddr(0),
	    x_scroll: 0,
	    w: false,

	    cycle: 0,
	    scanline: 0,
        }
    }

    pub fn init(&mut self, cartridge: &'a Cartridge) -> Result<(), Box<dyn std::error::Error>> {
	// self.renderer = Some(Renderer::new()?);
	self.cartridge = Some(cartridge);
	Ok(())
    }

    fn read_oam(&self) -> u8 {
	0
    }

    fn write_oam(&mut self, val: u8) {
    }

    fn write_scroll(&mut self, val: u8) {
    }

    fn write_ppu_addr(&mut self, val: u8) {
    }

    fn write_vram(&mut self, val: u8) {
	self.cartridge.unwrap().chr_write(self.v_addr.read(), val);
    }

    fn read_vram(&self) -> u8 {
	self.cartridge.unwrap().chr_read(self.v_addr.read())
    }

    fn read_status(&self) -> u8 {
	let ret = self.state.borrow().status.0;
	let mut state = self.state.borrow_mut();
	state.status.vblank(false);
	state.addr_latch = false;
	ret
    }
}

// TODO: match against each of the registers
impl Writeable for PPU<'_> {
    fn write(&mut self, addr: usize, val: u8) {
	match addr % PPU::NUM_REGS {
	    PPUCTRL => self.control.write(val),
	    PPUMASK => self.mask.write(val),
	    PPUSTATUS => {}, //panic!("PPUSTATUS unwriteable!"), TODO: Not sure why this is hit 
	    OAMADDR => self.oam_addr = val,
	    OAMDATA => self.write_oam(val),
	    PPUSCROLL => self.write_scroll(val),
	    PPUADDR => self.write_ppu_addr(val),
	    PPUDATA => self.write_vram(val),
	    _ => unreachable!(),
	}
	self.state.borrow_mut().status.set_low(val & 0x1F);
    }
}

// TODO: match against each of the registers
impl Readable for PPU<'_> {
    fn read(&self, addr: usize) -> u8 {
	match addr % PPU::NUM_REGS {
	    PPUCTRL => panic!("PPUCTRL unreadable!"),
	    PPUMASK => panic!("PPUMASK unreadable!"),
	    PPUSTATUS => self.read_status(),
	    OAMADDR => panic!("OAMADDR unreadable!"),
	    OAMDATA => self.read_oam(),
	    PPUSCROLL => panic!("PPUSCROLL unreadable!"),
	    PPUADDR => panic!("PPUADDR unreadable!"),
	    PPUDATA => self.read_vram(),
	    _ => unreachable!(),
	}
    }
}

impl Clocked for PPU<'_> {
    fn clock(&mut self) {
	// OAMADDR is set to 0 during each of ticks 257-320 (the sprite tile loading
	// interval) of the pre-render and visible scanlines.
	if self.cycle > 256 && self.cycle < 321 {
	    self.oam_addr = 0;
	}

	self.cycle = (self.cycle + 1) % 341;
	self.scanline = (self.scanline + 1) % 262;
    }
}
