/// TODO: This should be moved out into another module that implements the graphics/ui interface.
/// Or provided as the default ui
///
use super::constants::*;
use super::Renderer;
use crate::timer;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{TextureAccess, TextureCreator, WindowCanvas};
use sdl2::video::{DisplayMode, WindowContext};
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

pub struct SDLRenderer {
    canvas: WindowCanvas,
    tex_creator: TextureCreator<WindowContext>,
}

impl SDLRenderer {
    pub fn new() -> Self {
        let canvas = SDLRenderer::init_canvas();
        let tex_creator = canvas.texture_creator();
        SDLRenderer {
            canvas,
            tex_creator,
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

impl Renderer for SDLRenderer {
    fn render_line(&mut self, scanline: &[u8], row: u32) {
        timer::timed!("render", {
            assert_eq!(
                scanline.len() as u32,
                NES_SCREEN_WIDTH,
                "scanline is not the width of the screen!"
            );

            let mut texture = self
                .tex_creator
                .create_texture(None, TextureAccess::Streaming, NES_SCREEN_WIDTH, 1)
                .unwrap();
            texture
                .update(None, &scanline, (NES_SCREEN_WIDTH * PX_SIZE_BYTES) as usize)
                .unwrap();

            let dst_rect = Rect::new(
                0,
                (WINDOW_HEIGHT_MUL * row) as i32,
                WINDOW_WIDTH,
                WINDOW_HEIGHT_MUL,
            );

            self.canvas.copy(&texture, None, Some(dst_rect)).unwrap();
            self.canvas.present();
        })
    }

    /// Display a buffer buf on the screen. The format of the buffer is assumed to be in the RGB888
    /// format
    fn render_frame(&mut self, buf: &[u8], width: u32, height: u32) {
        timer::timed!("render", {
            let mut texture = self
                .tex_creator
                .create_texture_target(None, width, height)
                .unwrap();

            let pitch_bytes: usize = PX_SIZE_BYTES as usize * width as usize;
            texture.update(None, &buf, pitch_bytes).unwrap();
            self.canvas.copy(&texture, None, None).unwrap();
            self.canvas.present();
        });
    }
}

impl Clone for SDLRenderer {
    fn clone(&self) -> Self {
        SDLRenderer::new()
    }
}
