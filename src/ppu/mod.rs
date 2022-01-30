/* From nesdev wiki

The pattern table is an area of memory connected to the PPU that defines the shapes of tiles that
make up backgrounds and sprites. Each tile in the pattern table is 16 bytes, made of two planes.
The first plane controls bit 0 of the color; the second plane controls bit 1. Any pixel whose color
is 0 is background/transparent (represented by '.' in the following diagram):

                         Bit Planes            Pixel Pattern
              =================================================
              0     $0xx0=$41  01000001
              1     $0xx1=$C2  11000010
              2     $0xx2=$44  01000100
    Plane 1   3     $0xx3=$48  01001000
              4     $0xx4=$10  00010000
              5     $0xx5=$20  00100000         .1.....3
              6     $0xx6=$40  01000000         11....3.
              7     $0xx7=$80  10000000  =====  .1...3..
                                                .1..3...
              0     $0xx8=$01  00000001  =====  ...3.22.
              1     $0xx9=$02  00000010         ..3....2
              2     $0xxA=$04  00000100         .3...2.
              3     $0xxB=$08  00001000         3....222
    Plane 2   4     $0xxC=$16  00010110
              5     $0xxD=$21  00100001
              6     $0xxE=$42  01000010
              7     $0xxF=$87  10000111.

The pattern table is divided into two 256-tile sections: $0000-$0FFF, nicknamed "left", and
$1000-$1FFF, nicknamed "right". The nicknames come from how emulators with a debugger display the
pattern table. Traditionally, they are displayed as two side-by-side 128x128 pixel sections, each
representing 16x16 tiles from the pattern table, with $0000-$0FFF on the left and $1000-$1FFF on
the right.

    Addressing

              DCBA98 76543210
              ---------------
              0HRRRR CCCCPTTT
              |||||| |||||+++- T: Fine Y offset, the row number within a tile
              |||||| ||||+---- P: Bit plane (0: "lower"; 1: "upper")
              |||||| ++++----- C: Tile column
              ||++++---------- R: Tile row
              |+-------------- H: Half of sprite table (0: "left"; 1: "right")
              +--------------- 0: Pattern table is at $0000-$1FFF

*/

mod flags;
mod registers;

use crate::cartridge::{Cartridge, CartridgeInterface};
use crate::sdl_interface::SDL2Intrf;
use flags::*;
use registers::*;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{TextureAccess, WindowCanvas};
use sdl2::surface::Surface;
use sdl2::video::DisplayMode;
use std::mem::size_of;
use std::slice::from_raw_parts;

const COLORS: [u8; 4] = [0, 85, 170, 255];
const WINDOW_NAME: &str = "PPU pattern table";
const NES_SCREEN_WIDTH: u32 = 256;
const NES_SCREEN_HEIGHT: u32 = 240;
const WINDOW_WIDTH_MUL: u32 = 5;
const WINDOW_HEIGHT_MUL: u32 = 3;
const WINDOW_WIDTH: u32 = NES_SCREEN_WIDTH * WINDOW_WIDTH_MUL;
const WINDOW_HEIGHT: u32 = NES_SCREEN_HEIGHT * WINDOW_HEIGHT_MUL;

// for some reason textures are repeating every 120 bytes
fn dump_texture_buf(buf: &[u8], px_size: usize) {
    let width = 128;

    let mut s = String::new();
    for idx in (0..buf.len()).step_by(px_size) {
        if idx % (width * px_size) == 0 {
            s.push('\n');
        }

        let val = buf[idx];
        if val != buf[idx + 1] || val != buf[idx + 2] {
            s.push('#');
        } else {
            match val {
                85 | 170 | 255 => s.push(char::from_digit((val / 85) as u32, 10).unwrap()),
                0 => s.push('.'),
                _ => s.push('?'),
            }
        }
    }

    println!("\nTiles:\n{}", &s);
}

// This is safe since I know that the underlying data is valid and contiguous
fn to_sdl2_slice(slice: &[u32]) -> &[u8] {
    unsafe { from_raw_parts(slice.as_ptr() as *const u8, slice.len() * 4 as usize) }
}

pub struct PPU {
    game: Cartridge,
    registers: Registers,
    flags: Flags,

    canvas: WindowCanvas,
}

impl PPU {
    pub fn new(game: Cartridge) -> Self {
        let sdl_ctx = SDL2Intrf::context();
        let video_subsystem = sdl_ctx.video().unwrap();

        let mut window = video_subsystem
            .window(WINDOW_NAME, WINDOW_WIDTH, WINDOW_HEIGHT)
            .position_centered()
            .build()
            .unwrap();
        window
            .set_display_mode(Some(DisplayMode::new(
                PixelFormatEnum::RGB888,
                WINDOW_WIDTH as i32,
                WINDOW_HEIGHT as i32,
                30,
            )))
            .unwrap();

        let mut canvas = window.into_canvas().build().unwrap();
        canvas.clear();

        PPU {
            game,
            registers: Registers::default(),
            flags: Flags::default(),
            canvas,
        }
    }

    pub fn register_write(&mut self, addr: u16, val: u8) {
        match (addr - 0x2000) % 8 {
            0 => self.registers.ctrl = PpuCtrl::from_bits(val).expect("All bits covered"),
            1 => self.registers.mask = PpuMask::from_bits(val).expect("All bits covered"),
            2 => self.registers.status = PpuStatus::from_bits(val).expect("All bits covered"),
            3 => self.registers.oamaddr = val,
            4 => self.registers.oamdata = val,
            5 => self.registers.scroll = val,
            6 => self.registers.addr = val,
            7 => self.registers.data = val,
            _ => panic!("Invalid PPU Register write: address {:X}", addr),
        }
    }

    pub fn register_read(&self, addr: u16) -> u8 {
        match (addr - 0x2000) % 8 {
            0 => self.registers.ctrl.bits(),
            1 => self.registers.mask.bits(),
            2 => self.registers.status.bits(),
            3 => self.registers.oamaddr,
            4 => self.registers.oamdata,
            5 => self.registers.scroll,
            6 => self.registers.addr,
            7 => self.registers.data,
            _ => panic!("Invalid PPU Register read: address {:X}", addr),
        }
    }

    pub fn clock(&mut self) {
        self.flags.odd = !self.flags.odd;
        self.show_pattern_table();
    }

    fn show_pattern_table(&mut self) {
        const TILE_WIDTH_PX: u16 = 8;
        const TILE_HEIGHT_PX: u16 = 8;
        const PX_SIZE_BYTES: usize = 4; // 4th byte for the pixel is unused
        const HEIGHT_PX: u16 = 256;
        const WIDTH_TILES: u16 = 16;
        const WIDTH_PX: u16 = WIDTH_TILES * TILE_WIDTH_PX;
        const BUF_SIZE_BYTES: usize = PX_SIZE_BYTES * (HEIGHT_PX as usize) * (WIDTH_PX as usize);

        let mut buf = vec![0_u8; BUF_SIZE_BYTES];

        // The pattern table has a tile adjacent in memory, while SDL renders entire rows. When
        // reading the pattern table we need to add an offset that is the tile number
        //
        // Concretely, the first row of the SDL texture contains the first row of 16 tiles, which
        // are actually offset 16 bytes from each other. Display the tiles side-by-side so we have
        for row in 0..HEIGHT_PX {
            for col in 0..WIDTH_TILES {
                let tile_num_down = row / TILE_HEIGHT_PX;
                let row_offset = row % TILE_HEIGHT_PX;

                const TILE_SIZE_BYTES: u16 = 16;
                let chr_addr = row_offset
                    + (col * TILE_SIZE_BYTES)
                    + (tile_num_down * TILE_SIZE_BYTES * WIDTH_TILES);

                const HIGH_OFFSET_BYTES: u16 = 8;
                let low_byte = self.game.chr_read(chr_addr);
                let high_byte = self.game.chr_read(chr_addr + HIGH_OFFSET_BYTES);

                for px in 0..TILE_WIDTH_PX {
                    let color_idx =
                        ((low_byte >> (7 - px)) & 1) + (((high_byte >> (7 - px)) & 1) << 1);
                    let color = COLORS[color_idx as usize];
                    let buf_addr = PX_SIZE_BYTES
                        * (px as usize + ((row * WIDTH_TILES + col) * TILE_WIDTH_PX) as usize);

                    // Assign all pixels as the same color value so we get a grayscale version
                    buf[buf_addr..(buf_addr + PX_SIZE_BYTES)]
                        .swap_with_slice(&mut [color; PX_SIZE_BYTES]);
                }
            }
        }

        // Format pattern table s.t. 0x000-0x0FFF are on the left and 0x1000-0x1FFF are on the
        // right
        const HALF: usize = BUF_SIZE_BYTES / 2;
        let pattern_table = buf[0..HALF]
            .chunks(WIDTH_PX as usize * PX_SIZE_BYTES)
            .zip(buf[HALF..].chunks(WIDTH_PX as usize * PX_SIZE_BYTES))
            .flat_map(|(l, r)| [l, r].concat())
            .collect::<Vec<_>>();
        assert_eq!(pattern_table.len(), buf.len());

        // dump_texture_buf(&pattern_table, PX_SIZE_BYTES);

        const TEX_WIDTH_PX: u32 = 2 * (WIDTH_TILES * TILE_WIDTH_PX) as u32;
        const TEX_HEIGHT_PX: u32 = HEIGHT_PX as u32 / 2;
        let creator = self.canvas.texture_creator();
        let mut texture = creator
            .create_texture(None, TextureAccess::Streaming, TEX_WIDTH_PX, TEX_HEIGHT_PX)
            .unwrap();

        const PITCH_BYTES: usize = PX_SIZE_BYTES * TEX_WIDTH_PX as usize;
        texture.update(None, &pattern_table, PITCH_BYTES).unwrap();
        self.canvas.copy(&texture, None, None).unwrap();
        self.canvas.present();
    }
}
