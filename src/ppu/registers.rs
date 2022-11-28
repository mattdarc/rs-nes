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
                self.next_wr = ScrollNextWrite::X;
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

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
enum AddrNextWrite {
    Hi,
    Lo,
}

#[derive(Clone, Copy, Debug)]
pub struct PpuAddr {
    addr: u16,
    next_wr: AddrNextWrite,
}

impl Default for PpuAddr {
    fn default() -> Self {
        PpuAddr {
            addr: 0,
            next_wr: AddrNextWrite::Hi,
        }
    }
}

impl PpuAddr {
    pub fn write(&mut self, val: u8) {
        match self.next_wr {
            AddrNextWrite::Hi => {
                self.addr = ((val as u16) << 8) | (self.addr & 0xFF);
                self.next_wr = AddrNextWrite::Lo;
            }
            AddrNextWrite::Lo => {
                self.addr = (self.addr & 0xFF00) | val as u16;
                self.next_wr = AddrNextWrite::Hi;
            }
        }
    }

    pub fn incr(&mut self, amt: u16) {
        self.addr = self.addr.wrapping_add(amt) & 0x3FFF;
    }

    pub fn to_u16(self) -> u16 {
        self.addr & 0x3FFF
    }

    pub fn reset_addr(&mut self) {
        self.next_wr = AddrNextWrite::Hi;
    }
}
