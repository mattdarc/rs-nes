use bitfield::bitfield;

#[derive(Copy, Clone, Debug)]
pub enum SpriteSize {
    P8x8,
    P8x16,
}

#[derive(Copy, Clone, Debug)]
pub enum SlaveSel {
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
    bit_base_nametable_addr, _   : 1, 0;
    bit_vram_increment, _        : 2;
    bit_sprite_table_addr, _     : 3;
    bit_background_table_addr, _ : 4;
    bit_sprite_size, _           : 5;
    bit_master_slave_sel, _      : 6;
    pub nmi, _                   : 7;
    bit_scroll_pos, _            : 1, 2;
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

impl Control {
    pub fn vram_increment(&self) -> usize {
        if self.bit_vram_increment() {
            32
        } else {
            1
        }
    }

    pub fn sprite_table_addr(&self) -> usize {
        if self.bit_sprite_table_addr() {
            0x1000
        } else {
            0x0000
        }
    }

    pub fn background_table_addr(&self) -> usize {
        if self.bit_background_table_addr() {
            0x1000
        } else {
            0x0000
        }
    }

    pub fn sprite_size(&self) -> SpriteSize {
        if self.bit_sprite_size() {
            SpriteSize::P8x16
        } else {
            SpriteSize::P8x8
        }
    }

    pub fn master_slave_sel(&self) -> SlaveSel {
        if self.bit_master_slave_sel() {
            SlaveSel::WriteColor
        } else {
            SlaveSel::ReadBackdrop
        }
    }

    pub fn scroll_pos(&self) -> Scroll {
        let mut scroll = Scroll(0, 0);
        match self.bit_scroll_pos() {
	    0 => Scroll(0, 0),
	    1 => Scroll(256, 0),
	    2 => Scroll(0, 240),
	    3 => Scroll(256, 240),
        }
    }

    pub fn write(&mut self, val: u8) {
	self.0 = val;
    }
}

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

impl VRAMAddr {
    pub fn read(&self) -> usize {
	(self.0 & 0x3F) as usize
    }
}
