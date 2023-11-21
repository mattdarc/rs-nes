use super::constants::*;
use super::Renderer;
use crate::timer;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{Texture, WindowCanvas};
use sdl2::video::DisplayMode;
use std::mem::MaybeUninit;
use std::sync::Once;

static INIT_SDL: Once = Once::new();
static mut SDL_CONTEXT: MaybeUninit<sdl2::Sdl> = MaybeUninit::uninit();

pub struct SDL2Intrf;
impl SDL2Intrf {
    pub fn context() -> &'static sdl2::Sdl {
        unsafe {
            INIT_SDL.call_once(|| {
                SDL_CONTEXT.as_mut_ptr().write(sdl2::init().unwrap());
            });
            &(*SDL_CONTEXT.as_ptr())
        }
    }
}

pub struct SDLRenderer<'a> {
    canvas: WindowCanvas,
    texture: Texture<'a>,
    width_px: usize,
    height_px: usize,
}

impl SDLRenderer<'_> {
    pub fn new(width: usize, height: usize) -> Self {
        let canvas = SDLRenderer::init_canvas();
        // FIXME: Ideally we wouldn't need to leak but I can't get the lifetime right here...
        // Since we create only one of these it should be fine
        let tex_creator = Box::leak(Box::new(canvas.texture_creator()));
        let texture = tex_creator
            .create_texture_target(None, width as u32, height as u32)
            .unwrap();

        SDLRenderer {
            canvas,
            texture,
            width_px: width,
            height_px: height,
        }
    }

    fn get_canvas(&mut self) -> &mut WindowCanvas {
        &mut self.canvas
    }

    fn init_canvas() -> WindowCanvas {
        let sdl_ctx = SDL2Intrf::context();
        let video_subsystem = sdl_ctx.video().unwrap();

        let mut window = video_subsystem
            .window(WINDOW_NAME, WINDOW_WIDTH, WINDOW_HEIGHT)
            .position_centered()
            .build()
            .unwrap();
        const REFRESH_RATE_HZ: i32 = 60;
        window
            .set_display_mode(Some(DisplayMode::new(
                PixelFormatEnum::RGB888,
                WINDOW_WIDTH as i32,
                WINDOW_HEIGHT as i32,
                REFRESH_RATE_HZ,
            )))
            .unwrap();

        let mut canvas = window.into_canvas().present_vsync().build().unwrap();
        canvas.clear();

        canvas
    }
}

impl Renderer for SDLRenderer<'_> {
    fn draw_line(&mut self, scanline: &[u8], row: u32) {
        timer::timed!("render::draw", {
            assert_eq!(
                scanline.len() as u32,
                NES_SCREEN_WIDTH,
                "scanline is not the width of the screen!"
            );

            self.texture
                .update(None, &scanline, (NES_SCREEN_WIDTH * PX_SIZE_BYTES) as usize)
                .unwrap();

            let dst_rect = Rect::new(
                0,
                (WINDOW_HEIGHT_MUL * row) as i32,
                WINDOW_WIDTH,
                WINDOW_HEIGHT_MUL,
            );

            self.canvas
                .copy(&self.texture, None, Some(dst_rect))
                .unwrap();
        })
    }

    /// Display a buffer buf on the screen. The format of the buffer is assumed to be in the RGB888
    /// format
    fn draw_frame(&mut self, buf: &[u8]) {
        let pitch_bytes: usize = PX_SIZE_BYTES as usize * self.width_px;
        assert_eq!(buf.len(), pitch_bytes * self.height_px);

        timer::timed!("render::draw", {
            self.texture.update(None, &buf, pitch_bytes).unwrap();
            self.canvas.copy(&self.texture, None, None).unwrap();
        });
    }

    fn present(&mut self) {
        timer::timed!("render::present", { self.canvas.present() });
    }
}

impl Clone for SDLRenderer<'_> {
    fn clone(&self) -> Self {
        SDLRenderer::new(self.width_px, self.height_px)
    }
}
