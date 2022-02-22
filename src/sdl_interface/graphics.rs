use super::SDL2Intrf;

use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{TextureAccess, WindowCanvas};
use sdl2::video::DisplayMode;
use std::mem::size_of;
use std::slice::from_raw_parts;

const PALETTE_TABLE: [u32; 64] = [
    0x7C7C7C, 0x0000FC, 0x0000BC, 0x4428BC, 0x940084, 0xA80020, 0xA81000, 0x881400, 0x503000,
    0x007800, 0x006800, 0x005800, 0x004058, 0x000000, 0x000000, 0x000000, 0xBCBCBC, 0x0078F8,
    0x0058F8, 0x6844FC, 0xD800CC, 0xE40058, 0xF83800, 0xE45C10, 0xAC7C00, 0x00B800, 0x00A800,
    0x00A844, 0x008888, 0x000000, 0x000000, 0x000000, 0xF8F8F8, 0x3CBCFC, 0x6888FC, 0x9878F8,
    0xF878F8, 0xF85898, 0xF87858, 0xFCA044, 0xF8B800, 0xB8F818, 0x58D854, 0x58F898, 0x00E8D8,
    0x787878, 0x000000, 0x000000, 0xFCFCFC, 0xA4E4FC, 0xB8B8F8, 0xD8B8F8, 0xF8B8F8, 0xF8A4C0,
    0xF0D0B0, 0xFCE0A8, 0xF8D878, 0xD8F878, 0xB8F8B8, 0xB8F8D8, 0x00FCFC, 0xF8D8F8, 0x000000,
    0x000000,
];

const BLOCK_SIZE: u32 = 8; // 8x8 tile
const PX_SIZE_BYTES: u32 = (size_of::<u32>() / size_of::<u8>()) as u32; // RGB888 rounds up to word

const WINDOW_NAME: &str = "Venus NES Emulator";

// TODO these should not be constant, and should be able to be resized with the emulator screen
const WINDOW_WIDTH_MUL: u32 = 4;
const WINDOW_HEIGHT_MUL: u32 = 3;
const WINDOW_WIDTH: u32 = NES_SCREEN_WIDTH * WINDOW_WIDTH_MUL;
const WINDOW_HEIGHT: u32 = NES_SCREEN_HEIGHT * WINDOW_HEIGHT_MUL;

pub const NES_SCREEN_WIDTH: u32 = 256;
pub const NES_SCREEN_HEIGHT: u32 = 240;

pub trait Renderer {
    fn render_line(&mut self, line: &[u8], row: u32);
    fn render_frame(&mut self, buf: &[u8], width: u32, height: u32);
}

// for some reason textures are repeating every 120 bytes
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

// This is safe since I know that the underlying data is valid and contiguous
fn to_sdl2_slice(slice: &[u32]) -> &[u8] {
    unsafe {
        from_raw_parts(
            slice.as_ptr() as *const u8,
            slice.len() * PX_SIZE_BYTES as usize,
        )
    }
}

pub struct NOPRenderer;
impl NOPRenderer {
    pub fn new() -> Self {
        NOPRenderer {}
    }
}

impl Renderer for NOPRenderer {
    fn render_line(&mut self, _line: &[u8], _row: u32) {}
    fn render_frame(&mut self, _buf: &[u8], _width: u32, _height: u32) {}
}

pub struct SDLRenderer {
    canvas: Option<WindowCanvas>,
}

impl SDLRenderer {
    pub fn new() -> Self {
        SDLRenderer { canvas: None }
    }

    fn get_or_create_canvas(&mut self) -> &mut WindowCanvas {
        self.canvas.get_or_insert_with(SDLRenderer::init_canvas)
    }

    fn init_canvas() -> WindowCanvas {
        let sdl_ctx = SDL2Intrf::context();
        let video_subsystem = sdl_ctx.video().unwrap();

        let mut window = video_subsystem
            .window(WINDOW_NAME, WINDOW_WIDTH, WINDOW_HEIGHT)
            .position_centered()
            .build()
            .unwrap();
        window
            .set_display_mode(Some(DisplayMode::new(
                PixelFormatEnum::RGB888,
                WINDOW_WIDTH as i32,
                WINDOW_HEIGHT as i32,
                30,
            )))
            .unwrap();

        let mut canvas = window.into_canvas().build().unwrap();
        canvas.clear();

        canvas
    }
}

impl Renderer for SDLRenderer {
    // TODO: May need to find a way to batch these together, or clear() only
    // when the screen needs to be updated
    fn render_line(&mut self, scanline: &[u8], row: u32) {
        // TODO: Better way to handle noop rendering

        assert_eq!(
            scanline.len() as u32,
            NES_SCREEN_WIDTH,
            "scanline is not the width of the screen!"
        );

        let canvas = self.get_or_create_canvas();
        let line: Vec<_> = scanline
            .iter()
            .map(|c| PALETTE_TABLE[*c as usize])
            .collect();

        let creator = canvas.texture_creator();

        // TODO: Should this be created each time or reused??
        let mut texture = creator
            .create_texture(None, TextureAccess::Streaming, NES_SCREEN_WIDTH, 1)
            .unwrap();
        texture
            .update(
                None,
                to_sdl2_slice(&line),
                (NES_SCREEN_WIDTH * PX_SIZE_BYTES) as usize,
            )
            .unwrap();
        let dst_rect = Rect::new(
            0,
            (WINDOW_HEIGHT_MUL * row) as i32,
            WINDOW_WIDTH,
            WINDOW_HEIGHT_MUL,
        );
        canvas.copy(&texture, None, Some(dst_rect)).unwrap();
        canvas.present();
    }

    /// Display a buffer buf on the screen. The format of the buffer is assumed to be in the RGB888
    /// format
    fn render_frame(&mut self, buf: &[u8], width: u32, height: u32) {
        //dump_texture_buf(&buf, PX_SIZE_BYTES);

        let canvas = self.get_or_create_canvas();
        let creator = canvas.texture_creator();
        let mut texture = creator
            .create_texture(None, TextureAccess::Streaming, width, height)
            .unwrap();

        let pitch_bytes: usize = PX_SIZE_BYTES as usize * width as usize;
        texture.update(None, &buf, pitch_bytes).unwrap();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
    }
}

impl Clone for SDLRenderer {
    fn clone(&self) -> Self {
        SDLRenderer::new()
    }
}

// impl Drop for Renderer {
//     fn drop(&mut self) {
//         println!("Screen {:#X?}", &self.screen);
//     }
// }
