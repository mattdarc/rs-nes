// Bus trait that models the communication on the bus. An object is passed an instance of this type
// when clocked
pub trait Bus {
    fn read(&mut self, addr: usize) -> u8;
    fn write(&mut self, addr: usize, val: u8);

    fn read_n(&mut self, addr: usize, n: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(n);
        for idx in 0..n {
            v.push(self.read(addr + idx));
        }
        v
    }

    fn read16(&mut self, addr: usize) -> u16 {
        (self.read(addr) as u16) | ((self.read(addr + 1) as u16) << 8)
    }
}

// Clocked trait that is called as the entry point of execution of the
// component.
pub trait Clocked<BusType: Bus> {
    fn clock(&mut self, bus: &mut BusType);
}

// Snapshot trait that implements save and restore. An object that can be
// saved has its internal state saved, which can then be restored at a
// later time. The medium can be any time.
pub trait Snapshot {
    type Medium;

    // Save the internal state of the object to the Medium
    // Returns true on success, false on failure
    fn save(&self, medium: Self::Medium) -> bool;

    // Restore the internal state of the object from the Medium
    // Returns true on success, false on failure
    fn restore(&mut self, medium: Self::Medium) -> bool;
}

#[macro_export]
macro_rules! ternary {
    ($cond:expr; $a:expr, $b:expr) => {
        if $cond {
            $a
        } else {
            $b
        }
    };
}

#[macro_export]
macro_rules! set_status {
    ($($v:expr),*) => {
	(0 $(| (1 << $v))*).into()
    }
}

#[macro_export]
macro_rules! bit_set {
    ($value:expr, $bit:expr) => {
        ($value & (1 << $bit)) != 0
    };
}

pub const NTSC_CLOCK: u32 = 1_789_773;
pub const PAL_CLOCK: u32 = 1_662_607;
pub const RESET_VECTOR_START: u16 = 0xFFFC;
