use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use venus::cartridge::*;
use venus::ppu::PPU;
use venus::sdl_interface::SDL2Intrf;

fn main() {
    return;
    let game = Cartridge::load("donkey-kong.nes").expect("Failed to load ROM");
    let mut ppu = PPU::new(game);

    'running: loop {
        let mut event_pump = SDL2Intrf::context().event_pump().unwrap();

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

        ppu.clock(120);
        ::std::thread::sleep(Duration::new(0, venus::graphics::FRAME_RATE_NS));
    }
}
