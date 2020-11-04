pub enum Priority {
    Foreground,
    Background,
}

#[derive(Clone, Copy)]
pub enum Size {
    Large,
    Small,
}

pub struct Sprite {
    x: u8,
    y: u8,
    bank_sel: usize,
    tile_num: u8,
    palette_num: u8,
    priority: Priority,
    vert_flip: bool,
    horiz_flip: bool,
}

impl Sprite {
    pub const BYTES_PER: usize = 4;

    fn size(&self) -> (u8, u8) {
        match self {
            Large => (8, 16),
            Small => (8, 8),
        }
    }

    pub fn x(&self) -> u8 {
        self.x
    }

    pub fn y(&self) -> u8 {
        self.y
    }

    pub fn bank_sel(&self) -> usize {
        self.bank_sel
    }

    pub fn tile_num(&self) -> u8 {
        self.tile_num
    }

    pub fn addr(&self) -> usize {
        self.tile_num as usize * 16
    }

    pub fn table_addr(&self) -> usize {
        self.bank_sel
    }
}

impl std::convert::From<&[u8]> for Sprite {
    fn from(bytes: &[u8]) -> Sprite {
        assert!(bytes.len() == Sprite::BYTES_PER);
        let x = bytes[3];
        let y = bytes[0];
        let bank_sel = if bytes[1] & 0x1 != 0 { 0x1000 } else { 0x0000 };
        let tile_num = bytes[1] & 0xFE;
        let palette_num = bytes[2] & 0x3 + 4;
        assert!(palette_num < 8);
        let priority = if bytes[2] & 0x20 != 0 {
            Priority::Background
        } else {
            Priority::Foreground
        };
        let horiz_flip = bytes[2] & 0x40 != 0;
        let vert_flip = bytes[2] & 0x80 != 0;
        Sprite {
            x,
            y,
            bank_sel,
            tile_num,
            palette_num,
            priority,
            vert_flip,
            horiz_flip,
        }
    }
}
