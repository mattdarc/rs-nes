use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use venus::cartridge::*;
use venus::ppu::PPU;

// FIXME: Should write a TestRenderer which we can implement to read/write a binary file of frames
#[test]
fn donkey_kong() {
    return;
    // let game = Cartridge::load("donkey-kong.nes").expect("Failed to load ROM");
    // let mut ppu = PPU::new(game, Box::new(venus::graphics::sdl2::SDLRenderer::new()));

    // 'running: loop {
    //     let mut event_pump = SDL2Intrf::context().event_pump().unwrap();

    //     for event in event_pump.poll_iter() {
    //         match event {
    //             Event::Quit { .. }
    //             | Event::KeyDown {
    //                 keycode: Some(Keycode::Escape),
    //                 ..
    //             } => break 'running,
    //             _ => {}
    //         }
    //     }

    //     ppu.clock(120);
    //     ::std::thread::sleep(Duration::new(0, venus::graphics::constants::FRAME_RATE_NS));
    // }
}
