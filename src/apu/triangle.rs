use crate::apu::counter::{Counter, LengthCounter, Sampled};

#[derive(Default, Clone)]
pub struct Triangle {
    timer: Counter,
    linear_counter: Counter,
    length_counter: LengthCounter,
    sequencer: TriangleSequencer,
    ctrl_flag: bool,
    enabled: bool,
}

#[derive(Default, Clone)]
struct TriangleSequencer {
    current: u16,
}

impl TriangleSequencer {
    const LUT: [u8; 32] = [
        15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8,
        9, 10, 11, 12, 13, 14, 15,
    ];

    fn new() -> TriangleSequencer {
        TriangleSequencer { current: 0 }
    }

    fn tick(&mut self) {
        self.current = (self.current + 1) % TriangleSequencer::LUT.len() as u16;
    }
}

impl Triangle {
    pub fn new() -> Triangle {
        Triangle {
            timer: Counter::new(0),
            linear_counter: Counter::new(0),
            length_counter: LengthCounter::new(0),
            sequencer: TriangleSequencer::new(),
            ctrl_flag: true,
            enabled: true,
        }
    }

    pub fn tick_linear_counter(&mut self) {
        if !self.linear_counter.reloaded() {
            self.linear_counter.tick();
        }

        if !self.ctrl_flag {
            self.linear_counter.clear_reload();
        }
    }

    pub fn tick_sequencer(&mut self) {
        if !self.silenced() {
            self.sequencer.tick();
        }
    }

    pub fn silenced(&self) -> bool {
        self.linear_counter.get_count() == 0 || self.length_counter.silenced()
    }

    pub fn set_linear_counter_grp(&mut self, val: u8) {
        self.ctrl_flag = (val & 0x80) != 0;
        self.length_counter.set_halt(self.ctrl_flag);
        self.linear_counter.set_period((val & 0x7F) as u16);
    }

    pub fn set_timer_low_grp(&mut self, val: u8) {
        let high = self.timer.get_period() & 0xFF00;
        self.timer.set_period(high | (val as u16));
    }

    pub fn set_length_counter_grp(&mut self, val: u8) {
        let low = self.timer.get_period() & 0xFF;
        self.timer.set_period(((val & 0x7) as u16) << 8 | low);
        self.length_counter.set_period(val >> 3);
        self.linear_counter.set_reload();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter.disable();
        }
    }

    pub fn quarter_frame(&mut self) {
        self.linear_counter.tick();
    }

    pub fn half_frame(&mut self) {
        self.length_counter.tick();
    }

    pub fn tick(&mut self) {
        self.timer.tick();
    }
}

impl Sampled for Triangle {
    type OutputType = u16;
    fn sample(&mut self) -> Self::OutputType {
        if !self.silenced() {
            self.sequencer.sample()
        } else {
            0
        }
    }
}

impl Sampled for TriangleSequencer {
    type OutputType = u16;
    fn sample(&mut self) -> Self::OutputType {
        TriangleSequencer::LUT[self.current as usize] as u16
    }
}
