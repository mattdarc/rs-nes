use crate::common::*;

#[derive(Debug, Clone, Default)]
pub struct ROM {
    data: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct RAM {
    data: Vec<u8>,
}

impl ROM {
    pub fn new() -> ROM {
        ROM { data: Vec::new() }
    }

    pub fn with_data_and_size(data: &[u8], size: usize) -> ROM {
        println!("-- Creating ROM of size {}", size);
        let mut rom = vec![0; size];
        for (i, &b) in data.iter().enumerate() {
            rom[i] = b;
        }
        ROM { data: rom }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Readable for ROM {
    fn read(&self, idx: usize) -> u8 {
        self.data[idx]
    }
}

impl RAM {
    pub fn new(size: usize) -> RAM {
        RAM {
            data: vec![0; size],
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Readable for RAM {
    fn read(&self, idx: usize) -> u8 {
        self.data[idx]
    }
}

impl Writeable for RAM {
    fn write(&mut self, idx: usize, val: u8) {
        self.data[idx] = val;
    }
}
