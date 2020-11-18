use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use venus::graphics::Renderer;
use venus::sdl_interface::*;

fn main() {
    let mut renderer = Renderer::new().unwrap();

    let mut y: i32 = 0;
    let mut scanline: [u8; 256] = [0; 256];
    'running: loop {
        let _ = renderer
            .render(y, &scanline)
            .expect("Error rendering scanline");

        for event in SDL2Intrf::context()
            .event_pump()
            .expect("Missing sdl event pump")
            .poll_iter()
        {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }
        ::std::thread::sleep(Duration::new(0, venus::graphics::FRAME_RATE_NS));

        y = (y + 1) % 240;
        assert!(scanline.iter().all(|x| x == &scanline[0]));
        for c in scanline.iter_mut() {
            let old = *c;
            *c = (old + 1) % 64;
        }
    }
}
