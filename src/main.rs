use venus::VNES;

fn main() {
    let mut vnes = VNES::new("donkey-kong.nes").unwrap();
    vnes.play().unwrap();
}
