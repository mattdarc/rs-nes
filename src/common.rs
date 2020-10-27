
// Clocked trait that is called as the entry point of execution of the
// component.  Returns true if a timer edge was hit, and the component was
// actually executed
pub trait Clocked {
    fn clock(&mut self);
}

// Sampled trait that is called to return the current value based on the
// state of a component.
pub trait Sampled {
    type OutputType;
    fn sample(&self) -> Self::OutputType;
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

// Memory trait that defines a segment of readable memory
pub trait Readable {
    // Read from a segment of memory
    fn read(&self, idx: usize) -> u8;
}

// Memory trait that defines a segment of writeable memory
pub trait Writeable {
    // Write to a segment of memory
    fn write(&mut self, idx: usize, val: u8);
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
