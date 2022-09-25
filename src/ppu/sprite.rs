#[derive(Clone, Copy)]
pub enum Priority {
    Foreground,
    Background,
}

#[derive(Clone, Copy)]
pub struct Sprite {
    x: u8,
    y: u8,
    bank_sel: usize,
    tile_num: u8,
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

    pub fn in_scanline(&self, line: i16) -> bool {
        self.y as i16 <= line && line < (self.y + Sprite::PIX_HEIGHT) as i16
    }

    pub fn pix_width(sprite_size: u8) -> u8 {
        match sprite_size {
            0 => 8,
            1 => 16,
            _ => unreachable!(),
        }
    }

    pub fn raw(&self) -> &[u8] {
        assert!(self.bytes.len() == 4, "Sprites should be 4 bytes in size!");
        self.bytes.as_slice()
    }

    pub fn is_valid(&self) -> bool {
        self.bytes != [0xFF; 4]
    }

    pub fn x(&self) -> i32 {
        self.x as i32
    }

    pub fn y(&self) -> i32 {
        self.y as i32
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

    pub fn palette_num(&self) -> u8 {
        self.palette_num
    }

    pub fn vert_flip(&self) -> bool {
        self.vert_flip
    }
}

impl std::convert::From<&[u8]> for Sprite {
    fn from(bytes: &[u8]) -> Sprite {
        assert!(bytes.len() == Sprite::BYTES_PER);
        let x = bytes[3];
        let y = bytes[0];
        let bank_sel = if bytes[1] & 0x1 != 0 { 0x1000 } else { 0x0000 };
        let tile_num = bytes[1] & 0xFE;

        // This is 4-7 but I am using it like an index into the palette table
        let palette_num = (bytes[2] & 0x3) << 2;

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
            bank_sel,
            tile_num,
            palette_num,
            priority,
            vert_flip,
            horiz_flip,

            bytes: bytes_arr,
        }
    }
}
