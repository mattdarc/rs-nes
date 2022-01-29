#[derive(Clone)]
pub struct RAM {
    data: Vec<u8>,
}

#[derive(Clone)]
pub struct ROM {
    data: Vec<u8>,
}

impl ROM {
    pub fn with_size(size: u16) -> Self {
        ROM {
            data: vec![0; size as usize],
        }
    }

    pub fn with_data(data: &[u8]) -> Self {
        ROM { data: data.into() }
    }

    pub fn with_data_and_size(data: &[u8], size: u16) -> ROM {
        println!("Size of ROM: {:X}", size);
        let mut rom = ROM::with_size(size);
        rom.data
            .copy_from_slice(&data[0..(size as usize).min(data.len())]);
        rom
    }

    #[track_caller]
    pub fn read(&self, addr: u16) -> u8 {
        assert!(
            addr < self.len(),
            "{}: Read address 0x{:X} out of range of 0x{:X}",
            std::panic::Location::caller(),
            addr,
            self.data.len(),
        );
        self.data[addr as usize]
    }

    pub fn len(&self) -> u16 {
        self.data.len() as u16
    }
}

impl RAM {
    pub fn with_size(size: u16) -> Self {
        RAM {
            data: vec![0; size as usize],
        }
    }

    pub fn with_data(data: &[u8]) -> Self {
        RAM { data: data.into() }
    }

    pub fn with_data_and_size(data: &[u8], size: u16) -> RAM {
        println!("Size of DATA vs. RAM: {:X} vs. {:X}", data.len(), size);
        let mut ram = RAM::with_data(data);
        ram.data
            .extend(vec![0; size as usize - data.len()].into_iter());
        ram
    }

    #[track_caller]
    pub fn read(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    #[track_caller]
    pub fn write(&mut self, addr: u16, val: u8) {
        self.data[addr as usize] = val;
    }

    pub fn len(&self) -> u16 {
        self.data.len() as u16
    }
}
