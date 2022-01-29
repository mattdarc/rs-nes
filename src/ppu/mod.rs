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
use std::mem::size_of;
use std::slice::from_raw_parts;

const COLORS: [u8; 4] = [0, 63, 124, 255];
const WINDOW_NAME: &str = "PPU pattern table";
const NES_SCREEN_WIDTH: u32 = 256;
const NES_SCREEN_HEIGHT: u32 = 240;
const WINDOW_WIDTH_MUL: u32 = 5;
const WINDOW_HEIGHT_MUL: u32 = 3;
const WINDOW_WIDTH: u32 = NES_SCREEN_WIDTH * WINDOW_WIDTH_MUL;
const WINDOW_HEIGHT: u32 = NES_SCREEN_HEIGHT * WINDOW_HEIGHT_MUL;

// This is safe since I know that the underlying data is valid and contiguous
unsafe fn to_sdl2_slice(slice: &[u32]) -> &[u8] {
    from_raw_parts(slice.as_ptr() as *const u8, slice.len() * 4 as usize)
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

        let window = video_subsystem
            .window(WINDOW_NAME, WINDOW_WIDTH, WINDOW_HEIGHT)
            .position_centered()
            .build()
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

        let mut buf = vec![0_u8; 3 * 128 * 256];

        for row in 0..256 {
            for col in 0..128 {
                let addr = (row / 8 * 0x100) + (row % 8) + (col / 8) * 0x10;
                let low_bits = self.game.chr_read(addr);
                let high_bits = self.game.chr_read(addr + 8);
                let value =
                    ((low_bits >> (7 - (col % 8))) & 1) + ((high_bits >> (7 - (col % 8))) & 1) * 2;
                let value = COLORS[value as usize];
                let buf_addr = (row as usize * 128 * 3) + (col as usize * 3);

                buf[buf_addr] = value;
                buf[buf_addr + 1] = value;
                buf[buf_addr + 2] = value;
            }
        }

        let creator = self.canvas.texture_creator();
        let mut texture = creator
            .create_texture(
                Some(PixelFormatEnum::RGB888),
                TextureAccess::Streaming,
                96, // This should be 128 ...
                256,
            )
            .unwrap();

        texture.update(None, &buf, 128 * 3).unwrap();

        let rect = Rect::new(256, 0, 128 * 2, 256 * 2);
        self.canvas.copy(&texture, None, Some(rect)).unwrap();
        self.canvas.present();
    }
}
