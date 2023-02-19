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

#[derive(Default)]
pub struct Registers {
    pub ctrl: u8,
    pub mask: u8,
    pub status: u8,
    pub oamaddr: u8,
    pub oamdata: u8,
    pub addr: PpuAddr,
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
enum AddrNextWrite {
    FirstWrite,
    SecondWrite,
}

#[derive(Clone, Copy, Debug)]
pub struct PpuAddr {
    tmp: u16,
    addr: u16,
    fine_x: u16,
    next_wr: AddrNextWrite,
}

impl Default for PpuAddr {
    fn default() -> Self {
        PpuAddr {
            tmp: 0,
            addr: 0,
            fine_x: 0,
            next_wr: AddrNextWrite::FirstWrite,
        }
    }
}

impl PpuAddr {
    const HORIZ_MASK: u16 = 0x041F;
    const VERT_MASK: u16 = !PpuAddr::HORIZ_MASK;

    pub fn addr_write(&mut self, val: u8) {
        match self.next_wr {
            AddrNextWrite::FirstWrite => {
                self.tmp = ((val as u16) << 8) | (self.tmp & 0xFF);
                self.next_wr = AddrNextWrite::SecondWrite;
            }
            AddrNextWrite::SecondWrite => {
                // Addresses written via PPUADDR are mirrored down to 0-0x3FFF
                self.tmp = (self.tmp & 0x3F00) | val as u16;
                self.addr = self.tmp;
                self.next_wr = AddrNextWrite::FirstWrite;
            }
        }
    }

    pub fn scroll_write(&mut self, val: u8) {
        match self.next_wr {
            AddrNextWrite::FirstWrite => {
                self.tmp = ((val as u16) >> 3) | (self.tmp & 0xFFE0);
                self.fine_x = (val & 0x7) as u16;
                self.next_wr = AddrNextWrite::SecondWrite;
            }
            AddrNextWrite::SecondWrite => {
                let fine_y = ((val as u16) & 0x7) << 12;
                let coarse_y = ((val as u16) >> 3) << 5;
                let nt_select = self.tmp & 0xC00;
                self.tmp = fine_y | nt_select | coarse_y;
                self.next_wr = AddrNextWrite::FirstWrite;
            }
        }
    }

    pub fn set_nametable(&mut self, ctrl: u8) {
        let addr_nt_mask = (PpuCtrl::NAMETABLE_ADDR as u16) << 10;
        let nt_base = (ctrl & PpuCtrl::NAMETABLE_ADDR) as u16;
        self.tmp = (self.tmp & !addr_nt_mask) | (nt_base << 10);
    }

    pub fn incr(&mut self, amt: u16) {
        self.addr += amt;
    }

    pub fn incr_x(&mut self) {
        let old_addr = self.addr;
        if (self.addr & 0x001F) == 0x001F {
            self.addr &= 0xFFE0;
            self.addr ^= 0x0400;
        } else {
            self.addr += 1;
        }

        // Updated the horizontal component. Vertical should be the same
        assert_eq!(
            self.addr & PpuAddr::VERT_MASK,
            old_addr & PpuAddr::VERT_MASK
        )
    }

    pub fn incr_y(&mut self) {
        let old_addr = self.addr;

        if (self.addr & 0x7000) == 0 {
            self.addr += 0x1000;
            return;
        }

        self.addr &= !0x7000;
        let mut coarse_y = (self.addr & PpuAddr::VERT_MASK) >> 5;
        if coarse_y == 29 {
            coarse_y = 0;
            self.addr ^= 0x0800;
        } else if coarse_y == 31 {
            coarse_y = 0;
        } else {
            coarse_y += 1;
        }

        self.addr = (self.addr & !PpuAddr::VERT_MASK) | (coarse_y << 5);

        // Updated the vertical component. Horizontal should be the same
        assert_eq!(
            self.addr & PpuAddr::HORIZ_MASK,
            old_addr & PpuAddr::HORIZ_MASK
        )
    }

    pub fn to_u16(self) -> u16 {
        self.addr & 0x3FFF
    }

    pub fn reset(&mut self) {
        self.next_wr = AddrNextWrite::FirstWrite;
    }

    pub fn sync_x(&mut self) {
        self.addr = (self.tmp & PpuAddr::HORIZ_MASK) | (self.addr & !PpuAddr::HORIZ_MASK);
    }

    pub fn sync_y(&mut self) {
        self.addr = (self.tmp & PpuAddr::VERT_MASK) | (self.addr & !PpuAddr::VERT_MASK);
    }
}
