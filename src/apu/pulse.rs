use crate::apu::counter::{Counter, LengthCounter};
use crate::apu::volume::Envelope;
use crate::common::*;

#[derive(Clone)]
enum Arithmetic {
    OnesCompliment,
    TwosCompliment,
}

impl Default for Arithmetic {
    fn default() -> Arithmetic {
        Arithmetic::TwosCompliment
    }
}

#[derive(Default, Clone)]
pub struct Pulse {
    timer: Counter,
    envelope: Envelope,
    sweep: Sweep,
    sequencer: PulseSequencer,
    length_counter: LengthCounter,
    arithmetic_type: Arithmetic,
    enabled: bool,
}

#[derive(Default, Clone)]
struct Sweep {
    divider: Counter,
    enabled: bool,
    negate: bool,
    shift: u8,
}

#[derive(Default, Clone)]
struct PulseSequencer {
    idx: usize,
    seq: &'static [u8],
}

impl Sweep {
    fn new() -> Sweep {
        Sweep {
            divider: Counter::new(0),
            enabled: true,
            negate: false,
            shift: 0,
        }
    }

    fn load(&mut self, val: u8) {
        self.enabled = (val & 0x80) != 0;
        self.divider.set_period(((val >> 4) & 0x7) as u16);
        self.negate = (val & 0x8) != 0;
        self.shift = val & 0x70
    }

    fn update_tgt_period(&mut self, tmr: u16, offset: u16) -> u16 {
        self.divider.clock();
        if self.divider.has_edge() && self.enabled {
            let change = tmr >> self.shift;
            if self.negate {
                tmr - change - offset
            } else {
                tmr + change
            }
        } else {
            0
        }
    }

    fn reset(&mut self) {
        self.divider.set_reload();
    }
}

impl PulseSequencer {
    const SEQ_LEN: usize = 8;
    const SEQ_1: [u8; PulseSequencer::SEQ_LEN] = [0, 1, 0, 0, 0, 0, 0, 0];
    const SEQ_2: [u8; PulseSequencer::SEQ_LEN] = [0, 1, 1, 0, 0, 0, 0, 0];
    const SEQ_3: [u8; PulseSequencer::SEQ_LEN] = [0, 1, 1, 1, 1, 0, 0, 0];
    const SEQ_4: [u8; PulseSequencer::SEQ_LEN] = [1, 0, 0, 1, 1, 1, 1, 1];

    fn new() -> PulseSequencer {
        PulseSequencer {
            idx: 0,
            seq: &PulseSequencer::SEQ_1,
        }
    }

    fn reset(&mut self) {
        self.idx = 0;
    }

    fn set_duty_cycle(&mut self, duty: u8) {
        self.idx = 0;
        self.seq = match duty {
            1 => &PulseSequencer::SEQ_1,
            2 => &PulseSequencer::SEQ_2,
            3 => &PulseSequencer::SEQ_3,
            4 => &PulseSequencer::SEQ_4,
            _ => unreachable!("Invalid duty cycle {}!", duty),
        }
    }
}

impl Clocked for PulseSequencer {
    fn clock(&mut self) {
        self.idx = (self.idx + 1) % PulseSequencer::SEQ_LEN;
    }
}

impl Sampled for PulseSequencer {
    type OutputType = u16;
    fn sample(&mut self) -> Self::OutputType {
        self.seq[self.idx] as u16
    }
}

impl Sampled for Pulse {
    type OutputType = u16;
    fn sample(&mut self) -> Self::OutputType {
        return if !self.silenced() {
            (self.envelope.sample() * self.sequencer.sample()).into()
        } else {
            0
        };
    }
}

impl Clocked for Pulse {
    fn clock(&mut self) {
        self.sequencer.clock();
        self.timer.clock();
    }
}

impl Pulse {
    pub fn ones_complement() -> Pulse {
        Pulse {
            timer: Counter::new(0),
            envelope: Envelope::new(),
            sweep: Sweep::new(),
            sequencer: PulseSequencer::new(),
            length_counter: LengthCounter::new(0),
            arithmetic_type: Arithmetic::OnesCompliment,
            enabled: true,
        }
    }

    pub fn twos_complement() -> Pulse {
        Pulse {
            timer: Counter::new(0),
            envelope: Envelope::new(),
            sweep: Sweep::new(),
            sequencer: PulseSequencer::new(),
            length_counter: LengthCounter::new(0),
            arithmetic_type: Arithmetic::TwosCompliment,
            enabled: true,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter.disable();
        }
    }

    pub fn set_volume_grp(&mut self, val: u8) {
        self.sequencer.set_duty_cycle(val >> 6);
        self.envelope.set_divider((val & 0xF) as u16);

        let halt_and_loop = (val & 0x20) != 0;
        self.length_counter.set_halt(halt_and_loop);
        self.envelope.set_loop(halt_and_loop);
        self.envelope.set_constant((val & 0x10) != 0);
    }

    pub fn set_sweep(&mut self, val: u8) {
        self.sweep.load(val);
    }

    pub fn set_timer_low_grp(&mut self, val: u8) {
        let high = self.timer.get_period() & 0xFF00;
        self.timer.set_period(high | (val as u16));
    }

    pub fn set_length_counter_grp(&mut self, val: u8) {
        let low = self.timer.get_period() & 0xFF;
        self.timer.set_period(((val & 0x7) as u16) << 8 | low);
        self.length_counter.set_period(val >> 3);
        self.envelope.reset();
        self.sequencer.reset();
    }

    pub fn silenced(&self) -> bool {
        self.timer.get_period() < 8
            || self.length_counter.silenced()
            || self.timer.get_period() > 0x7FF
    }

    fn clock_sweep(&mut self) {
        let period = self.timer.get_period();
        let offset = match self.arithmetic_type {
            Arithmetic::OnesCompliment => 1,
            Arithmetic::TwosCompliment => 0,
        };
        self.timer
            .set_period(self.sweep.update_tgt_period(period, offset));
    }

    pub fn quarter_frame(&mut self) {
        self.envelope.clock();
    }

    pub fn half_frame(&mut self) {
        self.length_counter.clock();
        self.clock_sweep();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pulse() {
        // TODO
    }
}
