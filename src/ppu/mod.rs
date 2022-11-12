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

use crate::cartridge::header::{Header, Mirroring};
use crate::cartridge::{Cartridge, CartridgeInterface};
use crate::graphics::Renderer;
use crate::memory::RAM;
use flags::*;
use registers::*;
use sprite::Sprite;
use tracing::{event, Level};

const SCANLINES_PER_FRAME: i16 = 262;
const VISIBLE_SCANLINES: i16 = 240;
const CYCLES_PER_SCANLINE: i16 = 341;
const STARTUP_SCANLINES: i16 = 30_000_i16 / CYCLES_PER_SCANLINE;

const PX_SIZE_BYTES: usize = 4; // 4th byte for the pixel is unused

const TILE_WIDTH_PX: u16 = 8;
const TILE_HEIGHT_PX: u16 = 8;
const TILE_SIZE_BYTES: u16 = 16;

const FRAME_WIDTH_TILES: u16 = 32;
const FRAME_HEIGHT_TILES: u16 = 30;
const FRAME_HEIGHT_PX: u16 = 256;
const FRAME_WIDTH_PX: u16 = FRAME_WIDTH_TILES * TILE_WIDTH_PX;
const FRAME_SIZE_BYTES: usize =
    PX_SIZE_BYTES * (FRAME_HEIGHT_PX as usize) * (FRAME_WIDTH_PX as usize);

const PALETTE_TABLE: [u32; 64] = [
    0x7C7C7C00, 0x0000FC00, 0x0000BC00, 0x4428BC00, 0x94008400, 0xA8002000, 0xA8100000, 0x88140000,
    0x50300000, 0x00780000, 0x00680000, 0x00580000, 0x00405800, 0x00000000, 0x00000000, 0x00000000,
    0xBCBCBC00, 0x0078F800, 0x0058F800, 0x6844FC00, 0xD800CC00, 0xE4005800, 0xF8380000, 0xE45C1000,
    0xAC7C0000, 0x00B80000, 0x00A80000, 0x00A84400, 0x00888800, 0x00000000, 0x00000000, 0x00000000,
    0xF8F8F800, 0x3CBCFC00, 0x6888FC00, 0x9878F800, 0xF878F800, 0xF8589800, 0xF8785800, 0xFCA04400,
    0xF8B80000, 0xB8F81800, 0x58D85400, 0x58F89800, 0x00E8D800, 0x78787800, 0x00000000, 0x00000000,
    0xFCFCFC00, 0xA4E4FC00, 0xB8B8F800, 0xD8B8F800, 0xF8B8F800, 0xF8A4C000, 0xF0D0B000, 0xFCE0A800,
    0xF8D87800, 0xD8F87800, 0xB8F8B800, 0xB8F8D800, 0x00FCFC00, 0xF8D8F800, 0x00000000, 0x00000000,
];

pub struct PPU {
    frame_buf: [u8; FRAME_SIZE_BYTES],
    cartridge: Cartridge,

    // Cache the header in the PPU so we don't need to keep dispatching to the shared cartridge.
    // The header will not change for the lifetime of the game
    cartridge_header: Header,
    registers: Registers,
    flags: Flags,
    vram: RAM,
    palette_table: [u8; 32],
    renderer: Box<dyn Renderer>,

    // Sprites
    oam_primary: [u8; 256], // Reinterpreted as sprites
    oam_secondary: [Sprite; 8],
    //sprite_pattern_table: [u16; 8],
    //sprite_attrs: [u8; 8],
    //sprite_x_pos: [u16; 8],

    // Background
    //pattern_table_regs: [u16; 2],
    //palette_attr_regs: [u16; 2],
    cycle: i16,
    scanline: i16,
    frame: usize,

    fixme_written_addresses: std::collections::HashSet<usize>,
}

fn to_u8_slice(x: u32) -> [u8; 4] {
    [
        ((x >> 24) & 0xFF) as u8,
        ((x >> 16) & 0xFF) as u8,
        ((x >> 8) & 0xFF) as u8,
        ((x >> 0) & 0xFF) as u8,
    ]
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
        // AaBb
        Mirroring::Horizontal => addr & 0xBFF,

        // ABab
        Mirroring::Vertical => addr & 0x7FF,
    }
}

/// Convert the low and the high byte to the corresponding indices from [0,3]
fn tile_lohi_to_idx(low: u8, high: u8) -> [u8; 8] {
    let mut color_idx = [0_u8; 8];
    for i in 0..color_idx.len() as u8 {
        color_idx[i as usize] = ((low >> (7 - i)) & 1) + (((high >> (7 - i)) & 1) << 1);
    }

    color_idx
}

const NAMETABLE_START: u16 = 0x2F00;
impl PPU {
    pub fn new(cartridge: Cartridge, renderer: Box<dyn Renderer>) -> Self {
        let cartridge_header = cartridge.header();
        PPU {
            frame_buf: [0_u8; FRAME_SIZE_BYTES],
            cartridge,
            cartridge_header,
            registers: Registers::default(),
            flags: Flags::default(),
            renderer,
            oam_primary: [0; 256],
            oam_secondary: [Sprite::default(); 8],
            palette_table: [0; 32],
            cycle: 0,
            scanline: -1,
            frame: 0,
            vram: RAM::with_size(0x3000),
            fixme_written_addresses: std::collections::HashSet::new(),
        }
    }

    pub fn set_renderer(&mut self, renderer: Box<dyn Renderer>) {
        self.renderer = renderer;
    }

    // FIXME: These implement some special behavior after beiing accessed
    pub fn register_read(&mut self, addr: u16) -> u8 {
        let ret = match (addr - 0x2000) % 8 {
            0 => self.registers.ctrl,
            1 => self.registers.mask,
            2 => {
                self.registers.scroll.reset_addr();
                self.registers.status
            }
            3 => self.registers.oamaddr,
            4 => self.registers.oamdata,
            5 => panic!("Cannot read PPU scroll!"),
            6 => panic!("Cannot read PPU address!"),
            7 => {
                let addr: u16 = self.registers.addr.into();
                self.ppu_addr_next();
                self.vram_read(addr)
            }
            _ => panic!("Invalid PPU Register read: address {:X}", addr),
        };

        event!(
            Level::DEBUG,
            "ppu::register_read [{:#X}] (== {:#X})",
            addr,
            ret
        );

        ret
    }

    pub fn oam_dma(&mut self, data: &[u8]) {
        assert_eq!(data.len(), 256, "Data should be 1 full page");
        self.oam_primary.as_mut_slice().copy_from_slice(data);
    }

    pub fn register_write(&mut self, addr: u16, val: u8) {
        event!(
            Level::DEBUG,
            "ppu::register_write [{:#X}] = {:#X}",
            addr,
            val
        );

        match (addr - 0x2000) % 8 {
            // FIXME: After power/reset, writes to this register are ignored for about 30,000
            // cycles. Do we actually need to respect this?
            0 => self.registers.ctrl = val,
            1 => self.registers.mask = val,
            2 => self.registers.status = val,
            3 => self.registers.oamaddr = val,
            4 => {
                // For emulation purposes, it is probably best to completely ignore writes during
                // rendering.
                //
                // https://www.nesdev.org/wiki/PPU_registers#OAMDATA
                if self.scanline >= VISIBLE_SCANLINES {
                    self.registers.oamdata = val;
                    self.registers.oamaddr += 1;
                }
            }
            5 => self.registers.scroll.write(val),
            6 => self.registers.addr.write(val),
            7 => {
                let addr: u16 = self.registers.addr.into();
                self.ppu_addr_next();
                self.vram_write(addr, val);
            }
            _ => panic!("Invalid PPU Register write: address {:#X}", addr),
        }
    }

    fn vram_read(&self, addr: u16) -> u8 {
        self.vram
            .read(mirror(self.cartridge_header.get_mirroring(), addr))
    }

    fn vram_write(&mut self, addr: u16, val: u8) {
        self.vram
            .write(mirror(self.cartridge_header.get_mirroring(), addr), val)
    }

    fn ppu_addr_next(&mut self) {
        let amt = if self.registers.ctrl & PpuCtrl::VRAM_INCR != 0 {
            32
        } else {
            1
        };
        self.registers.addr.incr(amt);
    }

    fn pattern_table_read(&self, addr: u16) -> u8 {
        assert!(addr < 0x1FFF);
        self.cartridge.chr_read(addr)
    }

    fn show_clipped_lhs(&self) -> bool {
        self.registers.mask & (PpuMask::SHOW_LEFT_BG | PpuMask::SHOW_LEFT_SPRITES) != 0
            && self.oam_secondary[0].x() <= 7
    }

    fn sprite0_past_rhs(&self) -> bool {
        self.oam_secondary[0].x() == 255
    }

    fn sprites_enabled(&self) -> bool {
        self.registers.mask & PpuMask::SHOW_SPRITES != 0
    }

    fn has_sprite0_hit(&self) -> bool {
        self.registers.status & PpuStatus::SPRITE_0_HIT != 0
    }

    fn update_sprite0_hit(&mut self) {
        if self.sprites_enabled()
            && self.show_clipped_lhs()
            && !self.sprite0_past_rhs()
            && !self.has_sprite0_hit()
        {
            self.registers.status |= PpuStatus::SPRITE_0_HIT;
        }
    }

    fn rendering_enabled(&self) -> bool {
        (self.registers.mask & (PpuMask::SHOW_SPRITES | PpuMask::SHOW_BG)) != 0
    }

    fn do_start_vblank(&mut self) {
        self.registers.status &= !PpuStatus::SPRITE_0_HIT;
        self.registers.status |= PpuStatus::VBLANK_STARTED;
        if self.registers.ctrl & PpuCtrl::NMI_ENABLE != 0 {
            // NMI is generated only on the start of the VBLANK cycle
            self.flags.has_nmi = true;
        }
    }

    fn do_end_frame(&mut self) {
        event!(
            Level::INFO,
            "END FRAME {}> PPUSTATUS = {:#02X}",
            self.frame,
            self.registers.status
        );

        self.frame += 1;
        self.flags.has_nmi = false;
        self.registers.status &= !PpuStatus::SPRITE_0_HIT;
        self.registers.status &= !PpuStatus::VBLANK_STARTED;
        self.registers.scroll.update_y_latch();
        self.flags.odd = !self.flags.odd;
        self.oam_secondary = [Sprite::default(); 8];
        self.fixme_written_addresses.clear();

        if self.rendering_enabled() {
            // FIXME: This should be done on a line basis in do_visible_scanline
            self.render_frame();
        }
        // self.show_pattern_table();

        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    fn clear_render_buffer(&mut self) {
        for i in 0..self.frame_buf.len() {
            self.frame_buf[i] = 0;
        }
    }

    #[tracing::instrument(target = "ppu", skip(self))]
    pub fn clock(&mut self, ticks: i16) {
        self.cycle += ticks;
        if self.cycle < CYCLES_PER_SCANLINE {
            return;
        }

        // FIXME: Do I need to implement this behavior? SW could read this register (apparently
        // Micro Machines does this)
        //
        //  * Cycles 1-64: Secondary OAM (32-byte buffer for current sprites on scanline) is
        //    initialized to $FF - attempting to read $2004 will return $FF
        //  * Cycles 257-320: (the sprite tile loading interval) of the pre-render and visible
        //    scanlines. OAMADDR is set to 0 during each of ticks
        const NOP_SCANLINES: i16 = VISIBLE_SCANLINES + 1;
        match self.scanline {
            -1 => { /* dummy scanline */ }
            0 => {
                self.clear_render_buffer();
                self.registers.status &= !PpuStatus::VBLANK_STARTED;
                self.do_visible_scanline();
            }
            1..VISIBLE_SCANLINES => self.do_visible_scanline(),
            VISIBLE_SCANLINES => self.do_start_vblank(),
            NOP_SCANLINES..SCANLINES_PER_FRAME => {}
            SCANLINES_PER_FRAME => self.do_end_frame(),
            _ => unreachable!(),
        }

        self.cycle -= CYCLES_PER_SCANLINE;
        self.scanline += 1;
        if self.scanline > SCANLINES_PER_FRAME {
            self.scanline = -1;
        }
    }

    fn nametable_base(&self) -> u16 {
        match self.registers.ctrl & PpuCtrl::NAMETABLE_ADDR {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => unreachable!(),
        }
    }

    fn bg_table_base(&self) -> u16 {
        match (self.registers.ctrl & PpuCtrl::BG_TABLE_ADDR) == 0 {
            true => 0x0000,
            false => 0x1000,
        }
    }

    fn sprite_table_base(&self) -> u16 {
        match self.registers.ctrl & PpuCtrl::SPRITE_TABLE_ADDR == 0 {
            true => 0x0000,
            false => 0x1000,
        }
    }

    fn read_attr_table(&self, v: u16) -> u8 {
        assert!(0x2000 <= v && v < 0x4000, "address {:#X} out of range", v);
        // 120 attribute table is a 64-byte array at the end of each nametable that controls which
        // palette is assigned to each part of the background.
        //
        // Each attribute table, starting at $23C0, $27C0, $2BC0, or $2FC0, is arranged as an 8x8
        // byte array: https://wiki.nesdev.org/w/index.php?title=PPU_attribute_tables
        //
        // ,---+---+---+---.
        // |   |   |   |   |
        // + D1-D0 + D3-D2 +
        // |   |   |   |   |
        // +---+---+---+---+
        // |   |   |   |   |
        // + D5-D4 + D7-D6 +
        // |   |   |   |   |
        // `---+---+---+---'

        // Tile and attribute fetching
        // https://www.nesdev.org/wiki/PPU_scrolling
        let attr_addr = 0x23C0 | (v & 0x0C00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07);
        let attr_byte = self.vram_read(attr_addr);
        let shift = ((v >> 4) & 4) | (v & 2);
        ((attr_byte >> shift) & 0x3) << 2
    }

    /// Generate an NMI. One called, the flag will be reset to false
    pub fn generate_nmi(&mut self) -> bool {
        let nmi = self.flags.has_nmi;
        self.flags.has_nmi = false;
        nmi
    }

    /// Visible scanline, each of which the PPU:
    ///  1. Clears the list of sprites to draw
    ///  2. Chooses the first 8 sprites on the scanline to draw
    ///  3. Checks for anymore sprites on the scanline to set the sprite overflow flag
    ///  4. Actually draws the sprites
    ///
    /// https://www.nesdev.org/wiki/PPU_sprite_evaluation
    fn do_visible_scanline(&mut self) {
        // The value of OAMADDR when sprite evaluation starts at tick 65 of the visible scanlines
        // will determine where in OAM sprite evaluation starts, and hence which sprite gets
        // treated as sprite 0. The first OAM entry to be checked during sprite evaluation is the
        // one starting at OAM[OAMADDR].

        event!(Level::DEBUG, "FRAME {}> evaluating sprites", self.frame);
        let sprite_height =
            sprite::Sprite::pix_height(self.registers.ctrl & PpuCtrl::SPRITE_HEIGHT);

        let mut num_sprites = 0;

        // No more sprites will be found once the end of OAM is reached, effectively hiding any
        // sprites before OAM[OAMADDR].
        let mut m = 0;
        const SPRITE_STRIDE: usize = 4;
        for n in (self.registers.oamaddr as usize..256_usize).step_by(SPRITE_STRIDE) {
            if num_sprites == 8 {
                // HW bug -- once we have exactly 8 sprites this starts being incremented along
                // with n
                m = (m + 1) & 0x3;
            }

            let addr = (n + m) as usize & 0xFF;

            // If OAMADDR is unaligned and does not point to the y position (first byte) of an OAM
            // entry, then whatever it points to (tile index, attribute, or x coordinate) will be
            // reinterpreted as a y position, and the following bytes will be reinterpreted as well
            let sprite = Sprite::from(&self.oam_primary[addr..(addr + 4)]);

            assert!(sprite_height == 8);
            if !sprite.in_scanline(self.scanline, sprite_height) {
                // Not visible -- we don't care
                continue;
            }

            if num_sprites == 8 {
                // Too many sprites found (9) when we can only have 8. Set the sprite overflow flag
                // and bail
                self.registers.status |= PpuStatus::SPRITE_OVERFLOW;
                event!(Level::DEBUG, "Sprite overflow");
                break;
            }

            self.oam_secondary[num_sprites] = sprite;
            num_sprites += 1;
        }
        event!(
            Level::DEBUG,
            "scanline {}> found {} sprites",
            self.scanline,
            num_sprites,
        );

        self.write_scanline(num_sprites);
    }

    /// Renders a tile at a time, since that's what data we get intially. There are four reads
    /// required to render a full tile:
    ///  1. Name table Byte
    ///  2. Attribute table byte
    ///  3. Palette table tile low
    ///  4. Palette table tile high (+8 bytes)
    fn write_scanline(&mut self, num_sprites: usize) {
        assert!(
            0 <= self.scanline && self.scanline < VISIBLE_SCANLINES,
            "Should not render during non-visible scanline {}",
            self.scanline
        );
        assert!(num_sprites < 9, "We can only draw 8 sprites");

        // FIXME: probably don't need to do the work but this makes things easier to debug
        //
        // if !self.rendering_enabled() {
        //     event!(Level::INFO, "FRAME {}> rendering disabled", self.frame);
        //     return;
        // }

        // Sprite evaluation does not cause a hit -- only rendering
        //
        // https://www.nesdev.org/wiki/PPU_sprite_evaluation
        self.update_sprite0_hit();

        event!(Level::DEBUG, "FRAME {}> rendering background", self.frame);
        // Render background
        for tile_x in 0..(FRAME_WIDTH_TILES as usize) {
            let v = self.registers.addr.to_u16();
            let fine_y = (v >> 12) & 0x7;
            let nametable_addr: u16 = self.nametable_base() | (v & 0xFFF);
            let pat_table_addr =
                self.bg_table_base() | self.vram_read(nametable_addr) as u16 | fine_y;

            // https://www.nesdev.org/wiki/PPU_palettes
            let d4 = 0_u8; // Rendering background, choose background palette
            let d3_d2 = self.read_attr_table(nametable_addr);

            self.ppu_addr_next();

            const HIGH_OFFSET_BYTES: u16 = 8; // The next bitplane for this tile
            let pattern_lo = self.pattern_table_read(pat_table_addr);
            let pattern_hi = self.pattern_table_read(pat_table_addr + HIGH_OFFSET_BYTES);
            let color_idx = tile_lohi_to_idx(pattern_lo, pattern_hi);

            // We're drawing a scanline at a time. The tile_num here is the tile within the whole
            // frame. So for the first TILE_HEIGHT_PX scanlines we should have the same
            // tile_num
            let tile_y = self.scanline as usize / TILE_HEIGHT_PX as usize;
            let tile_row = self.scanline as usize % TILE_HEIGHT_PX as usize;

            let addr = ((tile_y * TILE_HEIGHT_PX as usize + tile_row) * FRAME_WIDTH_PX as usize)
                + tile_x * TILE_WIDTH_PX as usize;
            let inserted = self.fixme_written_addresses.insert(addr);
            assert!(
                inserted,
                "scanline={} tile_x={} tile_y={} addr {} used (frame={})",
                self.scanline, tile_x, tile_y, addr, self.frame
            );

            // 0 is transparent, filter these out
            for (px, &lo) in color_idx.iter().enumerate().filter(|(_, &idx)| idx > 0) {
                assert!(lo < 4);

                let buf_addr = PX_SIZE_BYTES * (addr + px);

                let color = PALETTE_TABLE[((d4 << 4) | (d3_d2 << 2) | lo) as usize];
                let render_slice = &mut self.frame_buf[buf_addr..(buf_addr + PX_SIZE_BYTES)];
                //assert!(render_slice.iter().all(|&p| p == 0));
                render_slice.copy_from_slice(&to_u8_slice(color));
            }
        }

        // Render sprites
        //
        //   NOTE: To handle overlapping, the sprite data that occurs first will overlap any other
        //   sprites after it. Therefore we iterate through the list of sprites to render in
        //   reverse.
        event!(Level::DEBUG, "FRAME {}> rendering sprites", self.frame);
        for (i, sprite) in self.oam_secondary[..num_sprites].iter().rev().enumerate() {
            event!(
                Level::DEBUG,
                "SCANLINE {}> rendering sprite {}",
                self.scanline,
                i
            );

            assert!(sprite.y() <= self.scanline as i32);
            let mut pat_table_addr = if self.registers.ctrl & PpuCtrl::SPRITE_HEIGHT != 0 {
                // 16 pixel height sprites
                (sprite.tile_num() as u16 & 1) << 12_u16 | (sprite.tile_num() as u16 & 0xFE)
            } else {
                self.sprite_table_base() | sprite.tile_num() as u16
            } | self.scanline as u16 - sprite.y() as u16;

            if self.scanline as i32 - sprite.y() > 7 {
                // Bottom half of the large sprite gets the next tile
                // pat_table_addr += 0x10;
                todo!("Need to render two tiles");
            }

            const HIGH_OFFSET_BYTES: u16 = 8; // The next bitplane for this tile
            let pattern_lo = self.pattern_table_read(pat_table_addr);
            let pattern_hi = self.pattern_table_read(pat_table_addr + HIGH_OFFSET_BYTES);
            let color_idx = tile_lohi_to_idx(pattern_lo, pattern_hi);

            for (px, &lo_idx) in color_idx.iter().enumerate().filter(|(_, &idx)| idx > 0) {
                assert!(lo_idx < 4);

                let buf_addr = PX_SIZE_BYTES
                    * ((self.scanline as usize) * (FRAME_WIDTH_PX as usize)
                        + sprite.x() as usize
                        + px as usize);

                let d4 = 1; // Rendering sprite from sprite table
                let idx = d4 << 4 | sprite.palette_num() | lo_idx;

                let color = PALETTE_TABLE[idx as usize];
                self.frame_buf[buf_addr..(buf_addr + PX_SIZE_BYTES)]
                    .copy_from_slice(&to_u8_slice(color));
            }
        }
    }

    fn render_frame(&mut self) {
        self.renderer.render_frame(
            &self.frame_buf,
            FRAME_WIDTH_PX as u32,
            FRAME_HEIGHT_PX as u32,
        );
    }

    fn show_pattern_table(&mut self) {
        let mut buf = vec![0_u8; FRAME_SIZE_BYTES / 2];

        let read_tile_lohi = |addr: u16| -> (u8, u8) {
            const HIGH_OFFSET_BYTES: u16 = 8;
            (
                self.cartridge.chr_read(addr),
                self.cartridge.chr_read(addr + HIGH_OFFSET_BYTES),
            )
        };

        // The pattern table has a tile adjacent in memory, while SDL renders entire rows. When
        // reading the pattern table we need to add an offset that is the tile number
        //
        // Concretely, the first row of the SDL texture contains the first row of 16 tiles, which
        // are actually offset 16 bytes from each other. Display the tiles side-by-side so we have
        // the traditional left and right halves

        // There are 16 x 32 tiles
        const NUM_TILES_VERT: u16 = 16;
        let mut used_addrs = [false; 0x2000];
        for row in 0..NUM_TILES_VERT * TILE_HEIGHT_PX {
            let (tile_y, tile_row) = (row / TILE_HEIGHT_PX, row % TILE_HEIGHT_PX);

            for tile_x in 0..FRAME_WIDTH_TILES {
                let tile_num = tile_y * FRAME_WIDTH_TILES + tile_x;
                let chr_addr = tile_row + tile_num * TILE_SIZE_BYTES;

                assert_eq!(used_addrs[chr_addr as usize], false);
                used_addrs[chr_addr as usize] = true;
                used_addrs[chr_addr as usize + 8] = true;

                let (low_byte, high_byte) = read_tile_lohi(chr_addr);
                let color_idx = tile_lohi_to_idx(low_byte, high_byte);

                for px in 0..TILE_WIDTH_PX {
                    const COLORS: [u8; 4] = [1, 85, 170, 255];
                    let color = COLORS[color_idx[px as usize] as usize];
                    let buf_addr = PX_SIZE_BYTES
                        * (px as usize
                            + (row * FRAME_WIDTH_TILES + tile_x) as usize * TILE_WIDTH_PX as usize);

                    // Assign all pixels as the same color value so we get a grayscale version
                    assert_eq!(
                        buf[buf_addr..(buf_addr + PX_SIZE_BYTES)],
                        [0; PX_SIZE_BYTES]
                    );
                    buf[buf_addr..(buf_addr + PX_SIZE_BYTES)]
                        .copy_from_slice(&[color; PX_SIZE_BYTES]);
                }
            }
        }
        for (addr, used) in used_addrs.iter().enumerate() {
            assert!(used, "Unused address {:#X}", addr);
        }

        // Format the pattern table s.t. 0x000-0x0FFF are on the left and 0x1000-0x1FFF are on the
        // right
        let half_frame: usize = buf.len() / 2;
        const HALF_TILES: usize = TILE_HEIGHT_PX as usize * FRAME_WIDTH_PX as usize * PX_SIZE_BYTES;
        let pattern_table = buf[..half_frame]
            .chunks(HALF_TILES)
            .zip(buf[half_frame..].chunks(HALF_TILES))
            .flat_map(|(l, r)| [l, r].concat())
            .collect::<Vec<_>>();
        assert_eq!(pattern_table.len(), buf.len());

        const TEX_WIDTH_PX: u32 = FRAME_WIDTH_PX as u32;
        const TEX_HEIGHT_PX: u32 = FRAME_HEIGHT_PX as u32 / 2;
        self.renderer
            .render_frame(&pattern_table, TEX_WIDTH_PX, TEX_HEIGHT_PX);
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

    #[test]
    fn lohi_to_index() {
        assert_eq!(
            tile_lohi_to_idx(0b11001100_u8, 0b11001100_u8),
            [3, 3, 0, 0, 3, 3, 0, 0]
        );
        assert_eq!(
            tile_lohi_to_idx(0b10001000_u8, 0b11001100_u8),
            [3, 2, 0, 0, 3, 2, 0, 0]
        );
    }
}
