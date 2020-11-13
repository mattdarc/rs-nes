// Sampled trait that is called to return the current value based on the
// state of a component.
pub trait Sampled {
    type OutputType;
    fn sample(&mut self) -> Self::OutputType;
}

#[derive(Debug, PartialEq)]
pub enum Frame {
    Quarter,
    Half,
    Interrupt,
    None,
}

#[derive(Clone)]
pub struct Counter {
    current: u16,
    period: u16,
    reload: bool,
    loop_flag: bool,
}

#[derive(Default, Clone)]
pub struct FrameCounter {
    counter: u8,
    period: u8,
    set_irq: bool,
}

#[derive(Default, Clone)]
pub struct LengthCounter {
    counter: Counter,
    halt: bool,
}

impl Counter {
    pub fn new(period: u16) -> Counter {
        Counter {
            current: period,
            period,
            reload: false,
            loop_flag: true,
        }
    }

    pub fn clear(&mut self) {
        self.current = 0;
    }

    pub fn set_period(&mut self, period: u16) {
        self.period = period;
    }

    pub fn get_period(&self) -> u16 {
        self.period
    }

    pub fn get_count(&self) -> u16 {
        self.current
    }

    pub fn set_reload(&mut self) {
        self.reload = true;
    }

    pub fn clear_reload(&mut self) {
        self.reload = false;
    }

    pub fn reloaded(&self) -> bool {
        self.reload
    }

    pub fn set_loop(&mut self, flag: bool) {
        self.loop_flag = flag;
    }

    pub fn has_edge(&self) -> bool {
        self.current == self.period
    }

    pub fn tick(&mut self) {
        if self.reload {
            self.current = self.period;
        } else if self.current == 0 {
            if self.loop_flag {
                self.current = self.period;
            }
        } else {
            self.current -= 1;
        }
    }
}

impl Default for Counter {
    fn default() -> Self {
        Counter::new(0)
    }
}

impl LengthCounter {
    const LUT: [u16; 32] = [
        10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48,
        20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
    ];

    pub fn new(period: u16) -> LengthCounter {
        LengthCounter {
            counter: Counter::new(period),
            halt: false,
        }
    }

    pub fn disable(&mut self) {
        self.counter.set_period(0);
        self.counter.clear();
    }

    pub fn silenced(&self) -> bool {
        self.counter.get_count() == 0
    }

    pub fn set_period(&mut self, val: u8) {
        assert!(val < 32);
        self.counter.set_period(LengthCounter::LUT[val as usize]);
        self.counter.set_reload();
    }

    pub fn clear(&mut self) {
        self.counter.clear();
    }

    pub fn set_halt(&mut self, flag: bool) {
        self.halt = flag;
    }

    pub fn has_edge(&self) -> bool {
        self.counter.has_edge()
    }

    pub fn tick(&mut self) {
        if !self.silenced() && !self.halt {
            self.counter.tick();
        }
    }
}

impl FrameCounter {
    const MODE_BIT: u8 = 7;
    const IRQ_INHIBIT: u8 = 6;

    pub fn new() -> FrameCounter {
        FrameCounter {
            counter: 0,
            period: 3,
            set_irq: false,
        }
    }

    pub fn set_control(&mut self, val: u8) -> Frame {
        let mode_flag = bit_set!(val, FrameCounter::MODE_BIT);
        match mode_flag {
            false => self.period = 3,
            true => self.period = 4,
        }
        self.set_irq = !bit_set!(val, FrameCounter::IRQ_INHIBIT);

        match mode_flag {
            false => Frame::None,
            true => Frame::Half,
        }
    }

    pub fn get_number(&mut self) -> Frame {
        eprintln!("Counter State: {}", self.counter);
        if self.period == 3 {
            match self.counter {
                0 => Frame::Quarter,
                1 => Frame::Half,
                2 => Frame::Quarter,
                3 => Frame::Interrupt,
                c => unreachable!("Invalid counter for frame counter {}!", c),
            }
        } else {
            match self.counter {
                0 => Frame::Quarter,
                1 => Frame::Half,
                2 => Frame::Quarter,
                3 => Frame::None,
                4 => Frame::Half,
                c => unreachable!("Invalid counter for frame counter {}!", c),
            }
        }
    }

    pub fn tick(&mut self) {
        if self.counter == self.period {
            self.counter = 0;
        } else {
            self.counter += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter() {
        let period = 15;
        let mut counter = Counter::new(period);
        assert_eq!(counter.get_count(), period);
        assert_eq!(counter.get_period(), period);

        let mut decr = 1;
        counter.tick();
        while !counter.has_edge() {
            assert_eq!(counter.get_count(), period - decr);
            counter.tick();
            decr += 1;
        }
        assert_eq!(decr, period + 1);
        assert_eq!(counter.get_count(), period);

        decr = 1;
        counter.set_loop(false);
        while counter.get_count() != 0 {
            counter.tick();
            assert_eq!(counter.get_count(), period - decr);
            decr += 1;
        }
        assert_eq!(decr, period + 1);

        counter.clear();
        assert_eq!(counter.get_count(), 0);

        counter.set_reload();
        assert_eq!(counter.get_count(), 0);
        counter.tick();
        assert_eq!(counter.get_count(), period);
    }

    #[test]
    fn length_counter() {
        let period = 15;
        let mut len_counter = LengthCounter::new(15);
        assert_eq!(len_counter.silenced(), false);

        len_counter.set_halt(true);
        len_counter.tick();
        assert_eq!(len_counter.counter.get_count(), period);

        len_counter.disable();
        assert_eq!(len_counter.silenced(), true);

        len_counter.tick();
        assert_eq!(len_counter.silenced(), true);
    }

    #[test]
    fn frame_counter() {
        let mut frm_ctr = FrameCounter::new();
        assert_eq!(frm_ctr.get_number(), Frame::Quarter);

        frm_ctr.tick();
        assert_eq!(frm_ctr.get_number(), Frame::Half);

        frm_ctr.tick();
        assert_eq!(frm_ctr.get_number(), Frame::Quarter);

        frm_ctr.tick();
        assert_eq!(frm_ctr.get_number(), Frame::Interrupt);

        frm_ctr.tick();
        assert_eq!(frm_ctr.get_number(), Frame::Quarter);

        // Change to the next mode (end of cycle)
        frm_ctr.set_control(1 << FrameCounter::MODE_BIT);
        assert_eq!(frm_ctr.get_number(), Frame::Quarter);

        frm_ctr.tick();
        assert_eq!(frm_ctr.get_number(), Frame::Half);

        frm_ctr.tick();
        assert_eq!(frm_ctr.get_number(), Frame::Quarter);

        frm_ctr.tick();
        assert_eq!(frm_ctr.get_number(), Frame::None);

        frm_ctr.tick();
        assert_eq!(frm_ctr.get_number(), Frame::Half);
    }
}
