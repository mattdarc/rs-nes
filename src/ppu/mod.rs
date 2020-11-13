// PPU implementation for rs-nes

#![allow(dead_code)] // TODO remove this once we get to a good stopping point

mod registers;
mod sprite;

use crate::common::*;
use crate::graphics::*;
use registers::*;
use sprite::*;

use std::cell::RefCell;

pub const PPU_NUM_FRAMES: usize = 256;
pub const PPU_NUM_SCANLINES: usize = 0;
const CYCLE_STRIDE: u32 = 8;

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

const OAM_SIZE: usize = 0x100;
const NUM_REGS: usize = 9;

pub struct PPU {
    renderer: Option<Renderer>,

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
    oam_data: [u8; OAM_SIZE],
    secondary_oam: [u8; 32],
    sprite_pat_tbl: [u16; 8],
    sprite_attr: [u8; 8],
    sprite_counter: [u8; 8],

    scanline_data: [u8; 256],

    cycle: u32,
    scanline: i32,
}

impl PPU {
    // The OAM (Object Attribute Memory) is internal memory inside
    // the PPU that contains a display list of up to 64 sprites, where
    // each sprite's information occupies 4 bytes.
    pub fn new() -> PPU {
        PPU {
            renderer: None,
            control: Control(0),
            mask: Mask(0),
            state: Internal::new(),
            oam_addr: 0,
            secondary_oam: [0; 32],
            oam_data: [0; OAM_SIZE],
            v_addr: VRAMAddr(0),
            t_addr: VRAMAddr(0),
            x_scroll: 0,

            bg_pat_tbl: [0; 2],
            pal_attr: [0; 2],

            sprite_pat_tbl: [0; 8],
            sprite_attr: [0; 8],
            sprite_counter: [0; 8],

            scanline_data: [0; 256],

            cycle: 0,
            scanline: 0,
        }
    }

    pub fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.renderer = Some(Renderer::new()?);
        Ok(())
    }

    // If the sprite has foreground priority or the BG pixel is zero, the sprite
    // pixel is output.
    // If the sprite has background priority and the BG pixel is nonzero, the BG
    // pixel is output
    fn mux_pixel(&mut self, bg: u8, sprite: u8) -> u8 {
        0
    }

    fn do_scanline(&mut self) {
        // 1. clear list of sprites to draw
        // 2. read through OAM, choose first 8 sprites to render
        // 3. set sprite overflow for > 8 sprites
        // 4. actually draw the sprites

        // TODO For cycle accurate...
        // match self.scanline {
        //     0..=239 => {
        //         match self.cycle {
        //             0 => {} // idle
        //             1..=256 => {
        //                 // data fetching, TODO should this be using vram_read?
        //             }
        //             257..=320 => {
        //                 // fetch sprite tile data
        //             }
        //             321..=336 => {
        //                 // fetch firt two tiles for next scanline
        //             }
        //             337..=340 => {
        //                 // fetch unknown nametable bytes
        //             }
        //             _ => unreachable!("cycle overflow!"),
        //         }
        //     }
        //     240 => {}
        //     241..=260 => {}
        //     261 => {}
        //     _ => unreachable!("scanline overflow!"),
        // }
        // TODO clean this up
        println!("Rendering scanline");
        match self.renderer.as_mut() {
            Some(r) => r.render(self.scanline, &self.scanline_data),
            None => {
                let mut r = Renderer::new().unwrap();
                let res = r.render(self.scanline, &self.scanline_data);
                self.renderer = Some(r);
                res
            }
        }
        .unwrap();
    }

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
    fn do_cycle<BusType: Bus>(&mut self, bus: &mut BusType) {
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

        let x_pix = self.cycle;
        let y_pix = self.scanline;

        // render sprites, reading from the primary oam, only if the flags are not set to hide them
        if self.mask.show_sprites() && (x_pix > 7 || self.mask.show_left_sprites()) {
            for sprite in self
                .oam_data
                .chunks(Sprite::BYTES_PER)
                .map(|data| Sprite::from(data))
            {
                let addr = match self.control.sprite_size() {
                    Size::Small => sprite.addr() + self.control.sprite_table_addr(),
                    Size::Large => sprite.addr() + sprite.table_addr(),
                };

                let mut colors = vec![0_u8; 8];

                // Determine the colors of the current row from the pattern table. Each pattern
                // table is made up of 16 bytes
                //     - 2 bytes per row
                //     - 1 byte per plane per row
                // The "left" pattern table are values less than 0x1000, while the right pattern table is >= 0x1000
                let lsb = bus.read(addr);
                let msb = bus.read(addr + 8);
                for (i, color) in colors.iter_mut().enumerate() {
                    *color = (lsb >> i) & 1
                        | ((msb >> i) & 1) << 1
                        | (sprite.palette_num() << 2);
                }

                // write the color to the write position in the scanline
                self.scanline_data[x_pix as usize..(x_pix + CYCLE_STRIDE) as usize]
                    .clone_from_slice(&colors);

                println!("Scanline data {:?}", &self.scanline_data);
            }
        }

        if self.cycle > 64 && self.cycle < 257 {
            match self.cycle % 2 {
                0 => {
                    // Even cycle, write data to secondary oam unless full, then read
                }
                1 => {
                    // Odd cycle, read data from primary oam
                }
                _ => unreachable!(),
            }
        }

        self.cycle = (self.cycle + CYCLE_STRIDE) % 341;
        if self.cycle == 0 {
            self.do_scanline();
            self.scanline = (self.scanline + 1) % 262;
        }
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

    fn read_status(&self) -> u8 {
        let ret = self.state.borrow().status.0;
        let mut state = self.state.borrow_mut();
        state.status.vblank(false);
        state.addr_latch = false;
        state.second_write = false;
        ret
    }

    pub fn write(&mut self, addr: usize, val: u8) {
        match addr % NUM_REGS {
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
            //PPUDATA => bus.write(self.v_addr.read() as usize, val),
            _ => unreachable!(),
        }
        self.state.borrow_mut().status.set_low(val & 0x1F);
    }

    // TODO: without comments causes failure due to missing implementation
    #[allow(unreachable_code)]
    pub fn read(&self, _addr: usize) -> u8 {
        return 0;

        match _addr % NUM_REGS {
            PPUCTRL => panic!("PPUCTRL unreadable!"),
            PPUMASK => panic!("PPUMASK unreadable!"),
            PPUSTATUS => self.read_status(),
            OAMADDR => panic!("OAMADDR unreadable!"),
            OAMDATA => self.read_oam(),
            PPUSCROLL => panic!("PPUSCROLL unreadable!"),
            PPUADDR => panic!("PPUADDR unreadable!"),
            //PPUDATA => bus.read(self.v_addr.read() as usize),
            _ => unreachable!(),
        }
    }

    pub fn is_ppu_data(&self, addr: usize) -> bool {
        addr % NUM_REGS == PPUDATA
    }

    pub fn vram_addr(&self) -> usize {
        self.v_addr.read() as usize
    }
}

impl<BusType: Bus> Clocked<BusType> for PPU {
    fn clock(&mut self, bus: &mut BusType) {
        // OAMADDR is set to 0 during each of ticks 257-320 (the sprite tile loading
        // interval) of the pre-render and visible scanlines.
        if self.cycle > 256 && self.cycle < 321 {
            self.oam_addr = 0;
        } else {
            self.do_cycle(bus);
        }
    }
}
