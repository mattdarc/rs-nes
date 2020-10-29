use venus::graphics::{Coordinates, Renderer, Texture};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;

fn main() {
    let mut renderer = Renderer::new().unwrap();

    let mut x: i32 = 0;
    let mut dx: i32 = 2;
    let mut y: i32 = 0;
    let mut dy: i32 = 2;
    let mut color: u8 = 0;
    'running: loop {
        let mut event_pump = renderer
            .render(Texture::new(vec![color; 64], Coordinates::new(x, y))).unwrap();

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
	if x > 800 || x < 0 {
	    dx *= -1;
	}
	if y > 600 || y < 0 {
	    dy *= -1;
	}

	x += dx;
	y += dy;
	color = (color + 1) % 64;
    }
}
