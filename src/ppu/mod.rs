// PPU implementation for rs-nes

#![allow(dead_code)] // TODO: remove this once

mod registers;
mod sprite;

use crate::cartridge::*;
use crate::common::*;
use crate::graphics::*;
use crate::memory::*;
use crate::sdl_interface::*;
use registers::*;

use std::cell::RefCell;

pub const PPU_NUM_FRAMES: usize = 256;
pub const PPU_NUM_SCANLINES: usize = 0;

#[derive(Clone)]
struct Internal {
    status: Status,
    addr_latch: bool,
    second_write: bool,
}

impl Internal {
    fn new() -> RefCell<Internal> {
        RefCell::new(Internal {
            status: Status(0),
            addr_latch: false,
            second_write: false,
        })
    }
}

#[derive(Clone)]
pub struct PPU<'a> {
    renderer: Option<Renderer>,
    cartridge: Option<&'a Cartridge>,

    // background registers
    control: Control,
    mask: Mask,
    state: RefCell<Internal>,
    oam_addr: u8,
    v_addr: VRAMAddr,
    t_addr: VRAMAddr,
    x_scroll: u8,
    bg_pat_tbl: [u16; 2],
    pal_attr: [u8; 2],

    // sprite registers. TODO These should be stored in the sprite struct I think, at least the
    // sprite_* ones, then the oam should actually just be a
    //    [Option<Sprite>; 64]
    oam_data: [u8; PPU::OAM_SIZE],
    secondary_oam: [u8; 32],
    sprite_pat_tbl: [u16; 8],
    sprite_attr: [u8; 8],
    sprite_counter: [u8; 8],

    cycle: u32,
    scanline: u32,
}

impl<'a> PPU<'a> {
    // The OAM (Object Attribute Memory) is internal memory inside 
    // the PPU that contains a display list of up to 64 sprites, where 
    // each sprite's information occupies 4 bytes.
    const OAM_SIZE: usize = 0x100;
    const NUM_REGS: usize = 9;

    pub fn new() -> PPU<'a> {
        PPU {
            renderer: None,
            cartridge: None,
            control: Control(0),
            mask: Mask(0),
            state: Internal::new(),
            oam_addr: 0,
            secondary_oam: [0; 32],
            oam_data: [0; PPU::OAM_SIZE],
            v_addr: VRAMAddr(0),
            t_addr: VRAMAddr(0),
            x_scroll: 0,

            bg_pat_tbl: [0; 2],
            pal_attr: [0; 2],

            sprite_pat_tbl: [0; 8],
            sprite_attr: [0; 8],
            sprite_counter: [0; 8],

            cycle: 0,
            scanline: 0,
        }
    }

    pub fn init(
        &mut self, cartridge: &'a Cartridge,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // self.renderer = Some(Renderer::new()?);
        self.cartridge = Some(cartridge);
        Ok(())
    }

    fn scanline(&mut self) {
        // 1. clear list of sprites to draw
        // 2. read through OAM, choose first 8 sprites to render
        // 3. set sprite overflow for > 8 sprites
        // 4. actually draw the sprites

        self.scanline = (self.scanline + 1) % 262;
    }

    // If the sprite has foreground priority or the BG pixel is zero, the sprite
    // pixel is output. 
    // If the sprite has background priority and the BG pixel is nonzero, the BG
    // pixel is output
    fn mux_pixel(&mut self, bg: u8, sprite: u8) -> u8 { 0 }
    
    // OAM data is made up of byte
    //   0) Y pos of top of sprite (plus 1, need to sub 1)
    //   1) index number
    //      8x8: tile number within the pattern table PPUCTRL[3]
    //      76543210
    //      ||||||||
    //      |||||||+- Bank ($0000 or $1000) of tiles
    //      +++++++-- Tile number of top of sprite (0 to 254; 
    //                bottom half gets the next tile)
    //   2) 76543210
    //      ||||||||
    //      ||||||++- Palette (4 to 7) of sprite
    //      |||+++--- Unimplemented
    //      ||+------ Priority (0: in front of background; 
    //                          1: behind background)
    //      |+------- Flip sprite horizontally
    //      +-------- Flip sprite vertically
    //   3) X position of left side of sprite.
    fn clock_cycle(&mut self) {
        let mut n = 0;

        // Every cycle, a bit is fetched from the 4 background shift registers
        // in order to create a pixel on screen. Exactly which bit is fetched
        // depends on the fine X scroll, set by $2005

        // Every 8 cycles/shifts, new data is loaded into these registers.

        // Every cycle, the 8 x-position counters for the sprites are
        // decremented by one. If the counter is zero, 
        //      - the sprite becomes "active", and the respective pair of shift
        //      registers for the sprite is shifted once every cycle. This output
        //      accompanies the data in the sprite's latch, to form a pixel. 
        //      - current pixel for each "active" sprite is checked (from highest
        //      to lowest priority), and the first non-transparent pixel moves
        //      on to a multiplexer, where it joins the BG pixel.
      
        if self.cycle > 64 && self.cycle < 257 {
            match self.cycle % 2 {
                0 => {
                    // Even cycle, write data to secondary oam unless full, then read
                },
                1 => {
                    // Odd cycle, read data from primary oam
                },
                _ => unreachable!(),
            }
        }

        match self.scanline {
            0..=239 => {
                match self.cycle {
                    0 => {}, // idle
                    1..=256 => {
                        // data fetching, TODO should this be using vram_read?
                    },
                    257..=320 => {
                        // fetch sprite tile data
                    },
                    321..=336 => {
                        // fetch firt two tiles for next scanline
                    },
                    337..=340 => {
                        // fetch unknown nametable bytes
                    },
                    _ => unreachable!("cycle overflow!"),
                }
            },
            240 => {},
            241..=260 => {
            },
            261 => {},
            _ => unreachable!("scanline overflow!"),
        }

        self.cycle = (self.cycle + 1) % 341;
    }

    fn read_oam(&self) -> u8 {
        if self.cycle < 65 {
            0xFF
        } else {
            0
        }
    }

    fn write_oam(&mut self, val: u8) {}

    fn write_scroll(&mut self, val: u8) {
        let second_write = self.state.borrow().second_write;
        if second_write {
            // Second write: w == 1
            self.t_addr.fine_y((val as u16) & 0x7);
            self.t_addr.coarse_y((val as u16) >> 3);
        } else {
            // First write: w == 0
            self.t_addr.coarse_x((val as u16) >> 3);
            self.x_scroll = val & 0x7;
        }
        self.state.borrow_mut().second_write = !second_write;
    }

    fn write_ppu_addr(&mut self, val: u8) {
        let second_write = self.state.borrow().second_write;
        if second_write {
            self.t_addr.write_low(val)
        } else {
            let modif = (val & 0x3F) as u16;
            let old_val = self.t_addr.read() & 0x80FF;
            self.t_addr.write((modif << 8) | old_val);
        }

        // toggle first/second write
        self.state.borrow_mut().second_write = !second_write;
    }

    fn write_vram(&mut self, val: u8) {
        self.cartridge.unwrap().chr_write(self.v_addr.read() as usize, val);
    }

    fn read_vram(&self) -> u8 {
        self.cartridge.unwrap().chr_read(self.v_addr.read() as usize)
    }

    fn read_status(&self) -> u8 {
        let ret = self.state.borrow().status.0;
        let mut state = self.state.borrow_mut();
        state.status.vblank(false);
        state.addr_latch = false;
        state.second_write = false;
        ret
    }
}

impl Writeable for PPU<'_> {
    fn write(&mut self, addr: usize, val: u8) {
        match addr % PPU::NUM_REGS {
            PPUCTRL => {
                self.control.write(val);
                self.t_addr.nametable_sel(val.into());
            }
            PPUMASK => self.mask.write(val),
            PPUSTATUS => {} //panic!("PPUSTATUS unwriteable!"), TODO: Not sure why this is hit
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

// TODO: without comments causes failure due to missing implementation
impl Readable for PPU<'_> {
    fn read(&self, addr: usize) -> u8 {
        return 0;

        // match addr % PPU::NUM_REGS {
        //     PPUCTRL => panic!("PPUCTRL unreadable!"),
        //     PPUMASK => panic!("PPUMASK unreadable!"),
        //     PPUSTATUS => self.read_status(),
        //     OAMADDR => panic!("OAMADDR unreadable!"),
        //     OAMDATA => self.read_oam(),
        //     PPUSCROLL => panic!("PPUSCROLL unreadable!"),
        //     PPUADDR => panic!("PPUADDR unreadable!"),
        //     PPUDATA => self.read_vram(),
        //     _ => unreachable!(),
        // }
    }
}

impl Clocked for PPU<'_> {
    fn clock(&mut self) {
        // OAMADDR is set to 0 during each of ticks 257-320 (the sprite tile loading
        // interval) of the pre-render and visible scanlines.
        if self.cycle > 256 && self.cycle < 321 {
            self.oam_addr = 0;
        }

    }
}
