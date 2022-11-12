#![allow(non_snake_case)]

pub struct PpuCtrl;
impl PpuCtrl {
    pub const NMI_ENABLE: u8 = 0x80;
    pub const SLAVE_SELECT: u8 = 0x40;
    pub const SPRITE_HEIGHT: u8 = 0x20;
    pub const BG_TABLE_ADDR: u8 = 0x10;
    pub const SPRITE_TABLE_ADDR: u8 = 0x08;
    pub const VRAM_INCR: u8 = 0x04;
    pub const NAMETABLE_ADDR: u8 = 0x03;
}

pub struct PpuMask;
impl PpuMask {
    pub const EMPH_BLUE: u8 = 0x80;
    pub const EMPH_GREEN: u8 = 0x40;
    pub const EMPH_RED: u8 = 0x20;
    pub const SHOW_SPRITES: u8 = 0x10;
    pub const SHOW_BG: u8 = 0x08;
    pub const SHOW_LEFT_SPRITES: u8 = 0x04;
    pub const SHOW_LEFT_BG: u8 = 0x02;
    pub const GRAYSCALE: u8 = 0x01;
}

pub struct PpuStatus;
impl PpuStatus {
    pub const VBLANK_STARTED: u8 = 0x80;
    pub const SPRITE_0_HIT: u8 = 0x40;
    pub const SPRITE_OVERFLOW: u8 = 0x20;
    pub const PREV_LSB: u8 = 0x1F;
}

#[derive(Eq, PartialEq)]
enum ScrollNextWrite {
    X,
    Y,
}

impl Default for ScrollNextWrite {
    fn default() -> Self {
        ScrollNextWrite::X
    }
}

#[derive(Default)]
pub struct PpuScroll {
    x: u8,
    y: u8,
    next_y: Option<u8>,
    next_wr: ScrollNextWrite,
}

impl PpuScroll {
    pub fn update_y_latch(&mut self) {
        if let Some(y) = self.next_y {
            self.y = y;
        }

        self.next_y = None;
    }

    pub fn reset_addr(&mut self) {
        self.next_wr = ScrollNextWrite::X;
    }

    // Changes made to the vertical scroll during rendering will only take effect on the next
    // frame. Always updating the value at frame end should be sufficient
    pub fn write(&mut self, val: u8) {
        match self.next_wr {
            ScrollNextWrite::X => {
                self.x = val;
                self.next_wr = ScrollNextWrite::Y;
            }
            ScrollNextWrite::Y => {
                self.next_y = Some(val);
            }
        }
    }

    pub fn x(&self) -> u8 {
        self.x
    }

    pub fn y(&self) -> u8 {
        self.y
    }
}

#[derive(Default)]
pub struct Registers {
    pub ctrl: u8,
    pub mask: u8,
    pub status: u8,
    pub oamaddr: u8,
    pub oamdata: u8,
    pub scroll: PpuScroll,
    pub addr: PpuAddr,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct PpuAddr(u16);

impl PpuAddr {
    pub fn write(&mut self, val: u8) {
        // Valid addresses are $0000-$3FFF; higher addresses will be mirrored down.
        self.0 = ((self.0 << 8) | (val as u16)) & 0x3FFF;
    }

    pub fn incr(&mut self, amt: u16) {
        self.0 = (self.0 + amt) & 0x3FFF;
    }

    pub fn to_u16(self) -> u16 {
        self.0
    }
}

impl std::convert::Into<u16> for PpuAddr {
    fn into(self) -> u16 {
        self.0
    }
}
