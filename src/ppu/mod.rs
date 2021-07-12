use crate::cartridge::Cartridge;

pub struct PPU {
    game: Cartridge,
}

impl PPU {
    pub fn new(game: Cartridge) -> Self {
        PPU { game }
    }
}
