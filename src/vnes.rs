use crate::apu::*;
use crate::cartridge::*;
use crate::controller::*;
use crate::cpu::*;
use crate::ppu::*;

pub struct VNES<'a> {
    cpu: Ricoh2A03<'a>,
    cartridge: Option<&'a Cartridge>,
}

impl<'a> VNES<'a> {
    pub fn new() -> VNES<'a> {
        VNES {
            cpu: Ricoh2A03::new(),
            cartridge: None,
        }
    }

    // TODO: Need a way to satisfy the borrow checker here ...  Ideally I
    // would create the VNES, then whenever I want to load a ROM, I would
    // just "play", but the issue is that the compiler doesn't know that I
    // will "eject" the ROM before leaving this scope, guaranteeing that no
    // memory is accessed past the lifetime
    pub fn play(&mut self, filename: &str) -> Result<u8, std::io::Error> {
        // let mut cartridge = Cartridge::load(filename)?;
        // self.cartridge = Some(&cartridge);
        // self.cpu.insert(&cartridge);
        // self.cpu.init();
        // let ret = self.cpu.run();
        // self.cpu.exit();
        // self.cartridge = None;
        Ok(0)
    }
}
