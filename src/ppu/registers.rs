use bitfield::bitfield;

#[derive(Copy, Clone, Debug)]
pub enum SpriteSize {
    P8x8,
    P8x16,
}

#[derive(Copy, Clone, Debug)]
pub enum EXTPins {
    ReadBackdrop,
    WriteColor,
}

#[derive(Clone)]
pub struct Scroll(pub u32, pub u32);

pub const PPUCTRL: usize = 0;
pub const PPUMASK: usize = 1;
pub const PPUSTATUS: usize = 2;
pub const OAMADDR: usize = 3;
pub const OAMDATA: usize = 4;
pub const PPUSCROLL: usize = 5;
pub const PPUADDR: usize = 6;
pub const PPUDATA: usize = 7;
// const OAMDMA: usize = 0;

bitfield! {
    #[derive(Clone)]
    pub struct Control(u8);
    // After power/reset, writes to this register are ignored for about
    // 30,000 cycles.
    pub base_nametable_addr, _   : 1, 0;
    pub vram_increment, _        : 2;
    pub sprite_table_addr, _     : 3;
    pub background_table_addr, _ : 4;
    pub sprite_size, _           : 5;
    pub master_slave_sel, _      : 6;
    pub nmi, _                   : 7;
    pub scroll_pos, _            : 1, 2;
}

bitfield! {
    #[derive(Clone)]
    pub struct VRAMAddr(u16);
    pub _, coarse_x      : 4, 0;
    pub _, coarse_y      : 9, 5;
    pub _, nametable_sel : 11, 10;
    pub _, fine_y        : 14, 12;
}

bitfield! {
    #[derive(Clone)]
    pub struct Mask(u8);
    pub grayscale, _            : 0;
    pub show_left_background, _ : 1;
    pub show_left_sprites, _    : 2;
    pub show_background, _      : 3;
    pub show_sprites, _         : 4;
    pub more_red, _             : 5;
    pub more_green, _           : 6;
    pub more_blue, _            : 7;
    
}

bitfield! {
    #[derive(Clone)]
    pub struct Status(u8);
    pub _, set_low        : 4,0;
    pub _, sprite_overflow: 5;
    pub _, sprite_zero_hit: 6;
    pub _, vblank         : 7;
}

// impl Control {
    // pub fn vram_increment(&self) -> usize {
    //     if self.0 & 0x4 != 0 {
    //         32
    //     } else {
    //         1
    //     }
    // }

    // pub fn sprite_table_addr(&self) -> usize {
    //     if self.0 & 0x8 != 0 {
    //         0x1000
    //     } else {
    //         0x0000
    //     }
    // }

    // pub fn bg_table_addr(&self) -> usize {
    //     if self.0 & 0x10 != 0 {
    //         0x1000
    //     } else {
    //         0x0000
    //     }
    // }

    // pub fn sprite_size(&self) -> SpriteSize {
    //     if self.0 & 0x20 != 0 {
    //         SpriteSize::P8x16
    //     } else {
    //         SpriteSize::P8x8
    //     }
    // }

    // pub fn master_slave_sel(&self) -> EXTPins {
    //     if self.0 & 0x40 != 0 {
    //         EXTPins::WriteColor
    //     } else {
    //         EXTPins::ReadBackdrop
    //     }
    // }

    // pub fn gen_nmi(&self) -> bool {
    //     self.0 & 0x80 != 0
    // }

    // pub fn scroll_pos(&self) -> Scroll {
    //     let mut scroll = Scroll(0, 0);
    //     if self.0 & 0x1 != 0 {
    //         scroll.0 = 256;
    //     } else if self.0 & 0x2 != 0 {
    //         scroll.1 = 240;
    //     }
    //     scroll
    // }
// }

impl Status {
    pub fn read(&self) -> u8 {
	self.0
    }
}

impl Mask {
    pub fn write(&mut self, val: u8) {
	self.0 = val;
    }
}

impl Control {
    pub fn write(&mut self, val: u8) {
	self.0 = val;
    }
}

impl VRAMAddr {
    pub fn read(&self) -> usize {
	(self.0 & 0x3F) as usize
    }
}
