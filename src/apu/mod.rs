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
    dmc_loop: bool,
    freq: u8,
    load_counter: u8,
    sample_addr: usize,
    sample_len: u16,

    samples: ROM,
}

impl Dmc {
    fn new(samples: ROM) -> Self {
        // FIXME: The samples we get from the cartridge should be all possible samples
        // assert_eq!(samples.len(), std::u8::MAX as usize * 16 + 1);

        Dmc {
            irq_en: false,
            dmc_loop: false,
            freq: 0,
            load_counter: 0,
            sample_addr: 0,
            sample_len: 0,

            samples,
        }
    }
}

impl Dmc {
    fn register_read(&mut self, addr: u16) -> u8 {
        match addr {
            0 => ((self.irq_en as u8) << 7) | ((self.dmc_loop as u8) << 6) | self.freq,
            1 => self.load_counter,
            2 => (self.sample_addr / 64) as u8,
            3 => ((self.sample_len - 1) / 16) as u8,
            _ => unreachable!("Invalid read {}", addr),
        }
    }

    fn register_write(&mut self, addr: u16, val: u8) {
        match addr {
            0 => {
                self.irq_en = (val & 0x80) != 0;
                self.dmc_loop = (val & 0x40) != 0;
                self.freq = val & 0xF;
            }
            1 => self.load_counter = val & 0x7F,
            2 => self.sample_addr = val as usize * 64,
            3 => self.sample_len = 0x1 + (val as u16 * 16),
            _ => unreachable!("Invalid write {}", addr),
        }
    }

    pub fn sample(&mut self) -> u8 {
        todo!()
    }

    pub fn disable(&mut self) {}

    pub fn enable(&mut self) {}
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
