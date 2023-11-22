#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Priority {
    Foreground,
    Background,
}

#[derive(Clone, Copy)]
pub struct Sprite {
    bytes: SpriteRaw,
}

pub type SpriteRaw = [u8; Sprite::BYTES_PER];

impl Default for Sprite {
    fn default() -> Self {
        // Reading from an uninitialized OAM secondary will return 0xFF
        Sprite::from(&[0xFF_u8; Sprite::BYTES_PER])
    }
}

impl Sprite {
    pub const BYTES_PER: usize = 4;
    pub const PIX_HEIGHT: u8 = 8;

    pub fn is_valid(&self) -> bool {
        self.bytes != [0xFF; 4]
    }

    pub fn x(&self) -> i16 {
        self.bytes[3] as i16
    }

    pub fn y(&self) -> i16 {
        self.bytes[0] as i16
    }

    pub fn tile16(&self) -> (u16, u16) {
        let bank = if self.bytes[1] & 0x1 != 0 {
            0x1000
        } else {
            0x0000
        };

        (bank, (self.bytes[1] & 0xFE) as u16)
    }

    pub fn tile8(&self) -> u16 {
        self.bytes[1] as u16
    }

    pub fn color_d3_d2(&self) -> u8 {
        self.bytes[2] & 0x3
    }

    pub fn vert_flip(&self) -> bool {
        self.bytes[2] & 0x80 != 0
    }

    pub fn horiz_flip(&self) -> bool {
        self.bytes[2] & 0x40 != 0
    }

    pub fn is_visible(&self) -> bool {
        let priority = if self.bytes[2] & 0x20 != 0 {
            Priority::Background
        } else {
            Priority::Foreground
        };

        priority == Priority::Foreground
    }
}

impl std::convert::From<&[u8; 4]> for Sprite {
    fn from(bytes: &[u8; 4]) -> Sprite {
        assert!(bytes.len() == Sprite::BYTES_PER);

        Sprite {
            bytes: bytes.clone(),
        }
    }
}
