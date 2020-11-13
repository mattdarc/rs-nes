use venus::cartridge::*;
use venus::cpu::*;
use venus::*;

fn main() {
    let mut vnes = VNES::new();
    while let Err(e) = vnes.play("../roms/Tetris.nes") {
        println!("Error: {}", e)
    }

    let cart = Cartridge::load("/home/mattdarcangelo/rs-nes/roms/Tetris.nes").unwrap();
    let mut cpu = Ricoh2A03::with(&cart);
    cpu.init();
    cpu.run();
}
