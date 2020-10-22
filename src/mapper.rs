// The mapper controls read/write to and from memory. A catridge should have a mapper and memory,
// then the memory should only be accessed using the mapper. The mapper defines where the RAM
// ROM PPU APU all are in memory AFAIK, and defines the mirroring

use crate::common::*;
use crate::memory::*;

use std::fmt;

pub trait Mapper {
    fn get_num(&self) -> u8;
    fn box_clone(&self) -> Box<dyn Mapper>;

    // PRG
    fn prg_read(&self, addr: usize) -> u8;
    fn prg_write(&mut self, addr: usize, val: u8);
    fn prg_size(&self) -> usize;

    // CHR
    fn chr_read(&self, addr: usize) -> u8;
    fn chr_write(&mut self, addr: usize, val: u8);
    fn chr_size(&self) -> usize;
}

#[derive(Clone, Debug)]
pub struct Header {
    // Byte 6
    prg_rom_size: u8,
    chr_ram_size: u8,
    ignore_mirror_ctrl: bool,
    has_trainer: bool,
    has_persistent_mem: bool,
    mirroring: Mirroring,

    // Byte 7
    mapper_num: u8,
    format: ROMFormat,
    prg_ram_size: u8,
}

impl Clone for Box<dyn Mapper> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

impl fmt::Debug for Box<dyn Mapper> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mapper")
            .field("mapper_num", &self.get_num())
            .field("prg_size", &self.prg_size())
            .field("chr_size", &self.chr_size())
            .finish()
    }
}

impl Default for Box<dyn Mapper> {
    fn default() -> Box<dyn Mapper> {
        Box::new(Mapper0::default())
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Mirroring {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug)]
enum ROMFormat {
    INES,
    NES20,
}

impl Header {
    pub fn get_prg_rom_size(&self) -> usize {
        const UNIT: usize = 16 * 1024; // 16 KB
        self.prg_rom_size as usize * UNIT
    }

    pub fn get_chr_ram_size(&self) -> usize {
        const UNIT: usize = 8 * 1024; // 8 KB
        self.chr_ram_size as usize * UNIT
    }

    pub fn get_prg_ram_size(&self) -> usize {
        const UNIT: usize = 8 * 1024; // 8 KB
        std::cmp::max(self.prg_ram_size as usize * UNIT, UNIT)
    }

    pub fn get_mapper_num(&self) -> u8 {
        self.mapper_num
    }
}

impl std::convert::From<&[u8; 16]> for Header {
    fn from(header: &[u8; 16]) -> Self {
        // https://wiki.nesdev.com/w/index.php/INES
        //  0-3: Constant $4E $45 $53 $1A ("NES" followed by MS-DOS end-of-file)
        // 4: Size of PRG ROM in 16 KB units
        // 5: Size of CHR ROM in 8 KB units (Value 0 means the board uses CHR RAM)
        // 6: Flags 6 - Mapper, mirroring, battery, trainer
        // 7: Flags 7 - Mapper, VS/Playchoice, NES 2.0
        // 8: Flags 8 - PRG-RAM size (rarely used extension)
        // 9: Flags 9 - TV system (rarely used extension)
        // 10: Flags 10 - TV system, PRG-RAM presence (unofficial, rarely used extension)
        // 11-15: Unused padding (should be filled with zero, but some rippers put their name across bytes 7-15)
        let prg_rom_size = header[4];
        let chr_ram_size = header[5];

        let flags_6 = &header[6];
        let ignore_mirror_ctrl = (0x8 & flags_6) != 0;
        let has_trainer = (0x4 & flags_6) != 0;
        let has_persistent_mem = (0x2 & flags_6) != 0;
        let mirroring = match (0x1 & flags_6) != 0 {
            true => Mirroring::Vertical,
            false => Mirroring::Horizontal,
        };

        let flags_7 = &header[7];
        let mapper_num = (flags_7 & 0xF0) | (flags_6 >> 4);
        let format = match (flags_7 >> 2) & 0x3 {
            2 => ROMFormat::NES20,
            _ => ROMFormat::INES,
        };

        let prg_ram_size = std::cmp::max(1, header[8]);
        Header {
            prg_rom_size,
            chr_ram_size,
            ignore_mirror_ctrl,
            has_trainer,
            has_persistent_mem,
            mirroring,
            mapper_num,
            format,
            prg_ram_size,
        }
    }
}

impl Default for Header {
    fn default() -> Self {
        Header {
            prg_rom_size: 2,
            chr_ram_size: 0,
            ignore_mirror_ctrl: true,
            has_trainer: false,
            has_persistent_mem: false,
            mirroring: Mirroring::Vertical,
            mapper_num: 0,
            format: ROMFormat::NES20,
            prg_ram_size: 1,
        }
    }
}

#[derive(Default, Debug, Clone)]
struct Mapper0 {
    prg_rom: ROM, // for CPU
    prg_ram: RAM, // for CPU
    chr_ram: RAM, // for PPU, "most emulators support ram"
}

impl Mapper0 {
    const ROM_START: u16 = 0x8000;

    fn new(header: &Header, data: &[u8]) -> Self {
        Mapper0 {
            prg_rom: ROM::with_data_and_size(
                data,
                header.get_prg_rom_size(),
            ),
            prg_ram: RAM::new(header.get_prg_ram_size()),
            chr_ram: RAM::new(header.get_chr_ram_size()),
        }
    }
}

#[derive(Default, Debug, Clone)]
struct Mapper1 {
    prg_rom: ROM, // for CPU
    prg_ram: RAM, // for CPU
    chr_ram: RAM, // for PPU, "most emulators support ram"
}

impl Mapper1 {
    const ROM_START: u16 = 0x8000;

    fn new(header: &Header, data: &[u8]) -> Self {
        Mapper1 {
            chr_ram: RAM::new(header.get_chr_ram_size()),
            prg_ram: RAM::new(header.get_prg_ram_size()),
            prg_rom: ROM::with_data_and_size(
                data,
                header.get_prg_rom_size(),
            ),
        }
    }
}

impl Mapper for Mapper1 {
    fn get_num(&self) -> u8 {
        1
    }

    fn box_clone(&self) -> Box<dyn Mapper> {
        Box::new(self.clone())
    }

    // PRG
    fn prg_read(&self, addr: usize) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram.read(addr - 0x6000),
            0x8000..=0xFFFF => self.prg_rom.read(addr - 0x8000),
            _ => unreachable!("Invalid read of address {:#X}!", addr),
        }
    }

    fn prg_write(&mut self, addr: usize, val: u8) {
        match addr {
            0x6000..=0x7FFF => self.prg_ram.write(addr - 0x6000, val),
            0x8000..=0xFFFF => {
                unreachable!("Tried to write to ROM at address {}!", addr)
            }
            _ => unreachable!("Invalid read of address {}!", addr),
        };
    }

    fn prg_size(&self) -> usize {
        self.prg_rom.len() + self.prg_ram.len()
    }

    // CHR
    fn chr_read(&self, addr: usize) -> u8 {
        self.chr_ram.read(addr)
    }

    fn chr_write(&mut self, addr: usize, val: u8) {
        self.chr_ram.write(addr, val)
    }

    fn chr_size(&self) -> usize {
        self.chr_ram.len()
    }
}

impl Mapper for Mapper0 {
    fn get_num(&self) -> u8 {
        0
    }

    fn box_clone(&self) -> Box<dyn Mapper> {
        Box::new(self.clone())
    }

    // PRG
    fn prg_read(&self, addr: usize) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram.read(addr - 0x6000),
            0x8000..=0xFFFF => self.prg_rom.read(addr - 0x8000),
            _ => unreachable!("Invalid read of address {}!", addr),
        }
    }

    fn prg_write(&mut self, addr: usize, val: u8) {
        match addr {
            0x6000..=0x7FFF => self.prg_ram.read(addr - 0x6000),
            0x8000..=0xFFFF => unreachable!("Tried to overwrite ROM!"),
            _ => unreachable!("Invalid read of address {}!", addr),
        };
    }

    fn prg_size(&self) -> usize {
        self.prg_rom.len() + self.prg_ram.len()
    }

    // CHR
    fn chr_read(&self, addr: usize) -> u8 {
        self.chr_ram.read(addr)
    }

    fn chr_write(&mut self, addr: usize, val: u8) {
        self.chr_ram.write(addr, val)
    }

    fn chr_size(&self) -> usize {
        self.chr_ram.len()
    }
}

pub fn create_mapper(header: &Header, data: &[u8]) -> Box<dyn Mapper> {
    match header.get_mapper_num() {
        0 => Box::new(Mapper0::new(header, data)),
        1 => Box::new(Mapper1::new(header, data)),
        n => unreachable!("Unimplemented mapper {}!", n),
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    // Create a test mapper, setting the reset vector to the first instruction
    pub fn mapper_with(data: &[u8]) -> Box<dyn Mapper> {
        let header = Header::default();
        let rom_start = match header.get_mapper_num() {
            0 => Mapper0::ROM_START,
            1 => Mapper1::ROM_START,
            n => unreachable!("Unimplemented mapper {}!", n),
        };
        let mut rom = vec![0; header.get_prg_rom_size()];

        println!("-- Cloning data {:?} into ROM", data);
        rom[0..data.len()].clone_from_slice(data);
        let low = (rom_start & 0xFF) as u8;
        let high = (rom_start >> 8) as u8;
        let reset_loc = (RESET_VECTOR_START - rom_start) as usize;

        println!(
            "-- Initializing ROM reset vector at {:#X} to {:#X}",
            reset_loc, rom_start
        );
        rom[reset_loc] = low;
        rom[reset_loc + 1] = high;
        super::create_mapper(&header, rom.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header() {
        const HEADER_RAW: [u8; 16] = [
            0x4e, 0x45, 0x53, 0x1a, 0x10, 0x12, 0x11, 0x00, 0x13, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let header = Header::from(&HEADER_RAW);
        assert_eq!(header.mirroring, Mirroring::Vertical);
        assert_eq!(header.prg_rom_size, 0x10);
        assert_eq!(header.chr_ram_size, 0x12);
        assert_eq!(header.prg_ram_size, 0x13);
        assert_eq!(header.get_mapper_num(), 0x1);
    }
}
