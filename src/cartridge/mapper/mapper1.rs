use super::*;
use crate::memory::*;

pub struct Mapper1 {
    prg_rom: ROM, // for CPU
    prg_ram: RAM, // for CPU
    chr_ram: RAM, // for PPU, "most emulators support ram"
}

impl Mapper1 {
    pub fn new(header: &Header, data: &[u8]) -> Self {
        let (prg, chr) = data.split_at(header.get_prg_rom_size() as usize);
        Mapper1 {
            prg_ram: RAM::with_size(header.get_prg_ram_size()),
            prg_rom: ROM::with_data_and_size(prg, header.get_prg_rom_size()),
            chr_ram: RAM::with_data_and_size(chr, header.get_chr_ram_size()),
        }
    }
}

impl Mapper for Mapper1 {
    fn number(&self) -> u8 {
        1
    }

    fn prg_read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        match addr {
            0x6000..=0x7FFF => self.prg_ram[addr - 0x6000],
            0x8000..=0xFFFF => self.prg_rom[addr - 0x8000],
            _ => unknown_address(addr),
        }
    }

    fn prg_write(&mut self, addr: u16, val: u8) {
        let addr = addr as usize;
        match addr {
            0x6000..=0x7FFF => self.prg_ram[addr - 0x6000] = val,
            0x8000..=0xFFFF => self.prg_rom[addr - 0x8000] = val,
            _ => unknown_address(addr),
        };
    }

    fn dpcm(&self) -> ROM {
        ROM::with_data(self.map_range(0xC000, 0xFFF1 - 0xC000))
    }

    fn chr(&self) -> ROM {
        ROM::with_data(&self.chr_ram)
    }
}

impl Mapper1 {
    fn map_range(&self, base: usize, len: usize) -> &[u8] {
        assert!((base & 0xFFFF) == base);
        assert!(len > 0);

        match base {
            0x6000..=0x7FFF => {
                let offset = base - 0x6000;
                assert!(offset + len < self.prg_ram.len());

                &self.prg_ram[offset..(offset + len)]
            }
            0x8000..=0xFFFF => {
                let offset = base - 0x8000;
                assert!(offset + len < self.prg_rom.len());

                &self.prg_rom[offset..(offset + len)]
            }
            _ => unknown_address(base),
        }
    }
}
