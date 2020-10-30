use crate::apu::counter::*;
use crate::apu::volume::*;
use crate::common::*;

#[derive(Default, Clone)]
pub struct Noise {
    timer: Counter,
    length_counter: LengthCounter,
    lfsr: LFShiftRegister,
    envelope: Envelope,
    enabled: bool,
}

#[derive(Default, Clone)]
struct LFShiftRegister {
    value: u16,
    mode_flag: bool,
}

impl Noise {
    const LUT_NTSC: [u16; 16] = [
        4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
    ];
    const LUT_PAL: [u16; 16] = [
        4, 8, 14, 30, 60, 88, 118, 148, 188, 236, 354, 472, 708, 944, 1890, 3778,
    ];

    pub fn new() -> Noise {
        Noise {
            timer: Counter::new(0),
            length_counter: LengthCounter::new(0),
            lfsr: LFShiftRegister::new(),
            envelope: Envelope::new(),
            enabled: false,
        }
    }

    fn silenced(&self) -> bool {
        self.lfsr.lsb_set() || self.length_counter.silenced()
    }

    pub fn set_volume_grp(&mut self, val: u8) {
        self.envelope.set_divider((val & 0xF) as u16);
        self.length_counter.set_halt((val & 0x20) != 0);
        self.envelope.set_constant(bit_set!(val, 5));
    }

    pub fn set_mode_grp(&mut self, val: u8) {
        self.lfsr.set_mode((val & 0x80) != 0);
        self.timer
            .set_period(Noise::LUT_NTSC[(val & 0xF) as usize].into());
    }

    pub fn set_length_counter_grp(&mut self, val: u8) {
        if self.enabled {
            self.length_counter.set_period(val >> 3);
            self.envelope.reset();
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter.disable();
        }
    }

    pub fn quarter_frame(&mut self) {
        self.envelope.clock();
    }

    pub fn half_frame(&mut self) {
        self.length_counter.clock();
    }
}

impl LFShiftRegister {
    fn new() -> LFShiftRegister {
        LFShiftRegister {
            value: 1,
            mode_flag: false,
        }
    }

    fn set_mode(&mut self, mode: bool) {
        self.mode_flag = mode;
    }

    fn lsb_set(&self) -> bool {
        (self.value & 0x1) != 0
    }
}

impl Clocked for Noise {
    fn clock(&mut self) {
        if self.timer.has_edge() {
            self.lfsr.clock();
        }
    }
}

impl Sampled for Noise {
    type OutputType = u16;
    fn sample(&self) -> Self::OutputType {
        if !self.silenced() {
            self.envelope.sample()
        } else {
            0
        }
    }
}

impl Clocked for LFShiftRegister {
    fn clock(&mut self) {
        let shift = ternary!(self.mode_flag; 6, 1);
        let feedback = ((self.value >> shift) & 0x1) ^ (self.value & 0x1);
        self.value = ((self.value >> 1) & 0x4FFF) | (feedback << 14);
    }
}

impl Sampled for LFShiftRegister {
    type OutputType = u16;
    fn sample(&self) -> Self::OutputType {
        self.value & 0x1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dmc() {
        // TODO
    }
}
