pub mod graphics;
use std::mem::MaybeUninit;
use std::sync::{Arc, Once};

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
