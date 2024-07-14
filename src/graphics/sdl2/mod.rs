use super::constants::*;
use super::Renderer;
use crate::timer;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{Texture, WindowCanvas};
use sdl2::video::DisplayMode;
use std::mem::MaybeUninit;
use std::sync::{mpsc, Once};
use std::thread;

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

/// The raw pointers here are safe because both the renderer and the buffers are owned by the PPU
enum RenderRequest {
    Stop,
    DrawLine(*const u8, usize, u32),
    DrawFrame(*const u8, usize),
}

unsafe impl Send for RenderRequest {}

struct SDLBackend<'a> {
    canvas: WindowCanvas,
    texture: Texture<'a>,
    width_px: usize,
    height_px: usize,
}

unsafe impl Send for SDLBackend<'_> {}

impl SDLBackend<'_> {
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

        timer::timed!("renderer::update", {
            self.texture.update(None, &buf, pitch_bytes).unwrap()
        });
        timer::timed!("renderer::update", {
            self.canvas.copy(&self.texture, None, None).unwrap()
        });
        timer::timed!("renderer::present", { self.canvas.present() });
    }

    fn present(&mut self) {}
}

pub struct SDLRenderer {
    sender: mpsc::SyncSender<RenderRequest>,
    render_thread: thread::JoinHandle<()>,
}

impl SDLRenderer {
    pub fn new(width: usize, height: usize) -> Self {
        let canvas = SDLBackend::init_canvas();

        // FIXME: Ideally we wouldn't need to leak but I can't get the lifetime right here...
        // Since we create only one of these it should be fine
        let tex_creator = Box::leak(Box::new(canvas.texture_creator()));
        let texture = tex_creator
            .create_texture_target(None, width as u32, height as u32)
            .unwrap();

        let mut backend = SDLBackend {
            canvas,
            texture,
            width_px: width,
            height_px: height,
        };

        // Use a bound of 0 so the PPU wwill have to wait until the previous frame is done drawing
        let (sender, receiver) = mpsc::sync_channel(0);
        let render_thread = thread::spawn(move || loop {
            match receiver.recv().expect("Error receiving render requests") {
                RenderRequest::Stop => return,
                RenderRequest::DrawFrame(buffer, size) => {
                    backend.draw_frame(unsafe { std::slice::from_raw_parts(buffer, size) })
                }
                RenderRequest::DrawLine(buffer, size, row) => {
                    backend.draw_line(unsafe { std::slice::from_raw_parts(buffer, size) }, row)
                }
            }
        });

        SDLRenderer {
            sender,
            render_thread,
        }
    }
}

impl Renderer for SDLRenderer {
    fn draw_line(&mut self, scanline: &[u8], row: u32) {
        self.sender
            .send(RenderRequest::DrawLine(
                scanline.as_ptr(),
                scanline.len(),
                row,
            ))
            .unwrap();
    }

    /// Display a buffer buf on the screen. The format of the buffer is assumed to be in the RGB888
    /// format
    fn draw_frame(&mut self, buf: &[u8]) {
        self.sender
            .send(RenderRequest::DrawFrame(buf.as_ptr(), buf.len()))
            .unwrap();
    }
}

impl Drop for SDLRenderer {
    fn drop(&mut self) {
        self.sender.send(RenderRequest::Stop).unwrap();
    }
}
