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

    bytes: Vec<u8>,
}

impl Sprite {
    pub const BYTES_PER: usize = 4;

    pub fn size(size: Size) -> i32 {
        match size {
            Size::Large => 16,
            Size::Small => 8,
        }
    }

    pub fn raw(&self) -> &[u8] {
        assert!(self.bytes.len() == 4, "Sprites should be 4 bytes in size!");
        self.bytes.as_slice()
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

        // this is 4-7 but I am using it like an index into the palette table
        let palette_num = (bytes[2] & 0x3) << 2;

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

            bytes: bytes.to_owned(),
        }
    }
}
