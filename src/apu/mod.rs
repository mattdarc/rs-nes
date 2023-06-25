use crate::cartridge::{Cartridge, CartridgeInterface};
use crate::memory::ROM;
use tracing::{event, Level};

struct ApuStatus;

impl ApuStatus {
    const W_DMC_ENABLE: u8 = 0x10;
    const W_NOISE_ENABLE: u8 = 0x8;
    const W_TRIANGLE_ENABLE: u8 = 0x4;
    const W_PULSE1_ENABLE: u8 = 0x2;
    const W_PULSE2_ENABLE: u8 = 0x1;

    const R_DMC_IRQ: u8 = 0x80;
    const R_FRAME_IRQ: u8 = 0x40;
    const R_DMC_ACTIVE: u8 = 0x10;
    const R_NOISE_ACTIVE: u8 = 0x80;
    const R_TRIANGLE_ACTIVE: u8 = 0x80;
    const R_PULSE1_ACTIVE: u8 = 0x80;
    const R_PULSE2_ACTIVE: u8 = 0x80;
}

pub struct APU {
    pulse_1: Pulse,
    pulse_2: Pulse,
    triangle: Triangle,
    noise: Noise,
    dmc: Dmc,
}

impl APU {
    pub fn new(game: &Cartridge) -> Self {
        APU {
            pulse_1: Pulse::default(),
            pulse_2: Pulse::default(),
            triangle: Triangle::default(),
            noise: Noise::default(),
            dmc: Dmc::new(game.dpcm()),
        }
    }

    pub fn register_read(&mut self, addr: u16) -> u8 {
        let ret = match addr {
            0x0..0x4 => self.pulse_1.register_read(addr),
            0x4..0x8 => self.pulse_2.register_read(addr - 0x4),
            0x8..0xC => self.triangle.register_read(addr - 0x8),
            0xC..0x10 => self.noise.register_read(addr - 0xC),
            0x10..0x14 => self.dmc.register_read(addr - 0x10),
            0x14 => {
                event!(Level::DEBUG, "apu::register_read ignored ({:#X})", addr);
                0xFF
            }
            0x15 => self.status_read(),
            _ => unreachable!("Invalid read {:#X}", addr),
        };

        event!(
            Level::DEBUG,
            "apu::register_read [{:#x}] (== {:#x})",
            addr,
            ret,
        );

        ret
    }

    pub fn register_write(&mut self, addr: u16, val: u8) {
        event!(
            Level::DEBUG,
            "apu::register_write [{:#x}] (== {:#x})",
            addr,
            val,
        );

        match addr {
            0x0..0x4 => self.pulse_1.register_write(addr, val),
            0x4..0x8 => self.pulse_2.register_write(addr - 0x4, val),
            0x8..0xC => self.triangle.register_write(addr - 0x8, val),
            0xC..0x10 => self.noise.register_write(addr - 0xC, val),
            0x10..0x14 => self.dmc.register_write(addr - 0x10, val),
            0x14 => event!(
                Level::DEBUG,
                "apu::register_write ignored ({:#X}, ={:#X})",
                addr,
                val
            ),
            0x15 => self.status_write(val),
            _ => unreachable!("Invalid write {:#X}", addr),
        }
    }

    pub fn irq_raised(&self) -> bool {
        self.dmc.irq_raised
    }

    fn status_read(&self) -> u8 {
        let mut status = 0;
        if self.dmc.irq_en {
            status |= ApuStatus::R_DMC_IRQ
        }
        // FIXME:

        status
    }

    fn status_write(&self, val: u8) {}
}

struct Dmc {
    irq_en: bool,
    irq_raised: bool,
    dmc_loop: bool,
    silence: bool,
    rate_index: u8,
    output_counter: u8,
    current_output: u8,
    sample_addr: usize,
    current_addr: usize,
    sample_len: u16,
    bytes_remaining: u16,
    bits_remaining: u16,
    sample_shift_reg: u8,
    cycles_this_sample: u16,

    samples: ROM,
}

impl Dmc {
    fn new(samples: ROM) -> Self {
        Dmc {
            irq_en: false,
            irq_raised: false,
            dmc_loop: false,
            silence: false,
            rate_index: 0,
            current_output: 0,
            output_counter: 0,
            sample_addr: 0,
            current_addr: 0,
            sample_len: 0,
            bytes_remaining: 0,
            bits_remaining: 0,
            sample_shift_reg: 0,
            cycles_this_sample: u16::MAX,

            samples,
        }
    }

    pub fn register_read(&mut self, addr: u16) -> u8 {
        match addr {
            0 => ((self.irq_en as u8) << 7) | ((self.dmc_loop as u8) << 6) | self.rate_index,
            1 => self.output_counter,
            2 => (self.sample_addr / 64) as u8,
            3 => ((self.sample_len - 1) / 16) as u8,
            _ => unreachable!("Invalid read {}", addr),
        }
    }

    pub fn register_write(&mut self, addr: u16, val: u8) {
        match addr {
            0 => {
                self.irq_en = (val & 0x80) != 0;
                self.dmc_loop = (val & 0x40) != 0;
                self.rate_index = val & 0xF;
            }
            1 => self.output_counter = val & 0x7F,
            2 => self.sample_addr = val as usize * 64,
            3 => self.sample_len = 0x1 + (val as u16 * 16),
            _ => unreachable!("Invalid write {}", addr),
        }
    }

    pub fn clock(&mut self) -> u8 {
        // The output does not change on every call to clock, but periodically based on the rate
        // index.
        if self.cycles_this_sample < self.cycles_per_sample() {
            self.cycles_this_sample += 1;
            return self.current_output;
        }

        self.current_output = self.get_current_output();

        self.current_output
    }

    fn get_current_output(&mut self) -> u8 {
        if self.bits_remaining == 0 {
            self.bits_remaining = 8;

            if let Some(sample) = self.sample_byte() {
                self.sample_shift_reg = sample;
                self.silence = false;
            } else {
                self.silence = true;
            }
        }

        let lsb = self.sample_shift_reg & 0x1;
        self.sample_shift_reg >>= 1;
        self.bits_remaining -= 1;
        self.cycles_this_sample = 1;

        if self.silence {
            return 0;
        }

        self.output_counter = match lsb {
            0 => {
                if self.output_counter < 2 {
                    self.output_counter
                } else {
                    self.output_counter - 2
                }
            }
            1 => {
                if self.output_counter > 125 {
                    self.output_counter
                } else {
                    self.output_counter + 2
                }
            }
            _ => unreachable!(),
        };

        self.output_counter
    }

    fn cycles_per_sample(&self) -> u16 {
        assert!(self.rate_index < 0x10);

        // NOTE: The rates are provided in terms of CPU cycles in
        // https://www.nesdev.org/wiki/APU_DMC but they are more useful as APU clocks
        const RATE_TABLE: [u16; 16] = [
            398, 354, 316, 298, 276, 236, 210, 198, 176, 148, 132, 118, 98, 78, 66, 50,
        ];

        RATE_TABLE[self.rate_index as usize] / 2
    }

    fn sample_byte(&mut self) -> Option<u8> {
        if self.bytes_remaining {
            return None;
        }

        let data = self.samples[self.current_addr];
        self.current_addr = self.current_addr.wrapping_add(1);
        self.bytes_remaining -= 1;

        if self.bytes_remaining == 0 {
            if self.dmc_loop {
                self.start_sampling();
            } else if self.irq_en {
                self.irq_raised = true;
            }
        }

        Some(data)
    }

    fn start_sampling(&mut self) {
        self.current_addr = self.sample_addr;
        self.bytes_remaining = self.sample_len;
    }

    pub fn enable(&mut self, en: bool) {
        if en {
            self.start_sampling();
        } else {
            self.bytes_remaining = 0;
        }
    }
}

#[derive(Default)]
struct Noise {
    v_loop: bool,
    v_const: bool,
    n_loop: bool,
    envelope: u8,
    period: u8,

    length_load: u8,
}

impl Noise {
    fn register_read(&mut self, addr: u16) -> u8 {
        match addr {
            0 => ((self.v_loop as u8) << 5) | ((self.v_const as u8) << 4) | self.envelope,
            1 => 0xff,
            2 => ((self.n_loop as u8) << 7) | self.period,
            3 => self.length_load << 3,
            _ => unreachable!("Invalid read {}", addr),
        }
    }

    fn register_write(&mut self, addr: u16, val: u8) {
        match addr {
            0 => {
                self.v_loop = (val & 0x20) != 0;
                self.v_const = (val & 0x10) != 0;
                self.envelope = val & 0xF;
            }
            1 => {}
            2 => {
                self.n_loop = (val & 0x80) != 0;
                self.period = val & 0xF;
            }
            3 => self.length_load = val >> 3,
            _ => unreachable!("Invalid write {}", addr),
        }
    }
}

#[derive(Default)]
struct Pulse {
    v_loop: bool,
    v_const: bool,
    enabled: bool,
    negate: bool,
    shift: u8,
    period: u8,
    duty: u8,
    envelope: u8,

    length_load: u8,
    timer_lo: u8,
    timer_hi: u8,
}

impl Pulse {
    fn register_read(&mut self, addr: u16) -> u8 {
        match addr {
            0 => {
                (self.duty << 6)
                    | ((self.v_loop as u8) << 5)
                    | ((self.v_const as u8) << 4)
                    | self.envelope
            }
            1 => {
                ((self.enabled as u8) << 7)
                    | (self.period << 4)
                    | ((self.negate as u8) << 3)
                    | self.shift
            }
            2 => self.timer_lo,
            3 => (self.length_load << 3) | self.timer_hi,
            _ => unreachable!("Invalid read {}", addr),
        }
    }

    fn register_write(&mut self, addr: u16, val: u8) {
        match addr {
            0 => {
                self.duty = val >> 6;
                self.v_loop = (val & 0x20) != 0;
                self.v_const = (val & 0x10) != 0;
                self.envelope = val & 0xF;
            }
            1 => {
                self.enabled = (val & 0x80) != 0;
                self.period = (val & 0x70) >> 4;
                self.negate = (val & 0x8) != 0;
                self.shift = val & 0x7;
            }
            2 => self.timer_lo = val,
            3 => {
                self.length_load = val >> 3;
                self.timer_hi = val & 0x7;
            }
            _ => unreachable!("Invalid write {}", addr),
        }
    }
}

#[derive(Default)]
struct Triangle {
    halt: bool,
    linear_load: u8,

    length_load: u8,
    timer_lo: u8,
    timer_hi: u8,
}

impl Triangle {
    fn register_read(&mut self, addr: u16) -> u8 {
        match addr {
            0 => ((self.halt as u8) << 7) | self.linear_load,
            1 => 0xff,
            2 => self.timer_lo,
            3 => (self.length_load << 3) | self.timer_hi,
            _ => unreachable!("Invalid read {}", addr),
        }
    }

    fn register_write(&mut self, addr: u16, val: u8) {
        match addr {
            0 => {
                self.halt = (val & 0x80) != 0;
                self.linear_load = val & 0x7F;
            }
            1 => {}
            2 => self.timer_lo = val,
            3 => {
                self.length_load = val >> 3;
                self.timer_hi = val & 0x7;
            }
            _ => unreachable!("Invalid write {}", addr),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: usize = 398 / 2;
    const CHAR_BIT: usize = 8;
    const NUM_HI: usize = RATE * CHAR_BIT * 8;
    const NUM_LO: usize = RATE * CHAR_BIT * 9;

    fn dmc_init() -> Dmc {
        let mut samples = vec![0xFF; 8];
        samples.append(&mut vec![0; 8]);
        samples.push(0);
        let samples = ROM::with_data(&samples);

        let mut dmc = Dmc::new(samples);

        // Sample length to 1 + 16 * 1 == 17
        dmc.register_write(3, 1);

        // Sample address == 0 * 64
        dmc.register_write(2, 0);

        dmc.enable(true);

        dmc
    }

    #[test]
    fn dmc_loop_irq() {
        let mut dmc = dmc_init();

        for i in 0..(NUM_HI - RATE) {
            let val = dmc.clock() as usize;
            assert_eq!(val, 2 * (i / RATE + 1), "Mismatch on iteration {}", i);
        }

        // 63rd * <rate> clock will overflow past 127, so it will be "stuck" at 126
        for _ in 0..RATE {
            let val = dmc.clock();
            assert_eq!(val, 126);
        }

        // Set the loop flag so we can replay the sample. This must be set before the final sample
        // is read or else the it will not be restarted
        dmc.register_write(0, 0x40);

        for i in 0..NUM_LO {
            let val = dmc.clock();
            let (mut expected, overflowed) = 126_u8.overflowing_sub(2 * (i / RATE + 1) as u8);
            if overflowed {
                expected = 0
            }

            assert_eq!(val, expected, "Mismatch on iteration {}", i);
        }

        // Next samples shoud be reading the beginning back
        let val = dmc.clock();
        assert_eq!(val, 2);

        // Disable the loop, exhaust all samples and we should generate an IRQ. This should happen
        // when the bytes remaining counter is 0, not when the sample is exhausted
        dmc.register_write(0, 0x80);
        for _ in 0..(NUM_LO + NUM_HI) - 1 {
            let _ = dmc.clock();
        }

        assert_eq!(dmc.irq_raised, true);

        // Re-enable the DMC to begin again
        dmc.enable(true);
        let val = dmc.clock();
        assert_eq!(val, 2);
    }

    #[test]
    fn dmc_no_loop_no_irq() {
        let mut dmc = dmc_init();

        for i in 0..(NUM_HI - RATE) {
            let val = dmc.clock() as usize;
            assert_eq!(val, 2 * (i / RATE + 1), "Mismatch on iteration {}", i);
        }

        // 63rd * <rate> clock will overflow past 127, so it will be "stuck" at 126
        for _ in 0..RATE {
            let val = dmc.clock();
            assert_eq!(val, 126);
        }

        for i in 0..NUM_LO - 1 {
            let val = dmc.clock();
            let (mut expected, overflowed) = 126_u8.overflowing_sub(2 * (i / RATE + 1) as u8);
            if overflowed {
                expected = 0
            }

            assert_eq!(val, expected, "Mismatch on iteration {}", i);
        }

        // Enable the IRQ and loop too late when the last byte has already been read. No IRQ should
        // be generated, and we should not loop
        dmc.register_write(0, 0xc0);
        for i in 0..(NUM_LO + NUM_HI) {
            let val = dmc.clock();
            assert_eq!(val, 0, "Mismatch on iteration {}", i);
        }

        assert_eq!(dmc.irq_raised, false);
    }

    #[test]
    fn dmc_output_counter() {
        let mut dmc = dmc_init();
        dmc.register_write(0x1, 0x1);
        assert_eq!(dmc.register_read(0x1), 0x1);

        for _ in 0..RATE {
            let val = dmc.clock();
            assert_eq!(val, 3);

            // Since the sample is not updated every cycle, writing to the output counter should
            // not take effect until the sample is read at the end of the period
            dmc.register_write(0x1, 101);
        }

        let val = dmc.clock();
        assert_eq!(val, 103);
    }
}
