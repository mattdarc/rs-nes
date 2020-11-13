use crate::common::*;

#[derive(Default, Clone)]
pub struct DMC {
    rate_index: u16,
    output_level: u8,
    irq_enabled: bool,
    loop_flag: bool,
    enabled: bool,
}

impl DMC {
    const LUT_NTSC: [u16; 16] = [
        428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
    ];
    const LUT_PAL: [u16; 16] = [
        398, 354, 316, 298, 276, 236, 210, 198, 176, 148, 132, 118, 98, 78, 66, 50,
    ];

    pub fn new() -> DMC {
        DMC {
            output_level: 0,
            rate_index: 0,
            irq_enabled: false,
            loop_flag: true,
            enabled: true,
        }
    }

    pub fn set_control_grp(&mut self, val: u8) {
        self.irq_enabled = (val & 0x80) != 0;
        self.loop_flag = (val & 0x40) != 0;
        self.rate_index = DMC::LUT_NTSC[(val & 0xF) as usize];
    }

    pub fn direct_load(&mut self, val: u8) {
        self.output_level = val & 0x7F;
    }

    pub fn set_sample_address(&mut self, val: u8) {
        let addr = 0xC000 | ((val as u16) << 6);
    }

    pub fn set_sample_length(&mut self, val: u8) {
        let len = ((val as u16) << 4) + 1;
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn quarter_frame(&mut self) {}

    pub fn half_frame(&mut self) {}
}

impl Clocked for DMC {
    fn clock(&mut self) {}
}

impl Sampled for DMC {
    type OutputType = u16;
    fn sample(&mut self) -> Self::OutputType {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dmc() {
        // TODO
    }
}
