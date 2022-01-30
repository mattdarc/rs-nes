use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use venus::graphics::{Renderer, SDL2Intrf};

fn main() {
    return;

    let mut renderer = Renderer::new();

    let mut y: i32 = 0;
    let mut scanline: [u8; 256] = [0; 256];

    let mut event_pump = SDL2Intrf::context().event_pump().unwrap();
    'running: loop {
        renderer.render_line(y, &scanline);

        for event in event_pump.poll_iter() {
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

        if y == 0 {
            break 'running;
        }
    }
}
