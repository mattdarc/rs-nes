pub mod graphics;
use std::mem::MaybeUninit;
use std::sync::{Arc, Once};

static INIT_SDL: Once = Once::new();
static mut SDL_CONTEXT: MaybeUninit<Arc<sdl2::Sdl>> = MaybeUninit::<Arc<sdl2::Sdl>>::uninit();

pub struct SDL2Intrf;

impl SDL2Intrf {
    pub fn context() -> Arc<sdl2::Sdl> {
        unsafe {
            INIT_SDL.call_once(|| {
                SDL_CONTEXT
                    .as_mut_ptr()
                    .write(Arc::new(sdl2::init().unwrap()));
            });
            (*SDL_CONTEXT.as_ptr()).clone()
        }
    }
}
