use super::*;
use crate::memory::{RAM, ROM};
use instructions::AddressingMode::*;
use instructions::InstrName::*;

const TEST_PROGRAM_START: usize = 0x7FF0;

struct TestBus {
    program: ROM,
    ram: RAM,
}

impl TestBus {
    pub fn new(data: &[u8]) -> Self {
        TestBus {
            program: ROM::with_data(data),
            ram: RAM::with_size(0x800),
        }
    }
}

impl Bus for TestBus {
    fn read(&mut self, addr: u16) -> u8 {
        let addr = addr as usize;
        match addr {
            TEST_PROGRAM_START..=0xFFFF => self.program[addr],
            _ => self.ram[addr % 0x800],
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        let addr = addr as usize;
        match addr {
            TEST_PROGRAM_START..=0xFFFF => self.program[addr] = val,
            _ => self.ram[addr % 0x800] = val,
        }
    }

    fn cycles(&self) -> usize {
        0
    }

    fn clock(&mut self, _cycles: u8) {}

    fn pop_nmi(&mut self) -> Option<u8> {
        None
    }
}

fn initialize_program(data: &[u8]) -> CPU<TestBus> {
    println!("DATA: {:x?}", data);
    let mut program = vec![0; 0xFFFF];
    program[TEST_PROGRAM_START as usize..(TEST_PROGRAM_START as usize + data.len())]
        .copy_from_slice(data);
    program[RESET_VECTOR_START as usize] = (TEST_PROGRAM_START & 0xFF) as u8;
    program[RESET_VECTOR_START as usize + 1] = (TEST_PROGRAM_START >> 8) as u8;

    let bus = TestBus::new(&program);
    let mut cpu = CPU::new(bus);
    cpu.reset();

    // Default status is not empty, but we make it such for ease in the following tests
    cpu.status = Status::empty();
    cpu
}

macro_rules! verify_op {
    ($name:ident, $addr_mode:ident, $opcode:literal, $($($operands:literal)*,)*
     [$($addr:literal=$val:literal),*]{$($reg:ident : $pv:expr),*} => [$($exp_addr:literal = $exp_b:expr),*]{$($eflg:ident : $ev:expr),*}) => {
	let act_instr = instructions::decode_instruction(($opcode).into());
	assert_eq!(act_instr.name(), &$name, "Instruction mismatch for {:?}", &$name);
	assert_eq!(act_instr.mode(), &$addr_mode, "Address mode mismatch for {:?}", &$addr_mode);

	// Set up initial CPU state
    let mut cpu = initialize_program(&[$opcode, $($($operands,)*)*]);
	$(cpu.$reg = $pv;)*
	$(cpu.bus.write($addr, $val);)*

	// Init and keep track of PC
	let pc_bef = cpu.pc;

	// Make sure we run for the correct number cycles
	cpu.clock();

	// Verify CPU state
	if !instructions::is_branch(&$name) {
        // Branch instructions will jump to an arbitrary PC, validated by the test itself
        assert_eq!(cpu.pc - pc_bef, act_instr.size(), "PC did not retrieve the correct number of bytes");

        // Similarly the cycle count wwill be validated as some branch instructions have cycle
        // penalties
	    assert_eq!(cpu.cycles, act_instr.cycles());
    }

	$(assert_eq!(cpu.$eflg, $ev, "Flag mismatch $eflg");)*
	$(assert_eq!(cpu.bus.read($exp_addr), $exp_b, "Memory at {:#X} does not match {:#}", $exp_addr, $exp_b);)*
    }
}

#[test]
fn paging() {
    assert!(crosses_page(0x7FFF, 0x8000));
}

#[test]
fn negative() {
    assert!(is_negative(255));
    assert!(is_negative(128));
    assert!(!is_negative(127));
    assert!(!is_negative(0));
}

// TODO: Add flag verification
#[test]
fn adc() {
    verify_op!(ADC, Immediate, 0x69, 0x03, []{acc: 2} => []{acc: 5});
    verify_op!(ADC, ZeroPage,  0x65, 0x00, [0x00=0x01]{acc: 2} => []{acc: 3});
    verify_op!(ADC, ZeroPageX, 0x75, 0x01, [0x07=0x01]{acc: 4, x: 6} => []{acc: 5});
    verify_op!(ADC, Absolute,  0x6D, 0x00, 0x10, [0x1000=0x01]{acc: 4} => []{acc: 5});
    verify_op!(ADC, AbsoluteX, 0x7D, 0x00, 0x10, [0x1006=0x01]{acc: 4, x: 6} => []{acc: 5});
    verify_op!(ADC, AbsoluteY, 0x79, 0x00, 0x10, [0x1006=0x01]{acc: 4, y: 6} => []{acc: 5});
    verify_op!(ADC, IndirectX, 0x61, 0x1, [0x08=0x10, 0x1000=0x02]{acc: 4, x: 6} => []{acc: 6});
    verify_op!(ADC, IndirectY, 0x71, 0x1, [0x2=0x10, 0x1006=0x02]{acc: 4, y: 6} => []{acc: 6});
}

#[test]
fn and() {
    verify_op!(AND, Immediate, 0x29, 0x03, []{acc: 2} => []{acc: 2});
    verify_op!(AND, ZeroPage,  0x25, 0x00, [0x00=0x01]{acc: 3} => []{acc: 1});
    verify_op!(AND, ZeroPageX, 0x35, 0x01, [0x07=0x01]{acc: 5, x: 6} => []{acc: 1});
    verify_op!(AND, Absolute,  0x2D, 0x00, 0x10, [0x1000=0x05]{acc: 5} => []{acc: 5});
    verify_op!(AND, AbsoluteX, 0x3D, 0x00, 0x10, [0x1006=0x05]{acc: 4, x: 6} => []{acc: 4});
    verify_op!(AND, AbsoluteY, 0x39, 0x00, 0x10, [0x1012=0x05]{acc: 4, y: 0x12} => []{acc: 4});
    verify_op!(AND, IndirectX, 0x21, 0x1, [0x08=0x10, 0x1000=0x07]{acc: 7, x: 6} => []{acc: 7});
    verify_op!(AND, IndirectY, 0x31, 0x1, [0x2=0x10, 0x1006=0x07]{acc: 7, y: 6} => []{acc: 7});
}

#[test]
fn asl() {
    verify_op!(ASL, Accumulator, 0x0A, []{acc: 3} => []{acc: 6});
    verify_op!(ASL, ZeroPage,    0x06, 0x00, [0x00=0x01]{} => [0x00=0x02]{});
    verify_op!(ASL, ZeroPageX,   0x16, 0x01, [0x07=0x01]{x: 6} => [0x07=0x02]{});
    verify_op!(ASL, Absolute,    0x0E, 0x00, 0x10, [0x1000=0x05]{} => [0x1000=0x0A]{});
    verify_op!(ASL, AbsoluteX,   0x1E, 0x00, 0x10, [0x1006=0x05]{x: 6} => [0x1006=0x0A]{});
}

#[test]
fn bit() {
    verify_op!(BIT, ZeroPage, 0x24, 0x00, [0x0=0xFF]{} => []{status: Status::ZERO | Status::OVERFLOW | Status::NEGATIVE});
    verify_op!(BIT, ZeroPage, 0x24, 0x00, [0x0=0xFF]{acc: 1} => []{status: Status::OVERFLOW | Status::NEGATIVE});
    verify_op!(BIT, ZeroPage, 0x24, 0x01, [0x0=0x5F]{} => []{});
    verify_op!(BIT, Absolute, 0x2C, 0x00, 0x10, [0x1000=0xFF]{} => []{status: Status::ZERO | Status::OVERFLOW | Status::NEGATIVE});
    verify_op!(BIT, Absolute, 0x2C, 0x00, 0x10, [0x1000=0xFF]{acc: 1} => []{status: Status::OVERFLOW | Status::NEGATIVE});
    verify_op!(BIT, Absolute, 0x2C, 0x01, 0x10, [0x1001=0x5F]{} => []{});
}

#[test]
fn branches() {
    verify_op!(BCS, Relative, 0xB0, 0x10, []{status: Status::empty()} => []{cycles: 2, pc: TEST_PROGRAM_START as u16 + 0x2});
    verify_op!(BCS, Relative, 0xB0, 0x7F, []{status: Status::CARRY} =>   []{cycles: 4, pc: TEST_PROGRAM_START as u16 + 0x81});

    verify_op!(BEQ, Relative, 0xF0, 0x10, []{status: Status::empty()} => []{cycles: 2, pc: TEST_PROGRAM_START as u16 + 0x2});
    verify_op!(BEQ, Relative, 0xF0, 0x7F, []{status: Status::ZERO} =>    []{cycles: 4, pc: TEST_PROGRAM_START as u16 + 0x81});

    verify_op!(BMI, Relative, 0x30, 0x10, []{status: Status::empty()} =>  []{cycles: 2, pc: TEST_PROGRAM_START as u16 + 0x2});
    verify_op!(BMI, Relative, 0x30, 0x7F, []{status: Status::NEGATIVE} => []{cycles: 4, pc: TEST_PROGRAM_START as u16 + 0x81});

    verify_op!(BNE, Relative, 0xD0, 0x10, []{status: Status::ZERO} =>    []{cycles: 2, pc: TEST_PROGRAM_START as u16 + 0x2});
    verify_op!(BNE, Relative, 0xD0, 0x7F, []{status: Status::empty()} => []{cycles: 4, pc: TEST_PROGRAM_START as u16 + 0x81});

    verify_op!(BPL, Relative, 0x10, 0x10, []{status: Status::NEGATIVE} => []{cycles: 2, pc: TEST_PROGRAM_START as u16 + 0x2});
    verify_op!(BPL, Relative, 0x10, 0x1, []{status: Status::empty()} =>  []{cycles: 3, pc: TEST_PROGRAM_START as u16 + 0x3});

    verify_op!(BVC, Relative, 0x50, 0x10, []{status: Status::OVERFLOW} => []{cycles: 2, pc: TEST_PROGRAM_START as u16 + 0x2});
    verify_op!(BVC, Relative, 0x50, 0x1, []{status: Status::empty()} =>  []{cycles: 3, pc: TEST_PROGRAM_START as u16 + 0x3});

    verify_op!(BVS, Relative, 0x70, 0x10, []{status: Status::empty()} =>  []{cycles: 2, pc: TEST_PROGRAM_START as u16 + 0x2});
    verify_op!(BVS, Relative, 0x70, 0x1, []{status: Status::OVERFLOW} => []{cycles: 3, pc: TEST_PROGRAM_START as u16 + 0x3});
}

#[test]
fn flags() {
    verify_op!(CLC, Implied, 0x18, []{status: Status::CARRY} => []{});
    verify_op!(CLD, Implied, 0xD8, []{status: Status::DECIMAL} => []{});
    verify_op!(CLI, Implied, 0x58, []{status: Status::INT_DISABLE} => []{});
    verify_op!(CLV, Implied, 0xB8, []{status: Status::OVERFLOW} => []{});

    verify_op!(SEC, Implied, 0x38, []{} => []{status: Status::CARRY});
    verify_op!(SED, Implied, 0xF8, []{} => []{status: Status::DECIMAL});
    verify_op!(SEI, Implied, 0x78, []{} => []{status: Status::INT_DISABLE});
}

// TODO Carry should be set if we wrap
#[test]
fn cmp() {
    verify_op!(CMP, Immediate, 0xC9, 0x03, []{acc: 2} => []{status: Status::NEGATIVE});
    verify_op!(CMP, ZeroPage,  0xC5, 0x00, [0x00=0x03]{acc: 3} => []{status: Status::ZERO | Status::CARRY});
    verify_op!(CMP, ZeroPageX, 0xD5, 0x01, [0x07=0x03]{acc: 5, x: 6} => []{status: Status::CARRY});
    verify_op!(CMP, Absolute,  0xCD, 0x00, 0x10, [0x1000=0x05]{acc: 5} => []{status: Status::ZERO| Status::CARRY});
    verify_op!(CMP, AbsoluteX, 0xDD, 0x00, 0x10, [0x1006=0x05]{acc: 4, x: 6} => []{status: Status::NEGATIVE});
    verify_op!(CMP, AbsoluteY, 0xD9, 0x00, 0x10, [0x1012=0x05]{acc: 4, y: 0x12} => []{status: Status::NEGATIVE});
    verify_op!(CMP, IndirectX, 0xC1, 0x1, [0x08=0x10, 0x1000=0x07]{acc: 7, x: 6} => []{status: Status::ZERO| Status::CARRY});
    verify_op!(CMP, IndirectY, 0xD1, 0x1, [0x2=0x10, 0x1006=0x07]{acc: 7, y: 6} => []{status: Status::ZERO| Status::CARRY});
}

#[test]
fn cpx() {
    verify_op!(CPX, Immediate, 0xE0, 0x03, []{x: 2} => []{status: Status::NEGATIVE});
    verify_op!(CPX, ZeroPage,  0xE4, 0x00, [0x00=0x03]{x: 3} => []{status: Status::ZERO | Status::CARRY});
    verify_op!(CPX, Absolute,  0xEC, 0x00, 0x10, [0x1000=0x05]{x: 5} => []{status: Status::CARRY | Status::ZERO});
}

#[test]
fn cpy() {
    verify_op!(CPY, Immediate, 0xC0, 0x03, []{y: 2} => []{status: Status::NEGATIVE});
    verify_op!(CPY, ZeroPage,  0xC4, 0x00, [0x00=0x03]{y: 3} => []{status: Status::CARRY | Status::ZERO});
    verify_op!(CPY, Absolute,  0xCC, 0x00, 0x10, [0x1000=0x05]{y: 5} => []{status: Status::CARRY | Status::ZERO});
}

#[test]
fn dec() {
    verify_op!(DEC, ZeroPage,  0xC6, 0x00, [0x00=0x01]{} => [0x00=0x00]{status: Status::ZERO});
    verify_op!(DEC, ZeroPageX, 0xD6, 0x00, [0x01=0x00]{x: 1} => [0x01=0xFF]{status: Status::NEGATIVE});
    verify_op!(DEC, Absolute,  0xCE, 0x00, 0x10, [0x1000=0x01]{} => [0x1000=0x00]{status: Status::ZERO});
    verify_op!(DEC, AbsoluteX, 0xDE, 0x00, 0x10, [0x1005=0x00]{x: 5} => [0x1005=0xFF]{status: Status::NEGATIVE});
}

#[test]
fn dex() {
    verify_op!(DEX, Implied, 0xCA, []{x: 1} => []{status: Status::ZERO});
    verify_op!(DEX, Implied, 0xCA, []{x: 0} => []{status: Status::NEGATIVE});
}

#[test]
fn dey() {
    verify_op!(DEY, Implied, 0x88, []{y: 1} => []{status: Status::ZERO});
    verify_op!(DEY, Implied, 0x88, []{y: 0} => []{status: Status::NEGATIVE});
}

#[test]
fn eor() {
    verify_op!(EOR, Immediate, 0x49, 0x03, []{acc: 3} => []{acc: 0, status: Status::ZERO});
    verify_op!(EOR, ZeroPage,  0x45, 0x00, [0x00=0x03]{acc: 0x83} => []{acc: 0x80, status: Status::NEGATIVE});
    verify_op!(EOR, ZeroPageX, 0x55, 0x01, [0x07=0x03]{acc: 5, x: 6} => []{acc: 6, status: Status::empty()});
    verify_op!(EOR, Absolute,  0x4D, 0x00, 0x10, [0x1000=0x05]{acc: 5} => []{acc: 0, status: Status::ZERO});
    verify_op!(EOR, AbsoluteX, 0x5D, 0x00, 0x10, [0x1006=0x05]{acc: 4, x: 6} => []{acc: 1, status: Status::empty()});
    verify_op!(EOR, AbsoluteY, 0x59, 0x00, 0x10, [0x1012=0x05]{acc: 4, y: 0x12} => []{acc: 1, status: Status::empty()});
    verify_op!(EOR, IndirectX, 0x41, 0x1, [0x08=0x10, 0x1000=0x07]{acc: 7, x: 6} => []{acc: 0, status: Status::ZERO});
    verify_op!(EOR, IndirectY, 0x51, 0x1, [0x2=0x10, 0x1006=0x07]{acc: 7, y: 6} => []{acc: 0, status: Status::ZERO});
}

#[test]
fn inc() {
    verify_op!(INC, ZeroPage,  0xE6, 0x00, [0x00=0xFF]{} => [0x00=0x00]{status: Status::ZERO});
    verify_op!(INC, ZeroPageX, 0xF6, 0x00, [0x01=0x00]{x: 1} => [0x01=0x1]{status: Status::empty()});
    verify_op!(INC, Absolute,  0xEE, 0x00, 0x10, [0x1000=0xFF]{} => [0x1000=0x00]{status: Status::ZERO});
    verify_op!(INC, AbsoluteX, 0xFE, 0x00, 0x10, [0x1005=0x00]{x: 5} => [0x1005=0x01]{status: Status::empty()});
}

#[test]
fn inx() {
    verify_op!(INX, Implied,  0xE8, []{x: 0xFF} => []{status: Status::ZERO});
    verify_op!(INX, Implied,  0xE8, []{x: 0xFE} => []{status: Status::NEGATIVE});
}

#[test]
fn iny() {
    verify_op!(INY, Implied,  0xC8, []{y: 0xFF} => []{status: Status::ZERO});
    verify_op!(INY, Implied,  0xC8, []{y: 0xFE} => []{status: Status::NEGATIVE});
}

#[test]
fn jmp() {
    verify_op!(JMP, Absolute, 0x4C, 0x00, 0x10, []{} => []{pc: 0x1000});
    verify_op!(JMP, Indirect, 0x6C, 0x00, 0x1, [0x100=0x01, 0x101=0x10]{} => []{pc: 0x1001});
}

#[test]
fn jsr() {
    verify_op!(JSR, Absolute, 0x20, 0x00, 0x10, []{} => []{pc: 0x1000});
}

#[test]
fn lda() {
    verify_op!(LDA, Immediate, 0xA9, 0x03, []{} => []{acc: 3});
    verify_op!(LDA, ZeroPage,  0xA5, 0x00, [0x00=0x00]{} => []{acc: 0x00, status: Status::ZERO});
    verify_op!(LDA, ZeroPageX, 0xB5, 0x01, [0x07=0xF0]{x: 6} => []{acc: 0xF0, status: Status::NEGATIVE});
    verify_op!(LDA, Absolute,  0xAD, 0x00, 0x10, [0x1000=0x00]{} => []{acc: 0, status: Status::ZERO});
    verify_op!(LDA, AbsoluteX, 0xBD, 0x00, 0x10, [0x1006=0x05]{x: 6} => []{acc: 5, status: Status::empty()});
    verify_op!(LDA, AbsoluteY, 0xB9, 0x00, 0x10, [0x1012=0x05]{y: 0x12} => []{acc: 5, status: Status::empty()});
    verify_op!(LDA, IndirectX, 0xA1, 0x1, [0x08=0x10, 0x1000=0xF7]{x: 6} => []{acc: 0xF7, status: Status::NEGATIVE});
    verify_op!(LDA, IndirectY, 0xB1, 0x1, [0x2=0x10, 0x1006=0x07]{y: 6} => []{acc: 0x07, status: Status::empty()});
}

#[test]
fn ldx() {
    verify_op!(LDX, Immediate, 0xA2, 0x03, []{} => []{x: 3});
    verify_op!(LDX, ZeroPage,  0xA6, 0x00, [0x00=0x00]{} => []{x: 0x00, status: Status::ZERO});
    verify_op!(LDX, ZeroPageY, 0xB6, 0x01, [0x07=0xF0]{y: 6} => []{x: 0xF0, status: Status::NEGATIVE});
    verify_op!(LDX, Absolute,  0xAE, 0x00, 0x10, [0x1000=0x00]{} => []{x: 0, status: Status::ZERO});
    verify_op!(LDX, AbsoluteY, 0xBE, 0x00, 0x10, [0x1012=0x05]{y: 0x12} => []{x: 5, status: Status::empty()});
}

#[test]
fn ldy() {
    verify_op!(LDY, Immediate, 0xA0, 0x03, []{} => []{y: 3});
    verify_op!(LDY, ZeroPage,  0xA4, 0x00, [0x00=0x00]{} => []{y: 0x00, status: Status::ZERO});
    verify_op!(LDY, ZeroPageX, 0xB4, 0x01, [0x07=0xF0]{x: 6} => []{y: 0xF0, status: Status::NEGATIVE});
    verify_op!(LDY, Absolute,  0xAC, 0x00, 0x10, [0x1000=0x00]{} => []{y: 0, status: Status::ZERO});
    verify_op!(LDY, AbsoluteX, 0xBC, 0x00, 0x10, [0x1006=0x05]{x: 6} => []{y: 5, status: Status::empty()});
}

#[test]
fn lsr() {
    verify_op!(LSR, Accumulator, 0x4A, []{acc: 0xFF} => []{acc: 0x7F, status: Status::CARRY});
    verify_op!(LSR, ZeroPage,    0x46, 0x00, [0x00=0x01]{} => [0x00=0x00]{status: Status::CARRY | Status::ZERO});
    verify_op!(LSR, ZeroPageX,   0x56, 0x01, [0x07=0xF0]{x: 6} => [0x07=0x78]{status: Status::empty()});
    verify_op!(LSR, Absolute,    0x4E, 0x00, 0x10, [0x1000=0x00]{} => [0x1000=0x00]{status: Status::ZERO});
    verify_op!(LSR, AbsoluteX,   0x5E, 0x00, 0x10, [0x1006=0x05]{x: 6} => [0x1006=0x02]{status: Status::CARRY});
}

#[test]
fn nop() {
    verify_op!(NOP, Implied, 0xEA, []{} => []{});
}

#[test]
fn ora() {
    verify_op!(ORA, Immediate, 0x09, 0x03, []{acc: 3} => []{acc: 3, status: Status::empty()});
    verify_op!(ORA, ZeroPage,  0x05, 0x00, [0x00=0x03]{acc: 0x83} => []{acc: 0x83, status: Status::NEGATIVE});
    verify_op!(ORA, ZeroPageX, 0x15, 0x01, [0x07=0x03]{acc: 5, x: 6} => []{acc: 7, status: Status::empty()});
    verify_op!(ORA, Absolute,  0x0D, 0x00, 0x10, [0x1000=0x00]{acc: 0} => []{acc: 0, status: Status::ZERO});
    verify_op!(ORA, AbsoluteX, 0x1D, 0x00, 0x10, [0x1006=0x05]{acc: 4, x: 6} => []{acc: 5, status: Status::empty()});
    verify_op!(ORA, AbsoluteY, 0x19, 0x00, 0x10, [0x1012=0x05]{acc: 4, y: 0x12} => []{acc: 5, status: Status::empty()});
    verify_op!(ORA, IndirectX, 0x01, 0x1, [0x08=0x10, 0x1000=0x07]{acc: 7, x: 6} => []{acc: 7, status: Status::empty()});
    verify_op!(ORA, IndirectY, 0x11, 0x1, [0x2=0x10, 0x1006=0x07]{acc: 7, y: 6} => []{acc: 7, status: Status::empty()});
}

#[test]
fn stack() {
    // verify_op!(PHA, Invalid, 0x48, 0x03, []{acc: 3} => []{acc: 3, status: Status::empty()});
    // verify_op!(PHP, Invalid, 0x08, 0x00, [0x00=0x03]{acc: 0x83} => []{acc: 0x83, status: set_status!(Status::NEGATIVE)});
    // verify_op!(PLA, Invalid, 0x68, 0x01, [0x07=0x03]{acc: 5, x: 6} => []{acc: 7, status: Status::empty()});
    // verify_op!(PLP, Invalid, 0x28, 0x00, 0x10, [0x1000=0x00]{acc: 0} => []{acc: 0, status: set_status!(Status::ZERO)});
}

#[test]
fn rol() {
    verify_op!(ROL, Accumulator, 0x2A, []{acc: 0xFF, status: Status::CARRY} => []{acc: 0xFF, status: Status::NEGATIVE | Status::CARRY});
    verify_op!(ROL, ZeroPage,    0x26, 0x00, [0x00=0x01]{} => [0x00=0x02]{status: Status::empty()});
    verify_op!(ROL, ZeroPageX,   0x36, 0x01, [0x07=0x80]{x: 6} => [0x07=0x00]{status: Status::CARRY | Status::ZERO});
    verify_op!(ROL, Absolute,    0x2E, 0x00, 0x10, [0x1000=0x70]{} => [0x1000=0xE0]{status: Status::NEGATIVE});
    verify_op!(ROL, AbsoluteX,   0x3E, 0x00, 0x10, [0x1006=0x05]{x: 6} => [0x1006=0x0A]{status: Status::empty()});
}

#[test]
fn ror() {
    verify_op!(ROR, Accumulator, 0x6A, []{acc: 0xFF, status: Status::CARRY} => []{acc: 0xFF, status: Status::NEGATIVE | Status::CARRY});
    verify_op!(ROR, ZeroPage,    0x66, 0x00, [0x00=0x01]{} => [0x00=0x00]{status: Status::CARRY | Status::ZERO});
    verify_op!(ROR, ZeroPageX,   0x76, 0x01, [0x07=0x80]{x: 6} => [0x07=0x40]{status: Status::empty()});
    verify_op!(ROR, Absolute,    0x6E, 0x00, 0x10, [0x1000=0x70]{} => [0x1000=0x38]{status: Status::empty()});
    verify_op!(ROR, AbsoluteX,   0x7E, 0x00, 0x10, [0x1006=0x05]{x: 6} => [0x1006=0x02]{status: Status::CARRY});
}

#[test]
fn rt() {
    //verify_op!(RTI, Invalid, 0x6A, []{acc: 0xFF, status: set_status!(Status::CARRY)} => []{acc: 0xFF, status:set_status!(Status::NEGATIVE, Status::CARRY)});
    //verify_op!(RTS, Invalid, 0x66, 0x00, [0x00=0x01]{} => [0x00=0x00]{status: set_status!(Status::CARRY, Status::ZERO)});
}

// TODO: Validate overflow with other implementations
#[test]
fn sbc() {
    verify_op!(SBC, IndirectY, 0xF1, 0x1, [0x2=0x10, 0x1006=0x07]{acc: 8, y: 6} => []{acc: 0, status: Status::ZERO | Status::CARRY});
    verify_op!(SBC, Immediate, 0xE9, 0x03, []{acc: 4} => []{acc: 0, status: Status::ZERO | Status::CARRY});
    verify_op!(SBC, ZeroPage,  0xE5, 0x00, [0x00=0x03]{acc: 0x84} => []{acc: 0x80, status: Status::NEGATIVE | Status::CARRY});
    verify_op!(SBC, ZeroPageX, 0xF5, 0x01, [0x07=0xFF]{acc: 5, x: 6, status: Status::CARRY} => []{acc: 6, status: Status::empty()});
    verify_op!(SBC, Absolute,  0xED, 0x00, 0x10, [0x1000=0x00]{acc: 0} => []{acc: 0xFF, status: Status::NEGATIVE});
    verify_op!(SBC, AbsoluteX, 0xFD, 0x00, 0x10, [0x1006=0x05]{acc: 4, x: 6} => []{acc: 0xFE, status: Status::NEGATIVE});
    verify_op!(SBC, AbsoluteY, 0xF9, 0x00, 0x10, [0x1012=0xFF]{acc: 5, y: 0x12} => []{acc: 0x5, status: Status::empty()});
    verify_op!(SBC, IndirectX, 0xE1, 0x1, [0x08=0x10, 0x1000=0x07]{acc: 7, x: 6, status: Status::CARRY} =>
    []{acc: 0, status: Status::ZERO | Status::CARRY});
}

#[test]
fn sta() {
    verify_op!(STA, ZeroPage,  0x85, 0x00, []{status: Status::NEGATIVE, acc: 0x83} => [0x00=0x83]{status: Status::NEGATIVE});
    verify_op!(STA, ZeroPageX, 0x95, 0x01, []{acc: 5, x: 6} => [0x07=0x05]{acc: 5});
    verify_op!(STA, Absolute,  0x8D, 0x00, 0x10, []{acc: 5} => [0x1000=0x05]{acc: 5});
    verify_op!(STA, AbsoluteX, 0x9D, 0x00, 0x10, []{acc: 4, x: 6} => [0x1006=0x04]{acc: 4});
    verify_op!(STA, AbsoluteY, 0x99, 0x00, 0x10, []{acc: 5, y: 0x12} => [0x1012=0x5]{acc: 5});
    verify_op!(STA, IndirectX, 0x81, 0x1, [0x08=0x10]{acc: 7, x: 6} => [0x1000=0x07]{acc: 7});
    verify_op!(STA, IndirectY, 0x91, 0x1, [0x2=0x10]{acc: 7, y: 6} => [0x1006=0x07]{acc: 7});
}

#[test]
fn stx() {
    verify_op!(STX, ZeroPage,  0x86, 0x00, []{status: Status::NEGATIVE, x: 0x83} => [0x00=0x83]{status: Status::NEGATIVE});
    verify_op!(STX, ZeroPageY, 0x96, 0x01, []{x: 5, y: 6} => [0x07=0x05]{x: 5});
    verify_op!(STX, Absolute,  0x8E, 0x00, 0x10, []{x: 5} => [0x1000=0x05]{x: 5});
}

#[test]
fn sty() {
    verify_op!(STY, ZeroPage,  0x84, 0x00, []{status: Status::NEGATIVE, y: 0x83} => [0x00=0x83]{status: Status::NEGATIVE});
    verify_op!(STY, ZeroPageX, 0x94, 0x01, []{y: 5, x: 6} => [0x07=0x05]{y: 5});
    verify_op!(STY, Absolute,  0x8C, 0x00, 0x10, []{y: 5} => [0x1000=0x05]{y: 5});
}

#[test]
fn tax() {
    verify_op!(TAX, Implied,  0xAA, []{acc: 0xFF} => []{acc: 0xFF, x: 0xFF, status: Status::NEGATIVE});
    verify_op!(TAX, Implied,  0xAA, []{acc: 0x00, x: 1} => []{acc: 0x00, x: 0x00, status: Status::ZERO});
}

#[test]
fn tay() {
    verify_op!(TAY, Implied,  0xA8, []{acc: 0xFF} => []{acc: 0xFF, y: 0xFF, status: Status::NEGATIVE});
    verify_op!(TAY, Implied,  0xA8, []{acc: 0x00, y: 1} => []{acc: 0x00, y: 0x00, status: Status::ZERO});
}

#[test]
fn tsx() {
    verify_op!(TSX, Implied,  0xBA, []{sp: 0x80} => []{sp: 0x80, x: 0x80, status: Status::NEGATIVE});
    verify_op!(TSX, Implied,  0xBA, []{sp: 0x00, x: 1} => []{sp: 0x00, x: 0x00, status: Status::ZERO});
}

#[test]
fn txa() {
    verify_op!(TXA, Implied,  0x8A, []{x: 0xFF} => []{x: 0xFF, acc: 0xFF, status: Status::NEGATIVE});
    verify_op!(TXA, Implied,  0x8A, []{x: 0x00, acc: 1} => []{x: 0x00, acc: 0x00, status: Status::ZERO});
}

#[test]
fn txs() {
    verify_op!(TXS, Implied,  0x9A, []{x: 0x80} => []{x: 0x80, sp: 0x80, status: Status::empty()});
    verify_op!(TXS, Implied,  0x9A, []{x: 0x00} => []{x: 0x00, sp: 0x00, status: Status::empty()});
}

#[test]
fn tya() {
    verify_op!(TYA, Implied,  0x98, []{y: 0xFF} => []{y: 0xFF, acc: 0xFF, status: Status::NEGATIVE});
    verify_op!(TYA, Implied,  0x98, []{y: 0x00, acc: 1} => []{y: 0x00, acc: 0x00, status: Status::ZERO});
}
