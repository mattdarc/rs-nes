#[derive(Clone, Debug, PartialEq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug)]
pub enum ROMFormat {
    INES,
    NES20,
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

impl Header {
    pub fn get_prg_rom_size(&self) -> u16 {
        const UNIT: u16 = 16 * 1024; // 16 KB
        self.prg_rom_size as u16 * UNIT
    }

    pub fn get_chr_ram_size(&self) -> u16 {
        const UNIT: u16 = 8 * 1024; // 8 KB
        self.chr_ram_size as u16 * UNIT
    }

    pub fn get_prg_ram_size(&self) -> u16 {
        const UNIT: u16 = 8 * 1024; // 8 KB
        std::cmp::max(self.prg_ram_size as u16 * UNIT, UNIT)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header() {
        const HEADER_RAW: [u8; 16] = [
            0x4e, 0x45, 0x53, 0x1a, 0x10, 0x12, 0x11, 0x00, 0x13, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];

        let header = Header::from(&HEADER_RAW);
        assert_eq!(header.mirroring, Mirroring::Vertical);
        assert_eq!(header.prg_rom_size, 0x10);
        assert_eq!(header.chr_ram_size, 0x12);
        assert_eq!(header.prg_ram_size, 0x13);
        assert_eq!(header.get_mapper_num(), 0x1);
    }
}
