#![allow(non_upper_case_globals)]
use std::ops::{Deref, DerefMut};

pub struct Memory<const ReadOnly: bool>(Vec<u8>);
pub type ROM = Memory<true>;
pub type RAM = Memory<false>;

impl<const ReadOnly: bool> Memory<ReadOnly> {
    pub fn with_size(size: usize) -> Self {
        Memory(vec![0; size])
    }

    pub fn with_data(data: &[u8]) -> Self {
        Memory(data.into())
    }

    pub fn with_data_and_size(data: &[u8], size: usize) -> Self {
        let mut memory = vec![0_u8; size];
        memory.resize(size, 0);
        memory.copy_from_slice(&data[0..(size as usize).min(data.len())]);

        Memory(memory)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<const ReadOnly: bool> Deref for Memory<ReadOnly> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl DerefMut for RAM {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut_slice()
    }
}

impl DerefMut for ROM {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unreachable!("Cannot write to ROM")
    }
}
