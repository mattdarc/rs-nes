// The pulse, triangle, and noise channels each have their own length
// counter unit. It is clocked twice per sequence, and counts down to zero
// if enabled. When the length counter reaches zero the channel is
// silenced. It is reloaded by writing a 5-bit value to the appropriate
// channel's length counter register, which will load a value from a lookup
// table.

#![allow(dead_code)]

mod counter;
mod dmc;
mod noise;
mod pulse;
mod triangle;
mod volume;
mod mixer;

use crate::common::*;

use self::mixer::Mixer;
use self::counter::{Frame, FrameCounter};
use self::dmc::DMC;
use self::noise::Noise;
use self::pulse::Pulse;
use self::triangle::Triangle;

fn unused(addr: u16, val: u8) {
    println!("Write {:#X} to unused address {:#X}!", val, addr);
}

#[derive(Default, Clone)]
pub struct APU {
    pulse1: Pulse,
    pulse2: Pulse,
    triangle: Triangle,
    noise: Noise,
    dmc: DMC,
    frame_counter: FrameCounter,
    mixer: Mixer,
    even_cycle: bool,
}

impl APU {
    const DMC_STATUS: u8 = 5;
    const NOI_STATUS: u8 = 4;
    const TRI_STATUS: u8 = 3;
    const PL1_STATUS: u8 = 2;
    const PL2_STATUS: u8 = 1;

    fn quarter_frame(&mut self) {
        self.pulse1.quarter_frame();
        self.pulse2.quarter_frame();
        self.triangle.quarter_frame();
        self.noise.quarter_frame();
        self.dmc.quarter_frame();
    }

    fn half_frame(&mut self) {
        self.quarter_frame();

        self.pulse1.half_frame();
        self.pulse2.half_frame();
        self.triangle.half_frame();
        self.noise.half_frame();
        self.dmc.half_frame();
    }

    fn interrupt_frame(&mut self) {
        self.half_frame();
        todo!("Handle interrupt");
    }

    fn clock_frame(&mut self, frame: Frame) {
        match frame {
            Frame::Quarter => self.quarter_frame(),
            Frame::Half => self.half_frame(),
            Frame::Interrupt => self.interrupt_frame(),
            Frame::None => (),
        }
    }
}

impl Sampled for APU {
    type OutputType = i16;
    fn sample(&self) -> Self::OutputType {
        let pulse = self.pulse1.sample()
            + self.pulse2.sample();
        let tri = self.triangle.sample();
        let noi = self.noise.sample();
        let dmc = self.dmc.sample();


	// TODO replace with LUT
	let pulse_out = if pulse != 0 {
            95.88 / ((8218.0 / (pulse as f64)) + 100.0)
	} else {
	    0.0
	};
	
        let tnd_out = if tri != 0 || noi != 0 || dmc != 0 {
	    159.79
		/ ((1.0 / ((tri as f64 / 8227.0) +
			   (noi as f64 / 12241.0) +
			   (dmc as f64 / 22638.0))) + 100.0)
	} else {
	    0.0
	};

	let scaled = (pulse_out + tnd_out) * 65535.0;

	self.mixer.filter(scaled) as i16
    }
}

impl Writeable for APU {
    fn write(&mut self, addr: usize, val: u8) {
        match addr {
            0x4000 => self.pulse1.set_volume_grp(val),
            0x4001 => self.pulse1.set_sweep(val),
            0x4002 => self.pulse1.set_timer_low_grp(val),
            0x4003 => self.pulse1.set_length_counter_grp(val),
            0x4004 => self.pulse2.set_volume_grp(val),
            0x4005 => self.pulse2.set_sweep(val),
            0x4006 => self.pulse2.set_timer_low_grp(val),
            0x4007 => self.pulse2.set_length_counter_grp(val),
            0x4008 => self.triangle.set_linear_counter_grp(val),
            0x4009 => unused(0x4009, val),
            0x400A => self.triangle.set_timer_low_grp(val),
            0x400B => self.triangle.set_length_counter_grp(val),
            0x4010 => self.dmc.set_control_grp(val),
            0x4011 => self.dmc.direct_load(val),
            0x4012 => self.dmc.set_sample_address(val),
            0x4013 => self.dmc.set_sample_length(val),
            0x4015 => {
                self.pulse1.set_enabled(!bit_set!(val, APU::PL1_STATUS));
                self.pulse2.set_enabled(!bit_set!(val, APU::PL2_STATUS));
                self.triangle.set_enabled(!bit_set!(val, APU::TRI_STATUS));
                self.noise.set_enabled(!bit_set!(val, APU::NOI_STATUS));
                self.dmc.set_enabled(!bit_set!(val, APU::DMC_STATUS));
            }
            0x4017 => {
		let frame_number = self.frame_counter.set_control(val);
		self.clock_frame(frame_number);
	    },

            _ => unreachable!("Invalid APU address!"),
        }
    }
}

impl Clocked for APU {
    fn clock(&mut self) {
        // Every clock
        self.triangle.clock();
        self.frame_counter.clock();

        // Every other clock
        if self.even_cycle {
            self.pulse1.clock();
            self.pulse2.clock();
            self.noise.clock();
            self.dmc.clock();
        }

	let frame_number = self.frame_counter.get_number();
	self.clock_frame(frame_number);

        self.even_cycle = !self.even_cycle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apu() {
        // TODO
    }
}
