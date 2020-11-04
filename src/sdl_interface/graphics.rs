use super::SDL2Intrf;

use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{TextureAccess, WindowCanvas};
use sdl2::surface::Surface;

const PALETTE_TABLE: [u32; 64] = [
    0x7C7C7C, 0x0000FC, 0x0000BC, 0x4428BC, 0x940084, 0xA80020, 0xA81000, 0x881400,
    0x503000, 0x007800, 0x006800, 0x005800, 0x004058, 0x000000, 0x000000, 0x000000,
    0xBCBCBC, 0x0078F8, 0x0058F8, 0x6844FC, 0xD800CC, 0xE40058, 0xF83800, 0xE45C10,
    0xAC7C00, 0x00B800, 0x00A800, 0x00A844, 0x008888, 0x000000, 0x000000, 0x000000,
    0xF8F8F8, 0x3CBCFC, 0x6888FC, 0x9878F8, 0xF878F8, 0xF85898, 0xF87858, 0xFCA044,
    0xF8B800, 0xB8F818, 0x58D854, 0x58F898, 0x00E8D8, 0x787878, 0x000000, 0x000000,
    0xFCFCFC, 0xA4E4FC, 0xB8B8F8, 0xD8B8F8, 0xF8B8F8, 0xF8A4C0, 0xF0D0B0, 0xFCE0A8,
    0xF8D878, 0xD8F878, 0xB8F8B8, 0xB8F8D8, 0x00FCFC, 0xF8D8F8, 0x000000, 0x000000,
];

const BLOCK_SIZE: u32 = 8; // 8x8 tile
const BYTES_PER_PIX: u32 = 4; // RGB888 rounds up to word
const WINDOW_NAME: &str = "Venus NES Emulator";

pub struct Renderer {
    canvas: WindowCanvas,
}

impl Renderer {
    pub fn new() -> Result<Renderer, Box<dyn std::error::Error>> {
        let video_subsystem = SDL2Intrf::context().video()?;

        let window = video_subsystem
            .window(WINDOW_NAME, 800, 600)
            .position_centered()
            .build()?;

        let mut canvas = window.into_canvas().build()?;
        canvas.clear();
        Ok(Renderer { canvas })
    }

    // TODO: May need top find a way to batch these together, or clear() only
    // when the screen needs to be updated
    pub fn render(
        &mut self, row: i32, scanline: &[u8],
    ) -> Result<sdl2::EventPump, Box<dyn std::error::Error>> {
        let canvas = &mut self.canvas;
        let line: Vec<_> = scanline
            .iter()
            .map(|c| PALETTE_TABLE[*c as usize])
            .collect();
        // canvas.clear();

        let creator = canvas.texture_creator();

        // TODO: Should this be created each time or reused??
        let line_size = scanline.len() as u32;
        let mut texture = creator.create_texture(
            Some(PixelFormatEnum::RGB888),
            TextureAccess::Streaming,
            line_size,
            1,
        )?;
        texture.update(None, scanline, (line_size * BYTES_PER_PIX) as usize)?;
        let dst_rect = Rect::new(0, row, 800, 1);
        canvas.copy(&texture, None, Some(dst_rect))?;
        canvas.present();
        Ok(SDL2Intrf::context().event_pump()?)
    }
}

impl Clone for Renderer {
    fn clone(&self) -> Self {
        Renderer::new().unwrap()
    }
}
