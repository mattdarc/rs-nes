pub mod instructions;
mod interpreter;
mod status;

use {
    crate::bus::Bus,
    crate::timer,
    crate::ExitStatus,
    instructions::Instruction,
    status::Status,
    std::stringify,
    tracing::{event, span, Level},
};

#[inline]
pub fn is_negative(v: u8) -> bool {
    is_bit_set(v, 7)
}

#[inline]
fn xor(a: bool, b: bool) -> bool {
    return (a || b) && (!a || !b);
}

#[inline]
fn is_bit_set(v: u8, bit: u8) -> bool {
    (v & (1 << bit)) != 0
}

#[inline]
fn as_hex_digit(i: u8) -> u8 {
    char::from_digit(i.into(), 16)
        .expect("Out of range [0, 16)")
        .to_ascii_uppercase() as u8
}

#[inline]
fn crosses_page(src: u16, dst: u16) -> bool {
    (src & 0xFF00) != (dst & 0xFF00)
}

#[inline]
fn sign_extend(x: u8) -> u16 {
    unsafe { std::mem::transmute((x as i16).wrapping_shl(8).wrapping_shr(8)) }
}

const STACK_BEGIN: u16 = 0x0100;

//  ADDRESSES	 |  VECTOR
// $FFFA, $FFFB	 |   NMI
// $FFFC, $FFFD	 |   Reset
// $FFFE, $FFFF	 |   IRQ/BRK
// https://www.nesdev.org/wiki/NMI
const NMI_VECTOR_START: u16 = 0xFFFA;
const RESET_VECTOR_START: u16 = 0xFFFC;
const IRQ_VECTOR_START: u16 = 0xFFFE;

// Exported for use in tests

enum TargetAddress {
    Memory(u16),
    Accumulator,
    None,
}

// FIXME: Write a proc macro for this
macro_rules! buildable {
    ($result:ident; $name: ident {
        $($fld:ident: $type: ty $(,)?)*
    }) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $result {
            $(pub $fld: $type,)*
        }

        #[derive(Clone, Debug, Default)]
        pub struct $name {
            $($fld: Option<$type>,)*
        }

        impl $name {
            pub fn new() -> Self {
                $name::default()
            }

            pub fn build(self) -> $result {
                $result {
                    $(
                        $fld: self.$fld.expect(
                            &format!("Field '{}' uninitialized", stringify!($fld))
                        ),
                    )*
                }
            }
$(
            pub fn $fld(mut self, $fld: $type) -> Self {
                assert!(self.$fld.is_none());
                self.$fld = Some($fld);
                self
            }
)*
        }
    };
}

buildable!(NESSnapshot; SnapshotBuilder {
    total_cycles: usize,
    instruction: Instruction,
    operands: Vec<u8>,
    acc: u8,
    x: u8,
    y: u8,
    pc: u16,
    sp: u8,
    status: u8,

    // PPU
    scanline: i16,
    ppu_cycle: i16,
});

pub trait CpuInterface {
    fn read_state(&self) -> NESSnapshot;
    fn read_address(&mut self, addr: u16) -> u8;
    fn request_stop(&mut self, code: i32);
}

impl<BusType: Bus> CpuInterface for CPU<BusType> {
    fn read_state(&self) -> NESSnapshot {
        let (scanline, ppu_cycle) = self.interpreter.bus.ppu_state();

        NESSnapshot {
            total_cycles: self.interpreter.bus.cycles(),
            instruction: self.interpreter.instruction().clone(),
            operands: self.interpreter.operands().to_vec(),
            acc: self.state.acc,
            x: self.state.x,
            y: self.state.y,
            pc: self.last_pc,
            sp: self.state.sp,
            status: self.state.status.to_u8(),
            scanline,
            ppu_cycle,
        }
    }

    // FIXME: find a way not to duplicate this with the interp
    fn read_address(&mut self, addr: u16) -> u8 {
        self.interpreter.bus.read(addr)
    }

    fn request_stop(&mut self, retcode: i32) {
        self.exit_status = ExitStatus::StopRequested(retcode);
    }
}

// State which is shared between the interpreter and the binary translator
struct CpuState {
    acc: u8,
    x: u8,
    y: u8,
    pc: u16,
    sp: u8,
    status: Status,

    instructions_executed: usize,
}

impl CpuState {
    pub fn new() -> Self {
        CpuState {
            acc: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: 0xFD,
            status: Status::empty(),
            instructions_executed: 0,
        }
    }

    // Update the CPU flags based on the accumulator
    fn update_nz(&mut self, v: u8) {
        self.status.set(Status::NEGATIVE, is_negative(v));
        self.status.set(Status::ZERO, v == 0);
    }
}

pub struct CPU<BusType: Bus> {
    state: CpuState,
    interpreter: interpreter::Interpreter<BusType>,

    last_pc: u16,
    exit_status: ExitStatus,
}

impl<BusType: Bus> CPU<BusType> {
    pub fn new(bus: BusType) -> Self {
        CPU {
            state: CpuState::new(),
            interpreter: interpreter::Interpreter::new(bus),
            exit_status: ExitStatus::Continue,
            last_pc: 0,
        }
    }

    pub fn pc(&self) -> u16 {
        self.state.pc
    }

    pub fn nestest_reset_override(&mut self, pc: u16) {
        self.interpreter.reset(&mut self.state);
        self.state.pc = pc;

        // The gold log starts with 7 cycles clocked on the bus
        self.interpreter.bus.clock(7);
    }

    pub fn reset(&mut self) {
        self.interpreter.reset(&mut self.state);
    }

    pub fn clock(&mut self) -> ExitStatus {
        let cpu_span = span!(
            target: "cpu",
            Level::TRACE,
            "clock",
        );

        self.last_pc = self.state.pc;
        let cycles = {
            let _enter = cpu_span.enter();

            if let Some(cycles) = self.interpreter.handle_nmi(&mut self.state) {
                cycles
            } else {
                self.interpreter.interpret(&mut self.state)
            }
        };

        self.interpreter.clock_bus(cycles as usize);
        self.exit_status.clone()
    }
}

fn trace_instruction(state: &CpuState, instr: &Instruction, operands: &[u8]) {
    const BUFSZ: usize = 10;
    let mut operands_str: [u8; BUFSZ] = [' ' as u8; BUFSZ];

    if tracing::enabled!(Level::DEBUG) {
        for (i, op) in operands.iter().enumerate() {
            operands_str[3 * i] = as_hex_digit(op >> 4);
            operands_str[3 * i + 1] = as_hex_digit(op & 0xf);
        }
        operands_str[BUFSZ - 1] = '\0' as u8;
    }

    event!(
            Level::DEBUG,
            "[{:>10}]  {:<04X}  {:<2X} {:<8} {:>5}  {:>12}  A:{:02X}  X:{:02X}  Y:{:02X}  P:{:02X}  SP:{:02X}",
            state.instructions_executed,
            state.pc,
            instr.opcode(),
            std::str::from_utf8(&operands_str).unwrap(),
            format!("{:>5}", instr.name()),
            format!("{:?}", instr.mode()),
            state.acc,
            state.x,
            state.y,
            state.status.bits(),
            state.sp,
        );
}

#[cfg(test)]
mod tests;
