#[derive(Default, Clone)]
pub struct PPU {
}

impl PPU {
    pub fn read(&self, addr: usize) -> u8 {
	0
    }

    pub fn write(&mut self, addr: usize, val: u8) {
    }
}
