use crate::apu::counter::Counter;
use crate::common::{Clocked, Sampled};

#[derive(Default, Clone)]
pub struct Envelope {
    decay: Counter,
    divider: Counter,
    start: bool,
    is_constant: bool,
}

impl Envelope {
    const DECAY_PERIOD: u16 = 15;

    pub fn new() -> Envelope {
        Envelope::default()
    }

    pub fn set_constant(&mut self, is_constant: bool) {
        self.is_constant = is_constant;
    }

    pub fn set_loop(&mut self, flag: bool) {
        self.decay.set_loop(flag);
    }

    pub fn set_divider(&mut self, period: u16) {
        self.divider.set_period(period);
    }

    pub fn reset(&mut self) {
        self.divider.set_reload();
    }
}

impl Clocked for Envelope {
    fn clock(&mut self) {
        if !self.start {
            self.divider.clock();
            if self.divider.has_edge() {
                self.decay.clock();
            }
        } else {
            self.start = false;
            self.decay.set_reload();
            self.divider.set_reload();
        }
    }
}

impl Sampled for Envelope {
    type OutputType = u16;
    fn sample(&self) -> Self::OutputType {
        if self.is_constant {
            self.divider.get_period()
        } else {
            self.decay.get_count()
        }
    }
}
