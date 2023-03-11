#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Priority {
    Foreground,
    Background,
}

#[derive(Clone, Copy)]
pub struct Sprite {
    x: u8,
    y: u8,
    palette_num: u8,
    priority: Priority,
    vert_flip: bool,
    horiz_flip: bool,

    bytes: [u8; 4],
}

impl Sprite {
    pub const BYTES_PER: usize = 4;
    pub const PIX_HEIGHT: u8 = 8;

    pub fn default() -> Self {
        // Reading from an uninitialized OAM secondary will return 0xFF
        Sprite::from([0xFF_u8; Sprite::BYTES_PER].as_slice())
    }

    pub fn raw(&self) -> &[u8] {
        assert!(self.bytes.len() == 4, "Sprites should be 4 bytes in size!");
        self.bytes.as_slice()
    }

    pub fn is_valid(&self) -> bool {
        self.bytes != [0xFF; 4]
    }

    pub fn x(&self) -> i16 {
        self.x as i16
    }

    pub fn y(&self) -> i16 {
        self.y as i16
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
        self.palette_num
    }

    pub fn vert_flip(&self) -> bool {
        self.vert_flip
    }

    pub fn horiz_flip(&self) -> bool {
        self.horiz_flip
    }

    pub fn is_visible(&self) -> bool {
        self.priority == Priority::Foreground
    }
}

impl std::convert::From<&[u8]> for Sprite {
    fn from(bytes: &[u8]) -> Sprite {
        assert!(bytes.len() == Sprite::BYTES_PER);
        let x = bytes[3];
        let y = bytes[0];
        let palette_num = bytes[2] & 0x3;

        let priority = if bytes[2] & 0x20 != 0 {
            Priority::Background
        } else {
            Priority::Foreground
        };
        let horiz_flip = bytes[2] & 0x40 != 0;
        let vert_flip = bytes[2] & 0x80 != 0;
        let mut bytes_arr = [0; 4];
        for i in 0..4 {
            bytes_arr[i] = bytes[i];
        }
        Sprite {
            x,
            y,
            palette_num,
            priority,
            vert_flip,
            horiz_flip,

            bytes: bytes_arr,
        }
    }
}
