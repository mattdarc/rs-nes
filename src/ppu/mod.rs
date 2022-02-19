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
mod sprite;

use crate::cartridge::header::Mirroring;
use crate::cartridge::{Cartridge, CartridgeInterface};
use crate::memory::RAM;
use crate::sdl_interface::graphics;
use flags::*;
use registers::*;
use sprite::Sprite;

const SCANLINES_PER_FRAME: i16 = 262;
const VISIBLE_SCANLINES: i16 = 241;
const CYCLES_PER_SCANLINE: i16 = 341;

pub struct PPU {
    game: Cartridge,
    registers: Registers,
    flags: Flags,
    vram: RAM,
    palette_table: [u8; 32],
    renderer: graphics::Renderer,
    // Sprites
    oam_primary: [Sprite; 64],
    oam_secondary: [Sprite; 8],
    //sprite_pattern_table: [u16; 8],
    //sprite_attrs: [u8; 8],
    //sprite_x_pos: [u16; 8],

    // Background
    //pattern_table_regs: [u16; 2],
    //palette_attr_regs: [u16; 2],
    cycle: i16,
    scanline: i16,
}

/// Mirror the provided address according to the Mirroring `mirror`
///
/// Horizontal:
///   [ A ] [ a ]
///   [ B ] [ b ]
///
/// Vertical:
///   [ A ] [ B ]
///   [ a ] [ b ]
fn mirror(mirror: &Mirroring, addr: u16) -> u16 {
    match mirror {
        // AABB
        Mirroring::Horizontal => addr & 0xBFF,

        // ABAB
        Mirroring::Vertical => addr & 0x7FF,
    }
}

impl PPU {
    pub fn new(game: Cartridge) -> Self {
        PPU {
            game,
            registers: Registers::default(),
            flags: Flags::default(),
            renderer: graphics::Renderer::new(),
            oam_primary: [Sprite::default(); 64],
            oam_secondary: [Sprite::default(); 8],
            palette_table: [0; 32],
            cycle: 0,
            scanline: -1,
            vram: RAM::with_size(0x3000),
        }
    }

    pub fn register_read(&self, addr: u16) -> u8 {
        match (addr - 0x2000) % 8 {
            0 => self.registers.ctrl,
            1 => self.registers.mask,
            2 => self.registers.status,
            3 => self.registers.oamaddr,
            4 => self.registers.oamdata,
            5 => self.registers.scroll,
            6 => panic!("Cannot read PPU address!"),
            7 => self.registers.data,
            _ => panic!("Invalid PPU Register read: address {:X}", addr),
        }
    }

    fn vram_read(&mut self, addr: u16) -> u8 {
        let addr = mirror(self.game.header().get_mirroring(), addr - 0x2000);
        self.vram.read(addr)
    }

    fn ppu_read(&mut self) -> u8 {
        let addr: u16 = self.registers.addr.into();
        let incr_amount = if self.registers.ctrl & PpuCtrl::VRAM_INCR != 0 {
            32
        } else {
            1
        };
        self.registers.addr.incr(incr_amount);

        match addr {
            0..=0x1FFF => self.game.chr_read(addr),
            0x2000..=0x2FFF => self.vram_read(addr),
            0x3F00..=0x3FFF => self.palette_table[(addr - 0x3F00) as usize],
            _ => panic!("Read from out of range address 0x{:X}!", addr),
        }
    }

    pub fn register_write(&mut self, addr: u16, val: u8) {
        match (addr - 0x2000) % 8 {
            0 => self.registers.ctrl = val,
            1 => self.registers.mask = val,
            2 => self.registers.status = val,
            3 => self.registers.oamaddr = val,
            4 => self.registers.oamdata = val,
            5 => self.registers.scroll = val,
            6 => self.registers.addr.write(val),
            7 => self.registers.data = val,
            _ => panic!("Invalid PPU Register write: address {:X}", addr),
        }
    }

    fn is_sprite0_hit(&self) -> bool {
        let sprites_enabled = self.registers.mask & PpuMask::SHOW_SPRITES != 0;
        let show_clipped_lhs =
            self.registers.mask & (PpuMask::SHOW_LEFT_BG | PpuMask::SHOW_LEFT_SPRITES) != 0
                && self.oam_primary[0].x() <= 7;
        let past_rhs = self.oam_primary[0].x() == 255;
        let sprite0_hit_occurred = self.registers.status & PpuStatus::SPRITE_0_HIT != 0;
        sprites_enabled && show_clipped_lhs && !past_rhs && !sprite0_hit_occurred
    }

    pub fn clock(&mut self, ticks: i16) {
        self.cycle += ticks;
        if self.cycle < CYCLES_PER_SCANLINE {
            return;
        }

        if self.is_sprite0_hit() {
            self.registers.status |= PpuStatus::SPRITE_0_HIT;
        }
        self.cycle -= CYCLES_PER_SCANLINE;
        self.scanline += 1;

        if self.scanline == VISIBLE_SCANLINES {
            // End of frame, restart new frame
            self.registers.status |= PpuStatus::VBLANK_STARTED;
            self.registers.status &= !PpuStatus::SPRITE_0_HIT;
            if self.registers.ctrl & PpuCtrl::NMI_ENABLE != 0 {
                self.flags.has_nmi = true;
            }
        }

        if self.scanline == SCANLINES_PER_FRAME {
            self.scanline = 0;
            self.flags.has_nmi = false;
            self.registers.status &= !PpuStatus::SPRITE_0_HIT;
            self.registers.status &= !PpuStatus::VBLANK_STARTED;
            self.flags.odd = !self.flags.odd;
        }
    }

    fn get_nametable_addr(&self) -> u16 {
        match self.registers.ctrl & PpuCtrl::NAMETABLE_ADDR {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => unreachable!("Nametable address should be 2 bits!"),
        }
    }

    fn get_pattable_addr(&self) -> u16 {
        match self.registers.ctrl & PpuCtrl::BG_TABLE_ADDR {
            0 => 0x0000,
            1 => 0x1000,
            _ => unreachable!("Pattern table address should be 1 bit!"),
        }
    }

    pub fn generate_nmi(&self) -> bool {
        self.flags.has_nmi
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

                    const COLORS: [u8; 4] = [0, 85, 170, 255];
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

        const TEX_WIDTH_PX: u32 = 2 * (WIDTH_TILES * TILE_WIDTH_PX) as u32;
        const TEX_HEIGHT_PX: u32 = HEIGHT_PX as u32 / 2;
        self.renderer
            .render_screen_raw(&pattern_table, TEX_WIDTH_PX, TEX_HEIGHT_PX);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn nametable_mirroring() {
        assert_eq!(mirror(&Mirroring::Vertical, 0x0000), 0x0000);
        assert_eq!(mirror(&Mirroring::Vertical, 0x0400), 0x0400);
        assert_eq!(mirror(&Mirroring::Vertical, 0x0038), 0x0038);
        assert_eq!(mirror(&Mirroring::Vertical, 0x0438), 0x0438);
        assert_eq!(mirror(&Mirroring::Vertical, 0x0801), 0x0001);

        assert_eq!(mirror(&Mirroring::Horizontal, 0x0000), 0x0000);
        assert_eq!(mirror(&Mirroring::Horizontal, 0x0400), 0x0000);
        assert_eq!(mirror(&Mirroring::Horizontal, 0x0038), 0x0038);
        assert_eq!(mirror(&Mirroring::Horizontal, 0x0438), 0x0038);
        assert_eq!(mirror(&Mirroring::Horizontal, 0x0838), 0x0838);
        assert_eq!(mirror(&Mirroring::Horizontal, 0x0C38), 0x0838);
    }
}
