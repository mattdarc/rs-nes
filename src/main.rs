use venus::*;

fn main() {
    let mut vnes = VNES::new();
    while let Err(e) = vnes.play("../roms/Tetris.nes") {
	println!("Error: {}", e)
    }
}
