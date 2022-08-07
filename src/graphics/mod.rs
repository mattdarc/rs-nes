pub mod nop;
pub mod sdl2;

pub mod constants {
    use std::mem::size_of;

    pub const PALETTE_TABLE: [u32; 64] = [
        0x7C7C7C, 0x0000FC, 0x0000BC, 0x4428BC, 0x940084, 0xA80020, 0xA81000, 0x881400, 0x503000,
        0x007800, 0x006800, 0x005800, 0x004058, 0x000000, 0x000000, 0x000000, 0xBCBCBC, 0x0078F8,
        0x0058F8, 0x6844FC, 0xD800CC, 0xE40058, 0xF83800, 0xE45C10, 0xAC7C00, 0x00B800, 0x00A800,
        0x00A844, 0x008888, 0x000000, 0x000000, 0x000000, 0xF8F8F8, 0x3CBCFC, 0x6888FC, 0x9878F8,
        0xF878F8, 0xF85898, 0xF87858, 0xFCA044, 0xF8B800, 0xB8F818, 0x58D854, 0x58F898, 0x00E8D8,
        0x787878, 0x000000, 0x000000, 0xFCFCFC, 0xA4E4FC, 0xB8B8F8, 0xD8B8F8, 0xF8B8F8, 0xF8A4C0,
        0xF0D0B0, 0xFCE0A8, 0xF8D878, 0xD8F878, 0xB8F8B8, 0xB8F8D8, 0x00FCFC, 0xF8D8F8, 0x000000,
        0x000000,
    ];

    pub const BLOCK_SIZE: u32 = 8; // 8x8 tile
    pub const PX_SIZE_BYTES: u32 = (size_of::<u32>() / size_of::<u8>()) as u32; // RGB888 rounds up to word

    pub const WINDOW_NAME: &str = "Venus NES Emulator";

    // TODO these should not be constant, and should be able to be resized with the emulator screen
    pub const WINDOW_WIDTH_MUL: u32 = 4;
    pub const WINDOW_HEIGHT_MUL: u32 = 3;
    pub const WINDOW_WIDTH: u32 = NES_SCREEN_WIDTH * WINDOW_WIDTH_MUL;
    pub const WINDOW_HEIGHT: u32 = NES_SCREEN_HEIGHT * WINDOW_HEIGHT_MUL;
    pub const FRAME_RATE_NS: u32 = 1_000_000_000 / 60 / NES_SCREEN_HEIGHT;
    pub const NES_SCREEN_WIDTH: u32 = 256;
    pub const NES_SCREEN_HEIGHT: u32 = 240;
}

pub trait Renderer {
    fn render_line(&mut self, line: &[u8], row: u32);
    fn render_frame(&mut self, buf: &[u8], width: u32, height: u32);
}

fn dump_texture_buf(buf: &[u8], px_size: usize) {
    let width = 128;

    let mut s = String::new();
    for idx in (0..buf.len()).step_by(px_size) {
        if idx % (width * px_size) == 0 {
            s.push('\n');
        }

        let val = buf[idx];
        if val != buf[idx + 1] || val != buf[idx + 2] {
            s.push('#');
        } else {
            match val {
                85 | 170 | 255 => s.push(char::from_digit((val / 85) as u32, 10).unwrap()),
                0 => s.push('.'),
                _ => s.push('?'),
            }
        }
    }

    println!("\nTiles:\n{}", &s);
}
