mod flags;
mod registers;
mod sprite;

use crate::cartridge::header::{Header, Mirroring};
use crate::cartridge::{Cartridge, CartridgeInterface};
use crate::graphics::Renderer;
use crate::memory::{RAM, ROM};
use crate::timer;
use flags::*;
use registers::*;
use sprite::Sprite;
use std::collections::VecDeque;
use tracing::{event, Level};

// FIXME: Need to fix up these types a bit
const SCANLINES_PER_FRAME: usize = 260;
const VISIBLE_SCANLINES: usize = 240;
const CYCLES_PER_SCANLINE: usize = 341;
const VISIBLE_CYCLES: usize = 257;
const CYCLES_PER_TILE: usize = 8;
const STARTUP_SCANLINES: usize = 30_000 / CYCLES_PER_SCANLINE;

const PX_SIZE_BYTES: usize = 4; // 4th byte for the pixel is unused

const TILE_HI_OFFSET_BYTES: u16 = 8;
const TILE_STRIDE_SHIFT: u16 = 4;
const TILE_WIDTH_PX: usize = 8;
const TILE_HEIGHT_PX: usize = 8;
const TILE_SIZE_BYTES: usize = 16;

const FRAME_NUM_TILES: usize = FRAME_WIDTH_TILES * FRAME_HEIGHT_TILES;
const FRAME_WIDTH_TILES: usize = 32;
const FRAME_HEIGHT_TILES: usize = 30;
const FRAME_HEIGHT_PX: usize = FRAME_HEIGHT_TILES * TILE_HEIGHT_PX;
const FRAME_WIDTH_PX: usize = FRAME_WIDTH_TILES * TILE_WIDTH_PX;
const FRAME_SIZE_BYTES: usize =
    PX_SIZE_BYTES * (FRAME_HEIGHT_PX as usize) * (FRAME_WIDTH_PX as usize);

const PALETTE_COLOR_LUT: [u32; 64] = [
    0x7C7C7C, 0x0000FC, 0x0000BC, 0x4428BC, 0x940084, 0xA80020, 0xA81000, 0x881400, 0x503000,
    0x007800, 0x006800, 0x005800, 0x004058, 0x000000, 0x000000, 0x000000, 0xBCBCBC, 0x0078F8,
    0x0058F8, 0x6844FC, 0xD800CC, 0xE40058, 0xF83800, 0xE45C10, 0xAC7C00, 0x00B800, 0x00A800,
    0x00A844, 0x008888, 0x000000, 0x000000, 0x000000, 0xF8F8F8, 0x3CBCFC, 0x6888FC, 0x9878F8,
    0xF878F8, 0xF85898, 0xF87858, 0xFCA044, 0xF8B800, 0xB8F818, 0x58D854, 0x58F898, 0x00E8D8,
    0x787878, 0x000000, 0x000000, 0xFCFCFC, 0xA4E4FC, 0xB8B8F8, 0xD8B8F8, 0xF8B8F8, 0xF8A4C0,
    0xF0D0B0, 0xFCE0A8, 0xF8D878, 0xD8F878, 0xB8F8B8, 0xB8F8D8, 0x00FCFC, 0xF8D8F8, 0x000000,
    0x000000,
];

#[derive(Default)]
struct Tile {
    number: usize,
    nametable_byte: u8,
    attribute_byte: u8,
    pattern_lo: u8,
    pattern_hi: u8,
}

struct OamSecondary {
    sprites: Vec<Sprite>,
    has_sprite_0: bool,
}

impl Default for OamSecondary {
    fn default() -> Self {
        OamSecondary {
            sprites: Vec::new(),
            has_sprite_0: false,
        }
    }
}

pub struct PPU {
    frame_buf: Vec<u8>,

    cartridge_header: Header,
    cartridge_chr: ROM,

    registers: Registers,
    ppudata_buffer: u8,
    flags: Flags,
    vram: RAM,
    renderer: Box<dyn Renderer>,

    // Sprites
    oam_primary: [u8; 256], // Reinterpreted as sprites
    oam_secondary: OamSecondary,

    // Number of cycles the NES has simulated outside of the PPU. The PPU may lag behind or skip
    // frames entirely if the result of the frame is neither human nor software visible
    cycles_behind: usize,
    ppu_cycle: i16,
    scanline: i16,
    frame: usize,

    // Background
    tile_q: VecDeque<Tile>,

    palette_table: [u8; 32],
}

const WHITE: [u8; 4] = [0xff; 4];
const BLACK: [u8; 4] = [0x00; 4];

fn to_u8_slice(x: u32) -> [u8; 4] {
    [
        ((x >> 0) & 0xFF) as u8,
        ((x >> 8) & 0xFF) as u8,
        ((x >> 16) & 0xFF) as u8,
        ((x >> 24) & 0xFF) as u8,
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
fn mirror(mirror: &Mirroring, addr: u16) -> usize {
    let addr = addr as usize;
    (addr & !0xFFF)
        | match mirror {
            // AaBb
            Mirroring::Horizontal => addr & 0xBFF,

            // ABab
            Mirroring::Vertical => addr & 0x7FF,
        }
}

/// Convert the low and the high byte to the corresponding indices from [0,3]
fn tile_lohi_to_idx(low: u8, high: u8) -> [u8; 8] {
    let mut color_idx = [0_u8; 8];
    for i in (0..color_idx.len()).rev() {
        color_idx[(7 - i) as usize] = ((low >> i) & 1) | (((high >> i) & 1) << 1);
    }

    color_idx
}

const PPU_VRAM_SIZE: usize = 0x2000;
impl PPU {
    pub fn new(cartridge: &Cartridge, renderer: Box<dyn Renderer>) -> Self {
        let cartridge_header = cartridge.header();
        let cartridge_chr = cartridge.chr();

        let mut tile_q = VecDeque::with_capacity(3);
        tile_q.push_back(Tile::default());
        tile_q.push_back(Tile::default());
        tile_q.push_back(Tile::default());

        PPU {
            frame_buf: vec![0_u8; FRAME_SIZE_BYTES],
            cartridge_chr,
            cartridge_header,
            palette_table: [0; 32],
            registers: Registers::default(),
            flags: Flags::default(),
            renderer,
            oam_primary: [0; 256],
            oam_secondary: OamSecondary::default(),

            cycles_behind: 0,
            ppu_cycle: 0,
            scanline: -1,
            frame: 0,
            tile_q,
            ppudata_buffer: 0,
            vram: RAM::with_size(PPU_VRAM_SIZE),
        }
    }

    pub fn cycle(&self) -> i16 {
        self.ppu_cycle
    }

    pub fn scanline(&self) -> i16 {
        self.scanline
    }

    pub fn register_read(&mut self, addr: u16) -> u8 {
        let ret = match addr % 8 {
            0 => self.registers.ctrl,
            1 => self.registers.mask,
            2 => {
                self.tick_n();

                self.registers.addr.reset();

                let val = self.registers.status;
                self.registers.status &= !PpuStatus::VBLANK_STARTED;
                val
            }
            3 => self.registers.oamaddr,
            4 => self.registers.oamdata,
            5 => {
                event!(Level::DEBUG, "garbage read from PPUSCROLL");
                0x0
            }
            6 => {
                event!(Level::DEBUG, "garbage read from PPUADDR");
                0x0
            }
            7 => {
                self.tick_n();

                let addr = self.registers.addr.to_u16();
                self.ppudata_addr_incr();

                let mut val = self.ppu_internal_read(addr);
                // Access to all memory except the palettes will return the contents of the
                // internal buffer. However the content of the buffer is the content of the
                // nametable "underneath" the palette table if the palette is read. This buffer is
                // only updated on reads of PPUDATA
                if addr < 0x3F00 {
                    std::mem::swap(&mut self.ppudata_buffer, &mut val);
                } else {
                    self.ppudata_buffer = self.ppu_internal_read(addr & 0x2FFF);
                }
                val
            }
            _ => unreachable!(),
        };

        event!(
            Level::DEBUG,
            "[CYC:{}][SL:{}] ppu::register_read [{:#x}] (== {:#x})",
            self.ppu_cycle,
            self.scanline,
            addr,
            ret
        );

        ret
    }

    pub fn register_write(&mut self, addr: u16, val: u8) {
        let regnum = addr % 8;
        if regnum == 7 {
            event!(
                Level::DEBUG,
                "[CYC:{}][SL:{}] ppu::register_write [{:#x}] VRAM({:#x}) = {:#x}",
                self.ppu_cycle,
                self.scanline,
                addr,
                self.registers.addr.to_u16(),
                val
            );
        } else {
            event!(
                Level::DEBUG,
                "[CYC:{}][SL:{}] ppu::register_write [{:#x}] = {:#x}",
                self.ppu_cycle,
                self.scanline,
                addr,
                val
            );
        }

        match regnum {
            0 => {
                self.tick_n();

                self.registers.ctrl = val;
                self.registers.addr.set_nametable(val);
            }
            1 => self.registers.mask = val,
            2 => self.registers.status = val,
            3 => self.registers.oamaddr = val,
            4 => {
                // For emulation purposes, it is probably best to completely ignore writes during
                // rendering (but the address is still updated)
                //
                // https://www.nesdev.org/wiki/PPU_registers#OAMDATA
                if self.scanline >= VISIBLE_SCANLINES as i16 {
                    self.registers.oamdata = val;
                }
                self.registers.oamaddr = self.registers.oamaddr.wrapping_add(1);
            }
            5 => {
                self.tick_n();

                self.registers.addr.scroll_write(val);
            }
            6 => {
                self.tick_n();

                self.registers.addr.addr_write(val);
            }
            7 => {
                self.tick_n();

                let addr = self.registers.addr.to_u16();
                self.ppudata_addr_incr();
                self.ppu_internal_write(addr, val);
            }
            _ => unreachable!(),
        }
    }

    pub fn oam_dma(&mut self, data: &[u8]) {
        assert_eq!(data.len(), 256, "Data should be 1 full page");
        self.oam_primary.as_mut_slice().copy_from_slice(data);
    }

    // https://www.nesdev.org/wiki/PPU_memory_map
    fn ppu_internal_write(&mut self, addr: u16, val: u8) {
        match addr {
            // Pattern tables 0 and 1
            0..=0x1FFF => {
                // Ignore writes to CHR
                event!(
                    Level::DEBUG,
                    "ignoring write to CHR ROM at {:#x} of {:#x}",
                    addr,
                    val,
                );
            }

            // Nametables
            0x2000..=0x3EFF => {
                let vram_offset =
                    mirror(self.cartridge_header.get_mirroring(), addr) - PPU_VRAM_SIZE;
                self.vram[vram_offset] = val;
            }

            // $3F00-$3F1F: Palette RAM
            0x3F00..=0x3FFF => self.palette_write(addr - 0x3F00, val),
            _ => unreachable!("Out of bounds: {:#x}", addr),
        }
    }

    // https://www.nesdev.org/wiki/PPU_memory_map
    fn ppu_internal_read(&mut self, addr: u16) -> u8 {
        match addr {
            // Pattern tables 0 and 1
            0..=0x1FFF => self.cartridge_chr[addr as usize],

            // Nametables
            0x2000..=0x3EFF => {
                let vram_offset =
                    mirror(self.cartridge_header.get_mirroring(), addr) - PPU_VRAM_SIZE;
                self.vram[vram_offset]
            }

            // $3F00-$3F1F: Palette RAM
            0x3F00..=0x3FFF => self.palette_read(addr - 0x3F00),
            _ => unreachable!("Out of bounds: {:#x}", addr),
        }
    }

    fn ppudata_addr_incr(&mut self) {
        let amt = if (self.registers.ctrl & PpuCtrl::VRAM_INCR) != 0 {
            32
        } else {
            1
        };
        self.registers.addr.incr(amt);
    }

    fn show_clipped_lhs(&self) -> bool {
        self.registers.mask & (PpuMask::SHOW_LEFT_BG | PpuMask::SHOW_LEFT_SPRITES) != 0
            && self.oam_secondary.has_sprite_0
            && self.oam_secondary.sprites[0].x() <= 7
    }

    fn sprite0_past_rhs(&self) -> bool {
        self.oam_secondary.has_sprite_0 && self.oam_secondary.sprites[0].x() == 255
    }

    fn background_enabled(&self) -> bool {
        self.registers.mask & PpuMask::SHOW_BG != 0
    }

    fn sprites_enabled(&self) -> bool {
        self.registers.mask & PpuMask::SHOW_SPRITES != 0
    }

    fn has_sprite0_hit(&self) -> bool {
        self.registers.status & PpuStatus::SPRITE_0_HIT != 0
    }

    fn rendering_enabled(&self) -> bool {
        (self.registers.mask & (PpuMask::SHOW_SPRITES | PpuMask::SHOW_BG)) != 0
    }

    fn total_ppu_cycles(&self) -> usize {
        (1 + self.scanline) as usize * CYCLES_PER_SCANLINE
            + self.ppu_cycle as usize
            + self.cycles_behind
    }

    fn do_start_vblank(&mut self) {
        event!(
            Level::DEBUG,
            "[CYC:{:<3}][SL:{:<3}] VBI",
            self.ppu_cycle,
            self.scanline,
        );

        self.registers.status &= !PpuStatus::SPRITE_0_HIT;
        self.registers.status |= PpuStatus::VBLANK_STARTED;
        if self.registers.ctrl & PpuCtrl::NMI_ENABLE != 0 {
            // NMI is generated only on the start of the VBLANK cycle
            self.flags.has_nmi = true;
        }
    }

    fn do_start_frame(&mut self) {
        self.registers.status &= !PpuStatus::SPRITE_0_HIT;
        self.registers.status &= !PpuStatus::VBLANK_STARTED;
        self.registers.status &= !PpuStatus::SPRITE_OVERFLOW;
    }

    fn do_end_frame(&mut self) {
        self.scanline = -1;
        self.frame += 1;
        self.flags.has_nmi = false;
        self.flags.odd = !self.flags.odd;

        // FIXME: Would be cool to make these options that could be passed at startup, and updated
        // during runtime
        // self.show_nametable();
        // self.show_pattern_table();
        if self.rendering_enabled() {
            // FIXME: Maybe this should be done on a line basis
            self.render_frame();
        }

        // FIXME: add extra checks mode
        // self.clear_render_buffer();
    }

    fn is_blanking(&self) -> bool {
        // SW can set forced-blank mode, which disables all rendering and updates. This is used
        // typically during initialization
        let forced_blank = !self.rendering_enabled();
        let in_vblank = self.scanline > VISIBLE_SCANLINES as i16;
        forced_blank || in_vblank
    }

    fn clear_render_buffer(&mut self) {
        for i in 0..self.frame_buf.len() {
            self.frame_buf[i] = 0;
        }
    }

    fn back_tile_mut(&mut self) -> &mut Tile {
        assert!(self.tile_q.len() == 3);
        self.tile_q
            .back_mut()
            .expect("tile_q should always be of size 3")
    }

    fn back_tile(&self) -> &Tile {
        assert!(self.tile_q.len() == 3);
        self.tile_q
            .back()
            .expect("tile_q should always be of size 3")
    }

    fn front_tile(&self) -> &Tile {
        assert!(self.tile_q.len() == 3);
        self.tile_q
            .front()
            .expect("tile_q should always be of size 3")
    }

    fn do_nametable_fetch(&mut self) {
        // Upper bits are the fine_y scrolling
        let tile_addr = self.registers.addr.to_u16() & 0xFFF;

        self.back_tile_mut().number = (tile_addr % 960) as usize;
        self.back_tile_mut().nametable_byte = self.ppu_internal_read(0x2000 | tile_addr);
    }

    fn do_attribute_fetch(&mut self) {
        let v = self.registers.addr.to_u16();
        let attribute_addr = 0x23C0 | (v & 0x0C00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07);
        let attribute_byte = self.ppu_internal_read(attribute_addr);
        self.back_tile_mut().attribute_byte = attribute_byte;
    }

    fn do_pattern_fetch(&mut self) {
        let v = self.registers.addr.to_u16();
        let fine_y = (v >> 12) & 0x7;

        let tile_base = self.bg_table_base()
            | ((self.back_tile_mut().nametable_byte as u16) << TILE_STRIDE_SHIFT);

        let pattable_addr = tile_base | fine_y;
        self.back_tile_mut().pattern_lo = self.ppu_internal_read(pattable_addr);
        self.back_tile_mut().pattern_hi =
            self.ppu_internal_read(pattable_addr + TILE_HI_OFFSET_BYTES);
    }

    fn do_prepare_next_tile(&mut self) {
        assert!(!self.is_blanking());

        event!(
            Level::DEBUG,
            "[CYC:{:<3}][SL:{:<3}] TILE:{:X} V({:#<04X}): (NT={:0X}, ATTR={:0X}, LO={:0X}, HI={:0X})",
            self.ppu_cycle,
            self.scanline,
            self.registers.addr.to_u16(),
            self.back_tile().number,
            self.back_tile().nametable_byte,
            self.back_tile().attribute_byte,
            self.back_tile().pattern_lo,
            self.back_tile().pattern_hi,
        );

        self.tile_q.rotate_left(1);
    }

    pub fn sprite_hit_next_scanline(&self, sprite: &Sprite) -> bool {
        // NOTE: sprites on the first scanline are never rendered
        let next_scanline = self.scanline + 1;
        if next_scanline == VISIBLE_SCANLINES as i16 {
            return false;
        }

        let sprite_height = if (self.registers.ctrl & PpuCtrl::SPRITE_HEIGHT) != 0 {
            16
        } else {
            8
        };

        sprite.y() as i16 <= next_scanline && next_scanline < (sprite.y() + sprite_height) as i16
    }

    fn update_vaddr(&mut self) {
        let prev_addr = self.registers.addr.to_u16();
        match self.ppu_cycle {
            0 => { /* skipped */ }
            8 | 16 | 24 | 32 | 40 | 48 | 56 | 64 | 72 | 80 | 88 | 96 | 104 | 112 | 120 | 128
            | 136 | 144 | 152 | 160 | 168 | 176 | 184 | 192 | 200 | 208 | 216 | 224 | 232 | 240
            | 248 | 328 | 336 => self.registers.addr.incr_x(),
            256 => {
                self.registers.addr.incr_y();
                self.registers.addr.incr_x();
            }
            257 => self.registers.addr.sync_x(),
            280..=304 => {
                if self.scanline == -1 {
                    self.registers.addr.sync_y();
                }
            }
            _ => {}
        }

        if prev_addr != self.registers.addr.to_u16() {
            event!(
                Level::DEBUG,
                "[CYC:{:<3}][SL:{:<3}] V({:#X}) ==> V({:#X})",
                self.ppu_cycle,
                self.scanline,
                prev_addr,
                self.registers.addr.to_u16(),
            );
        }
    }

    fn do_tile_fetches(&mut self) {
        if self.ppu_cycle == 0 {
            return;
        }

        if 257 <= self.ppu_cycle && self.ppu_cycle < 320 {
            // No fetches happen here, this is where sprites are fetched (we don't yet though)
            return;
        }

        if self.ppu_cycle > 336 {
            // FIXME: Fetches after this cycle are special and fetch two garbage nametables. Might
            // not need to implement this
            // https://www.nesdev.org/wiki/PPU_rendering#Pre-render_scanline_(-1_or_261)
            return;
        }

        // FIXME: A possible performance improvement would be to pre-allocate a scanline-size buffer
        // for tiles where we could then render all at once. We could potentially do the tile fetches
        // in one-shot as well. Would need to validate that this works with scrolling though before
        // changing it
        if (self.ppu_cycle - 1) % 8 == 0 {
            timer::timed!("ppu::tile fetch", {
                self.do_prepare_next_tile();
                self.do_nametable_fetch();
                self.do_attribute_fetch();
                self.do_pattern_fetch();
            });
        }
    }

    fn tick_once(&mut self) {
        if !self.is_blanking() {
            self.do_tile_fetches();
        }

        // https://www.nesdev.org/wiki/PPU_rendering
        match (self.scanline, self.ppu_cycle) {
            (-1, 1) => timer::timed!("ppu::clear flags", { self.do_start_frame() }),
            (-1, _) => { /* dummy scanline */ }

            // Visible scanlines (0-239)
            (0..240, 0) => { /* idle cycle */ }
            (0..240, (1..256)) => {
                if ((self.ppu_cycle - 1) % TILE_WIDTH_PX as i16) == 0 {
                    // Render one tile at a time. This is how frequently the real hardware is
                    // updated. A possible cycle-accurate improvement would be to do this fetch
                    // every 8 cycles but write the pixels every cycle. Not sure if we actually
                    // need to do this to get a workable game.
                    timer::timed!("ppu::draw background", { self.draw_background() });
                }
            }
            // Draw sprites once on the last visible cycle so they're over the background
            (0..240, 256) => timer::timed!("ppu::draw sprites", { self.draw_sprites() }),
            (0..240, 257..321) => {
                timer::timed!("ppu::evaluate sprites", {
                    self.evaluate_sprites_next_scanline()
                })
            }
            (0..240, 321..342) => { /* garbage nametable fetches */ }

            (240, _) => { /* idle scanline */ }

            (241, 1) => timer::timed!("ppu::vblank", { self.do_start_vblank() }),
            (241..261, _) => { /* PPU should make no memory accesses */ }

            (scanline, cycle) => unreachable!("scanline={}, cycle={}", scanline, cycle),
        }

        if !self.is_blanking() {
            self.update_vaddr();
        }

        self.ppu_cycle += 1;

        // End this scanline
        if self.ppu_cycle >= CYCLES_PER_SCANLINE as i16 {
            self.ppu_cycle -= CYCLES_PER_SCANLINE as i16;
            self.scanline += 1;
            if self.scanline > SCANLINES_PER_FRAME as i16 {
                timer::timed!("ppu::EOF", {
                    self.do_end_frame();
                });
            }
        }
    }

    #[tracing::instrument(target = "ppu", skip(self))]
    pub fn clock(&mut self, ticks: usize) {
        self.cycles_behind += ticks;

        let total_cycles = self.total_ppu_cycles();

        const VBLANK_START_SL: usize = 241;
        const VBLANK_START: usize = VBLANK_START_SL * CYCLES_PER_SCANLINE + 1;
        const VBLANK_END: usize = SCANLINES_PER_FRAME * CYCLES_PER_SCANLINE;

        if self.registers.ctrl & PpuCtrl::NMI_ENABLE != 0 && total_cycles >= VBLANK_START {
            self.tick_n();
        } else if total_cycles >= VBLANK_END {
            self.do_end_frame();
            self.ppu_cycle = 0;
            self.cycles_behind = total_cycles - VBLANK_END;
        }
    }

    fn tick_n(&mut self) {
        for _ in 0..self.cycles_behind {
            self.tick_once();
        }

        self.cycles_behind = 0;
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

    /// Generate an NMI. One called, the flag will be reset to false
    pub fn generate_nmi(&mut self) -> bool {
        let nmi = self.flags.has_nmi;
        self.flags.has_nmi = false;
        nmi
    }

    fn palette_read(&mut self, addr: u16) -> u8 {
        assert!(addr <= 0xFF);
        let mut addr = addr & 0x1F;

        // Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C
        if addr % 4 == 0 {
            addr &= !0x10;
        }

        // $3F20-$3FFF: mirrors of palette RAM
        self.palette_table[addr as usize] & 0x3F
    }

    fn palette_write(&mut self, mut addr: u16, val: u8) {
        assert!(addr <= 0xFF);

        // Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C
        if addr % 4 == 0 {
            addr &= !0x10;
        }
        // $3F20-$3FFF: mirrors of palette RAM
        self.palette_table[(addr & 0x1F) as usize] = val;
    }

    fn is_visible_cycle(&self) -> bool {
        0 <= self.scanline
            && self.scanline < VISIBLE_SCANLINES as i16
            && self.ppu_cycle < VISIBLE_CYCLES as i16
    }

    /// Compute the rendering base address into the buffer to render at the current scanline at the
    /// specified x-coordinate. Should only be called during a visible cycle and scanline
    fn render_base_address(&self, x: usize) -> usize {
        assert!(self.is_visible_cycle());

        let tile_y = self.scanline as usize / TILE_HEIGHT_PX;
        let tile_row = self.scanline as usize % TILE_HEIGHT_PX;

        ((tile_y * TILE_HEIGHT_PX as usize + tile_row) * FRAME_WIDTH_TILES as usize)
            * TILE_WIDTH_PX as usize
            + x
    }

    fn draw_background(&mut self) {
        assert!(self.is_visible_cycle());

        if !self.background_enabled() {
            return;
        }

        let Tile {
            number: tile_number,
            nametable_byte: _,
            attribute_byte,
            pattern_lo,
            pattern_hi,
        } = self.front_tile();

        // https://www.nesdev.org/wiki/PPU_palettes
        let d4 = 0_u8; // Rendering background, choose background palette

        // 120 attribute table is a 64-byte array at the end of each nametable that controls which
        // palette is assigned to each part of the background.
        //
        // Each attribute table, starting at $23C0, $27C0, $2BC0, or $2FC0, is arranged as an 8x8
        // byte array: https://wiki.nesdev.org/w/index.php?title=PPU_attribute_tables
        //
        //        0       1
        //    ,---+---+---+---.
        //    |   |   |   |   |
        //  0 + D1-D0 + D3-D2 +
        //    |   |   |   |   |
        //    +---+---+---+---+
        //    |   |   |   |   |
        //  1 + D5-D4 + D7-D6 +
        //    |   |   |   |   |
        //    `---+---+---+---'

        // Tile and attribute fetching
        // https://www.nesdev.org/wiki/PPU_scrolling
        let tile_attr_x = tile_number % FRAME_WIDTH_TILES;
        let tile_attr_y = tile_number / FRAME_WIDTH_TILES;
        let d3_d2 = match ((tile_attr_x % 4) / 2, (tile_attr_y % 4) / 2) {
            (0, 0) => (attribute_byte >> 0) & 0x3,
            (1, 0) => (attribute_byte >> 2) & 0x3,
            (0, 1) => (attribute_byte >> 4) & 0x3,
            (1, 1) => (attribute_byte >> 6) & 0x3,
            _ => unreachable!(),
        };

        // Rendering the background shouldbe tile-aligned
        let x = (self.ppu_cycle - 1) as usize;
        assert!((x % TILE_WIDTH_PX) == 0);
        let base_addr = self.render_base_address(x);

        // 0 is transparent, filter these out
        let color_idx = tile_lohi_to_idx(*pattern_lo, *pattern_hi);
        for (px, &lo) in color_idx.iter().enumerate() {
            assert!(lo < 4);

            let palette_addr = (d4 << 4) | (d3_d2 << 2) | lo;
            let color_idx = self.palette_read(palette_addr as u16);
            let color = PALETTE_COLOR_LUT[color_idx as usize];

            let buf_addr = PX_SIZE_BYTES * (base_addr + px);
            let render_slice = &mut self.frame_buf[buf_addr..(buf_addr + PX_SIZE_BYTES)];

            // FIXME: add extra checks mode
            // assert!(render_slice.iter().all(|&p| p == 0));
            render_slice.copy_from_slice(&to_u8_slice(color));
        }
    }

    fn show_nametable(&mut self) {
        let mut buf = vec![0_u8; FRAME_SIZE_BYTES];

        const NAMETABLE_BASE: u16 = 0x2000;
        for v in 0..FRAME_NUM_TILES {
            let nt_addr = NAMETABLE_BASE | (v as u16 & 0xFFF);
            let nt_byte = self.ppu_internal_read(nt_addr) as u16;

            const TILE_STRIDE_SHIFT: u16 = 4;
            let tile_base = self.bg_table_base() | (nt_byte << TILE_STRIDE_SHIFT);

            let tile_x = v % FRAME_WIDTH_TILES;
            let tile_y = v / FRAME_WIDTH_TILES;
            let attribute_addr = 0x23C0 | (v & 0x0C00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07);
            let attribute_byte = self.ppu_internal_read(attribute_addr as u16);

            let d3_d2 = match ((tile_x % 4) / 2, (tile_y % 4) / 2) {
                (0, 0) => (attribute_byte >> 0) & 0x3,
                (1, 0) => (attribute_byte >> 2) & 0x3,
                (0, 1) => (attribute_byte >> 4) & 0x3,
                (1, 1) => (attribute_byte >> 6) & 0x3,
                _ => unreachable!(),
            };

            for tile_row in 0..8_usize {
                let pattable_addr = tile_base | tile_row as u16;
                const HIGH_OFFSET_BYTES: u16 = 8; // The next bitplane for this tile
                let pattern_lo = self.ppu_internal_read(pattable_addr);
                let pattern_hi = self.ppu_internal_read(pattable_addr + HIGH_OFFSET_BYTES);

                let base_addr = (((tile_y * TILE_HEIGHT_PX) + tile_row) * FRAME_WIDTH_TILES
                    + tile_x)
                    * TILE_WIDTH_PX;
                let base_addr_px = base_addr;

                let color_idx = tile_lohi_to_idx(pattern_lo, pattern_hi);
                for (px, &lo) in color_idx.iter().enumerate() {
                    assert!(lo < 4);

                    let palette_addr = (d3_d2 << 2) | lo;
                    let color_idx = self.palette_read(palette_addr as u16);
                    let color = PALETTE_COLOR_LUT[color_idx as usize];

                    let buf_addr = PX_SIZE_BYTES * (base_addr_px + px);
                    let render_slice = &mut buf[buf_addr..(buf_addr + PX_SIZE_BYTES)];

                    assert!(render_slice.iter().all(|&p| p == 0));
                    render_slice.copy_from_slice(&to_u8_slice(color));
                }
            }
        }

        self.renderer
            .render_frame(&buf, FRAME_WIDTH_PX as u32, FRAME_HEIGHT_PX as u32);
    }

    fn show_pattern_table(&mut self) {
        let mut buf = vec![0_u8; FRAME_SIZE_BYTES / 2];

        let read_tile_lohi = |addr: u16| -> (u8, u8) {
            const HIGH_OFFSET_BYTES: usize = 8;
            (
                self.cartridge_chr[addr as usize],
                self.cartridge_chr[addr as usize + HIGH_OFFSET_BYTES],
            )
        };

        // The pattern table has a tile adjacent in memory, while SDL renders entire rows. When
        // reading the pattern table we need to add an offset that is the tile number
        //
        // Concretely, the first row of the SDL texture contains the first row of 16 tiles, which
        // are actually offset 16 bytes from each other. Display the tiles side-by-side so we have
        // the traditional left and right halves

        // There are 16 x 32 tiles
        const NUM_TILES_VERT: usize = 16;
        let mut used_addrs = [false; 0x2000];
        for row in 0..NUM_TILES_VERT * TILE_HEIGHT_PX {
            let (tile_y, tile_row) = (row / TILE_HEIGHT_PX, row % TILE_HEIGHT_PX);

            for tile_x in 0..FRAME_WIDTH_TILES {
                let tile_num = tile_y * FRAME_WIDTH_TILES + tile_x;
                let chr_addr = tile_row + tile_num * TILE_SIZE_BYTES;

                assert_eq!(used_addrs[chr_addr as usize], false);
                used_addrs[chr_addr as usize] = true;
                used_addrs[chr_addr as usize + 8] = true;

                let (low_byte, high_byte) = read_tile_lohi(chr_addr as u16);
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

    fn evaluate_sprites_next_scanline(&mut self) {
        if !self.sprites_enabled() {
            return;
        }

        const FIRST_CYCLE: i16 = 257;
        let n = (self.ppu_cycle - FIRST_CYCLE) as usize;
        assert!(n < 64);

        // Process the sprite in the primary OAM at this location. If it is in the range of the
        // next scanline being rendered, copy it to the second OAM to be rendered
        let sprite_range = (4 * n)..((4 * n) + 4);
        let sprite = Sprite::from(&self.oam_primary[sprite_range]);
        if !self.sprite_hit_next_scanline(&sprite) {
            return;
        }

        if self.oam_secondary.sprites.len() >= 8 {
            // Sprite found but all of them are already set. Set the overflow flag without adding
            // the sprite to be rendered
            self.registers.ctrl |= PpuStatus::SPRITE_OVERFLOW;
            return;
        }

        // This is sprite 0 in the OAM
        if n == 0 {
            self.oam_secondary.has_sprite_0 = true;
        }

        // Success: fouund a sprite we can actually push
        self.oam_secondary.sprites.push(sprite);
    }

    fn create_range(rev: bool, n: usize) -> impl Iterator<Item = usize> {
        let (mut start, step) = if rev {
            (n, usize::max_value())
        } else {
            (usize::max_value(), 1)
        };

        std::iter::repeat_with(move || {
            start = start.wrapping_add(step);
            start
        })
        .take(n)
    }

    fn draw_sprites(&mut self) {
        assert!(self.is_visible_cycle());
        assert!(
            self.oam_secondary.sprites.len() < 9,
            "The NES can only draw 8 sprites"
        );

        // This must happen when the PPU is drawing the picture, as this is the next scanline from
        // when the sprites were evaluated
        if self.show_clipped_lhs() && !self.sprite0_past_rhs() {
            self.registers.status |= PpuStatus::SPRITE_0_HIT;
        }

        let large_sprites = self.registers.ctrl & PpuCtrl::SPRITE_HEIGHT != 0;

        let mut sprite_queue = OamSecondary::default();
        std::mem::swap(&mut sprite_queue, &mut self.oam_secondary);

        // Sprites with a lower index are drawn in front, reverse the vec
        for sprite in sprite_queue.sprites.iter().rev() {
            if !sprite.is_visible() {
                continue;
            }

            let (pattern_table_base, tile) = if large_sprites {
                sprite.tile16()
            } else {
                (self.sprite_table_base(), sprite.tile8())
            };

            assert!(sprite.y() <= self.scanline);
            let mut sprite_row = (self.scanline - sprite.y()) as u16;
            if sprite.vert_flip() {
                sprite_row = if large_sprites { 16 } else { 8 } - sprite_row;
            }
            assert!(sprite_row < 16, "sprite row too large: {}", sprite_row);

            // https://www.nesdev.org/wiki/PPU_palettes
            let d4 = 1_u8; // Sprite, choose sprite palette
            let d3_d2 = sprite.color_d3_d2();

            let tile_row_addr = pattern_table_base | (tile << TILE_STRIDE_SHIFT) | sprite_row;
            let pattern_lo = self.ppu_internal_read(tile_row_addr);
            let pattern_hi = self.ppu_internal_read(tile_row_addr + TILE_HI_OFFSET_BYTES);
            let color_idx = tile_lohi_to_idx(pattern_lo, pattern_hi);
            let px_idx = PPU::create_range(sprite.horiz_flip(), 8);

            // FIXME: factor this out to merge with background
            let base_addr = self.render_base_address(sprite.x() as usize);
            for (px, &lo) in px_idx.zip(color_idx.iter()).filter(|(_, &lo)| lo != 0) {
                assert!(lo < 4);

                let palette_addr = (d4 << 4) | (d3_d2 << 2) | lo;
                let color_idx = self.palette_read(palette_addr as u16);
                let color = PALETTE_COLOR_LUT[color_idx as usize];

                let buf_addr = PX_SIZE_BYTES * (base_addr + px);
                let render_slice = &mut self.frame_buf[buf_addr..(buf_addr + PX_SIZE_BYTES)];

                // FIXME: add extra checks mode
                // assert!(render_slice.iter().all(|&p| p == 0));
                render_slice.copy_from_slice(&to_u8_slice(color));
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn nametable_mirroring() {
        assert_eq!(mirror(&Mirroring::Vertical, 0x0000), 0x0000);
        assert_eq!(mirror(&Mirroring::Vertical, 0x1400), 0x1400);
        assert_eq!(mirror(&Mirroring::Vertical, 0x3038), 0x3038);
        assert_eq!(mirror(&Mirroring::Vertical, 0x7438), 0x7438);
        assert_eq!(mirror(&Mirroring::Vertical, 0xF801), 0xF001);

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
