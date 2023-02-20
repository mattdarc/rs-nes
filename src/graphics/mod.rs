pub mod nop;
pub mod sdl2;

pub mod constants {
    use std::mem::size_of;

    pub const PX_SIZE_BYTES: u32 = (size_of::<u32>() / size_of::<u8>()) as u32; // RGB888 rounds up to word
    pub const WINDOW_NAME: &str = "Venus NES Emulator";

    // TODO these should not be constant, and should be able to be resized with the emulator screen
    pub const WINDOW_WIDTH_MUL: u32 = 5;
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
    let width = 256;

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
