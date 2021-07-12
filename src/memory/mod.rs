#[derive(Clone)]
pub struct RAM {
    data: Vec<u8>,
}

#[derive(Clone)]
pub struct ROM {
    data: Vec<u8>,
}

impl ROM {
    pub fn with_size(size: usize) -> Self {
        ROM {
            data: vec![0; size],
        }
    }

    pub fn with_data(data: &[u8]) -> Self {
        ROM { data: data.into() }
    }

    pub fn with_data_and_size(data: &[u8], size: usize) -> ROM {
        println!("Size of ROM: 0x{:X}", size);
        let mut rom = ROM::with_data(data);
        rom.data.extend(vec![0; size - data.len()].into_iter());
        rom
    }

    #[track_caller]
    pub fn read(&self, addr: usize) -> u8 {
        assert!(
            addr < self.data.len(),
            "{}: Read address 0x{:X} out of range of 0x{:X}",
            std::panic::Location::caller(),
            addr,
            self.data.len(),
        );
        self.data[addr]
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl RAM {
    pub fn with_size(size: usize) -> Self {
        RAM {
            data: vec![0; size],
        }
    }

    #[track_caller]
    pub fn read(&self, addr: usize) -> u8 {
        self.data[addr]
    }

    #[track_caller]
    pub fn write(&mut self, addr: usize, val: u8) {
        self.data[addr] = val;
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}
