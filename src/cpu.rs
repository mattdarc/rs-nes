// References:
// https://en.wikipedia.org/wiki/MOS_Technology_6502#Registers
// http://nparker.llx.com/a2/opcodes.html
// https://www.masswerk.at/6502/6502_instruction_set.html

// - Branch instructions are a signed 8-bit offset from the address of the instruction following the branch
// - Accumulator mode uses the accumulator as an effective address and does not need any operand data
// - Immediate mode uses an 8-bit literal operand
// - With the 5/6 cycle "(indirect),y" mode, the 8-bit Y register is added to a 16-bit base address read
//   from zero page, which is located by a single byte following the opcode
// - "(indirect,x)" mode the effective address for the operation is found at the zero page address formed
//   by adding the second byte of the instruction to the contents of the X register

// Validation ROMs: https://wiki.nesdev.com/w/index.php/Emulator_tests#Validation_ROMs

use crate::common::*;
use crate::instructions::*;

const STACK_BEGIN: u16 = 0x0100;

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct Status {
    flags: u8,
}

impl Status {
    pub const NEGATIVE: u8 = 5;
    pub const ZERO: u8 = 4;
    pub const CARRY: u8 = 3;
    pub const IRQ: u8 = 2;
    pub const DECIMAL: u8 = 1;
    pub const OVERFLOW: u8 = 0;

    fn set(&mut self, bit: bool, idx: u8) {
        //println("-- Updating bit {}", idx);
        self.flags = (self.flags & !(1 << idx)) | ((bit as u8) << idx);
    }

    fn get(&self, idx: u8) -> bool {
        bit_set!(self.flags, idx)
    }
}

// impl std::fmt::UpperHex for Vec<u8> {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
//     }
// }

impl std::convert::Into<u8> for Status {
    fn into(self) -> u8 {
        self.flags
    }
}

impl std::convert::From<u8> for Status {
    fn from(flags: u8) -> Self {
        Status { flags }
    }
}

pub struct Ricoh2A03 {
    // CPU State
    pc: u16,
    acc: u8,
    x: u8,
    y: u8,
    sp: u8,
    status: Status,

    noop_cycles: u8,
    cycle: usize,
}

// NOTE this is a temporary type just so I don't need to plumb the bus through all the relevant
// function calls
pub struct ConnectedCPU<'a, BusType>
where
    BusType: Bus,
{
    cpu: &'a mut Ricoh2A03,
    bus: &'a mut BusType,
}

#[inline]
fn is_negative(v: u8) -> bool {
    (v & 0x80) != 0
}

#[inline]
fn crosses_page(src: u16, dst: u16) -> bool {
    // If the address is on a separate page, return true
    let crosses = ((src & 0xFF00) ^ (dst & 0xFF00)) != 0;
    if crosses {
        //println("-- Crossed page");
    }
    crosses
}

impl Ricoh2A03 {
    pub fn new() -> Ricoh2A03 {
        Ricoh2A03 {
            pc: 0,
            acc: 0,
            x: 0,
            y: 0,
            sp: 0,
            status: Status::from(0),

            cycle: 0,
            noop_cycles: 0,
        }
    }

    pub fn done(&self) -> bool {
        false
    }

    pub fn connect<'a, BusType: Bus>(
        &'a mut self, bus: &'a mut BusType,
    ) -> ConnectedCPU<'a, BusType> {
        ConnectedCPU { cpu: self, bus }
    }

    fn incr_pc(&mut self, v: u16) {
        // let pc = self.pc;
        self.pc = self.pc.wrapping_add(v);
        // pc
    }
}

impl<'a, BusType> ConnectedCPU<'a, BusType>
where
    BusType: Bus,
{
    pub fn init(&mut self) {
        //println("-- INITIALIZING");
        self.reset();
    }

    fn reset(&mut self) {
        self.cpu.pc = self.bus.read16(RESET_VECTOR_START as usize);
        //println("-- START VECTOR: : {:#X?}", self.pc);
    }

    // get_addr, get_mem, read_mem_mut should all use the mapper. Based on that address
    // we read the ram/ppu/apu
    fn get_addr(&mut self, addr_mode: &AddressingMode) -> u16 {
        use crate::instructions::AddressingMode::*;
        let ptr = self.cpu.pc as usize;
        let bus = &mut self.bus;
        match &addr_mode {
            ZeroPage => bus.read(ptr) as u16,
            ZeroPageX => {
                let low = bus.read(ptr) as u16;
                low.wrapping_add(self.cpu.x as u16)
            }
            ZeroPageY => (bus.read(ptr) as u16).wrapping_add(self.cpu.y as u16),
            Absolute => bus.read16(ptr),
            AbsoluteX => {
                let base = bus.read16(ptr);
                let addr = base.wrapping_add(self.cpu.x as u16);
                self.cpu.noop_cycles += crosses_page(base, addr) as u8;
                addr
            }
            AbsoluteY => {
                let base = bus.read16(ptr);
                let addr = base.wrapping_add(self.cpu.y as u16);
                self.cpu.noop_cycles += crosses_page(base, addr) as u8;
                addr
            }
            Indirect => {
                let addr = bus.read16(ptr) as usize;
                //println("-- Indirect address {:#X}", addr);
                bus.read16(addr)
            }
            IndirectX => {
                let addr = bus.read(ptr).wrapping_add(self.cpu.x) as usize;
                bus.read16(addr)
            }
            IndirectY => {
                let low = bus.read(ptr) as usize;
                let base = bus.read16(low);
                let addr = base.wrapping_add(self.cpu.y as u16);
                self.cpu.noop_cycles += crosses_page(base, addr) as u8;
                addr
            }
            Accumulator | Immediate | Relative | Invalid => {
                unreachable!("Invalid AddressingMode")
            }
        }
    }

    // TODO: consider refactoring out the address computation to reuse for read and write
    fn write_mem(&mut self, mode: &AddressingMode, val: u8) {
        use crate::instructions::AddressingMode::*;
        match &mode {
            ZeroPage | ZeroPageX | ZeroPageY | Absolute | AbsoluteX | AbsoluteY
            | Indirect | IndirectX | IndirectY => {
                let addr = self.get_addr(mode) as usize;
                self.bus.write(addr, val);
            }
            Accumulator => self.cpu.acc = val,
            Immediate | Relative => unreachable!("Tried to modify ROM!"),
            Invalid => unreachable!("Invalid AddressingMode!"),
        }
    }

    fn read_mem(&mut self, mode: &AddressingMode) -> u8 {
        use crate::instructions::AddressingMode::*;
        match &mode {
            ZeroPage | ZeroPageX | ZeroPageY | Absolute | AbsoluteX | AbsoluteY
            | Indirect | IndirectX | IndirectY => {
                let addr = self.get_addr(mode) as usize;
                self.bus.read(addr)
            }
            Accumulator => self.cpu.acc,
            Immediate | Relative => self.bus.read(self.cpu.pc as usize),
            Invalid => unreachable!("Invalid AddressingMode"),
        }
    }

    // HELPERS:
    fn do_branch(&mut self, dst: u8) {
        let pc = if is_negative(dst) {
            self.cpu.pc.wrapping_sub(dst.wrapping_neg() as u16)
        } else {
            self.cpu.pc.wrapping_add(dst as u16)
        };
        //println("-- Taking branch from {:#X} to {:#X}", self.pc, pc);

        // add 1 if same page, 2 if different
        self.cpu.noop_cycles += 1 + crosses_page(self.cpu.pc, pc) as u8;
        self.cpu.pc = pc;
    }

    fn peek(&mut self) -> u8 {
        let ptr = (self.cpu.sp as u16).wrapping_add(STACK_BEGIN) as usize;
        self.bus.read(ptr)
    }

    fn poke(&mut self, val: u8) {
        let ptr = (self.cpu.sp as u16).wrapping_add(STACK_BEGIN) as usize;
        self.bus.write(ptr, val);
    }

    // Update the CPU flags based on the accumulator
    fn update_flags(&mut self, v: u8) {
        // NOTE: anything greater than 127 is negative since it is a 2's compliment format
        self.cpu.status.set(is_negative(v), Status::NEGATIVE);
        self.cpu.status.set(v == 0, Status::ZERO);
    }

    fn push16(&mut self, v: u16) {
        self.push8((v >> 8) as u8);
        self.push8((0xF & v) as u8);
    }

    fn push8(&mut self, v: u8) {
        self.poke(v);
        self.cpu.sp = self.cpu.sp.wrapping_add(1);
        assert!(self.cpu.sp != 0, "Stack overflow!");
    }

    fn pop16(&mut self) -> u16 {
        let low = self.pop8() as u16;
        ((self.pop8() as u16) << 8) | low
    }

    fn pop8(&mut self) -> u8 {
        assert!(self.cpu.sp != 0, "Tried to pop empty stack!");
        self.cpu.sp = self.cpu.sp.wrapping_sub(1);
        self.peek()
    }

    // BRANCHES:
    // BPL
    fn branch_if_pos(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if !self.cpu.status.get(Status::NEGATIVE) {
            self.do_branch(dst);
        }
    }

    // BMI
    fn branch_if_neg(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if self.cpu.status.get(Status::NEGATIVE) {
            self.do_branch(dst);
        }
    }

    // BVC
    fn branch_if_overflow_clear(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if !self.cpu.status.get(Status::OVERFLOW) {
            self.do_branch(dst);
        }
    }

    // BVS
    fn branch_if_overflow_set(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if self.cpu.status.get(Status::OVERFLOW) {
            self.do_branch(dst);
        }
    }

    // BCC
    fn branch_if_carry_clear(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if !self.cpu.status.get(Status::OVERFLOW) {
            self.do_branch(dst);
        }
    }

    // BCS
    fn branch_if_carry_set(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if self.cpu.status.get(Status::CARRY) {
            self.do_branch(dst);
        }
    }

    // BNE
    fn branch_if_not_zero(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if !self.cpu.status.get(Status::ZERO) {
            self.do_branch(dst);
        }
    }

    // BEQ
    fn branch_if_zero(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if self.cpu.status.get(Status::ZERO) {
            self.do_branch(dst);
        }
    }

    // ADC
    fn add_with_carry(&mut self, mode: &AddressingMode) {
        let operand = self.read_mem(mode);
        let (result, over1) = operand.overflowing_add(self.cpu.acc);
        let (result, over2) =
            result.overflowing_add(self.cpu.status.get(Status::CARRY) as u8);

        let over_carry = over1 || over2;
        self.cpu.status.set(over_carry, Status::CARRY);
        self.cpu.status.set(over_carry, Status::OVERFLOW);
        self.cpu.acc = result;
        self.update_flags(self.cpu.acc);
    }

    // AND
    fn and_with_acc(&mut self, mode: &AddressingMode) {
        let operand = self.read_mem(mode);
        self.cpu.acc &= operand;
        self.update_flags(self.cpu.acc);
    }

    // ASL
    fn shift_left(&mut self, mode: &AddressingMode) {
        let val = self.read_mem(mode);
        self.write_mem(mode, val << 1);
        self.update_flags(val);
        self.cpu.status.set(is_negative(val), Status::CARRY);
    }

    // BIT
    fn test_bits(&mut self, mode: &AddressingMode) {
        let operand = self.read_mem(mode);
        //println("-- Test bits {} & {}", operand, self.acc);
        self.cpu.status.set(bit_set!(operand, 6), Status::OVERFLOW);
        self.cpu.status.set(is_negative(operand), Status::NEGATIVE);
        self.cpu
            .status
            .set((self.cpu.acc & operand) == 0, Status::ZERO);
    }

    // BRK
    fn force_break(&mut self) {
        self.push16(self.cpu.pc.wrapping_add(2));
        self.push_status();
        self.cpu.status.set(true, Status::IRQ);
    }

    // CLC
    fn clear_carry(&mut self) {
        self.cpu.status.set(false, Status::CARRY);
    }

    // CLD
    fn clear_decimal(&mut self) {
        self.cpu.status.set(false, Status::DECIMAL);
    }

    // CLI
    fn clear_interrupt(&mut self) {
        self.cpu.status.set(false, Status::IRQ);
    }

    // CLV
    fn clear_overflow(&mut self) {
        self.cpu.status.set(false, Status::OVERFLOW);
    }

    // CMP
    fn cmp_with_acc(&mut self, mode: &AddressingMode) {
        let operand = self.read_mem(mode);
        let (result, carry) = self.cpu.acc.overflowing_sub(operand);
        self.cpu.status.set(carry, Status::CARRY);
        self.update_flags(result);
    }

    // CPX
    fn cmp_with_x(&mut self, mode: &AddressingMode) {
        let operand = self.read_mem(mode);
        let (result, carry) = self.cpu.x.overflowing_sub(operand);
        self.cpu.status.set(carry, Status::CARRY);
        self.update_flags(result);
    }

    // CPY
    fn cmp_with_y(&mut self, mode: &AddressingMode) {
        let operand = self.read_mem(mode);
        let (result, carry) = self.cpu.y.overflowing_sub(operand);
        self.cpu.status.set(carry, Status::CARRY);
        self.update_flags(result);
    }

    // DEC
    fn dec_mem(&mut self, mode: &AddressingMode) {
        let val = self.read_mem(mode).wrapping_sub(1);
        self.write_mem(mode, val);
        self.update_flags(val);
    }

    // DEX
    fn dec_x(&mut self) {
        self.cpu.x = self.cpu.x.wrapping_sub(1);
        self.update_flags(self.cpu.x);
    }

    // DEY
    fn dec_y(&mut self) {
        self.cpu.y = self.cpu.y.wrapping_sub(1);
        self.update_flags(self.cpu.y);
    }

    // EOR
    fn xor_acc(&mut self, mode: &AddressingMode) {
        let mem = self.read_mem(mode);
        self.cpu.acc ^= mem;
        self.update_flags(self.cpu.acc);
    }

    // INC
    fn inc_mem(&mut self, mode: &AddressingMode) {
        let val = self.read_mem(mode).wrapping_add(1);
        self.write_mem(mode, val);
        self.update_flags(val);
    }

    // INX
    fn inc_x(&mut self) {
        self.cpu.x = self.cpu.x.wrapping_add(1);
        self.update_flags(self.cpu.x);
    }

    // INY
    fn inc_y(&mut self) {
        self.cpu.y = self.cpu.y.wrapping_add(1);
        self.update_flags(self.cpu.y);
    }

    // JMP
    fn jump_to(&mut self, mode: &AddressingMode) {
        let addr = self.get_addr(mode) as usize;
        //println("-- PC Destination from {:#X}", addr);
        self.cpu.pc = self.bus.read16(addr);
        //println("-- Jump to {}", self.pc);
    }

    // JSR
    fn jump_save_ret(&mut self, mode: &AddressingMode) {
        let pc = self.cpu.pc;
        self.push16(pc);
        let addr = self.get_addr(mode) as usize;
        self.cpu.pc = self.bus.read16(addr);
    }

    // LDA
    fn load_acc_with_mem(&mut self, mode: &AddressingMode) {
        let mem = self.read_mem(mode);
        self.cpu.acc = mem;
        self.update_flags(self.cpu.acc);
    }

    // LDX
    fn load_x_with_mem(&mut self, mode: &AddressingMode) {
        let mem = self.read_mem(mode);
        self.cpu.x = mem;
        self.update_flags(self.cpu.x);
    }

    // LDY
    fn load_y_with_mem(&mut self, mode: &AddressingMode) {
        let mem = self.read_mem(mode);
        self.cpu.y = mem;
        self.update_flags(self.cpu.y);
    }

    // LSR
    fn shift_right(&mut self, mode: &AddressingMode) {
        let mut mem = self.read_mem(mode);
        let carry = (mem & 0x01) != 0;
        mem >>= 1;
        self.write_mem(mode, mem);
        self.update_flags(mem);
        self.cpu.status.set(carry, Status::CARRY);
    }

    // NOP
    fn nop(&self) {}

    // ORA
    fn or_acc(&mut self, mode: &AddressingMode) {
        let mem = self.read_mem(mode);
        self.cpu.acc |= mem;
        self.update_flags(self.cpu.acc);
    }

    // PHA
    fn push_acc(&mut self) {
        //println("-- Pushing {} onto stack", self.acc);
        self.push8(self.cpu.acc);
    }

    // PHP
    fn push_status(&mut self) {
        self.push8(self.cpu.status.into());
    }

    // PLA
    fn pull_acc(&mut self) {
        self.cpu.acc = self.pop8();
    }

    // PLP
    fn pull_status(&mut self) {
        self.cpu.status = Status::from(self.pop8());
    }

    // ROL
    fn rotate_left(&mut self, mode: &AddressingMode) {
        let carry = self.cpu.status.get(Status::CARRY);
        let mem = self.read_mem(mode);
        self.cpu.status.set((mem & 0x80) != 0, Status::CARRY);
        let val = (mem << 1) | (carry as u8);
        self.write_mem(mode, val);
        self.update_flags(val);
    }

    // ROR
    fn rotate_right(&mut self, mode: &AddressingMode) {
        let carry = self.cpu.status.get(Status::CARRY);
        let mem = self.read_mem(mode);
        self.cpu.status.set((mem & 0x01) != 0, Status::CARRY);
        let val = (mem >> 1) | ((carry as u8) << 7);
        self.write_mem(mode, val);
        self.update_flags(val);
    }

    // RTI
    fn ret_from_interrupt(&mut self) {
        self.pull_status();
        self.cpu.pc = self.pop16();
    }

    // RTS
    fn ret_from_subr(&mut self) {
        self.cpu.pc = self.pop16().wrapping_add(1);
    }

    // SBC
    fn sub_with_carry(&mut self, mode: &AddressingMode) {
        let mem = self.read_mem(mode);
        let (result, over1) = self.cpu.acc.overflowing_sub(mem);
        let (result, over2) =
            result.overflowing_sub(self.cpu.status.get(Status::CARRY) as u8);

        let over_carry = over1 || over2;
        self.cpu.status.set(over_carry, Status::CARRY);
        self.cpu.status.set(over_carry, Status::OVERFLOW);
        self.cpu.acc = result;
        self.update_flags(self.cpu.acc);
    }

    // SEC
    fn set_carry(&mut self) {
        self.cpu.status.set(true, Status::CARRY);
    }

    // SED
    fn set_decimal(&mut self) {
        self.cpu.status.set(true, Status::DECIMAL);
    }

    // SEI
    fn set_interrupt(&mut self) {
        self.cpu.status.set(true, Status::IRQ);
    }

    // STA
    fn store_acc_mem(&mut self, mode: &AddressingMode) {
        self.write_mem(mode, self.cpu.acc);
    }

    // STX
    fn store_x_mem(&mut self, mode: &AddressingMode) {
        self.write_mem(mode, self.cpu.x);
    }

    // STY
    fn store_y_mem(&mut self, mode: &AddressingMode) {
        self.write_mem(mode, self.cpu.y);
    }

    // TAX
    fn tx_acc_to_x(&mut self) {
        self.cpu.x = self.cpu.acc;
        self.update_flags(self.cpu.x);
    }

    // TAY
    fn tx_acc_to_y(&mut self) {
        self.cpu.y = self.cpu.acc;
        self.update_flags(self.cpu.y);
    }

    // TSX
    fn tx_sp_to_x(&mut self) {
        self.cpu.x = self.cpu.sp;
        self.update_flags(self.cpu.x);
    }

    // TXA
    fn tx_x_to_acc(&mut self) {
        self.cpu.acc = self.cpu.x;
        self.update_flags(self.cpu.acc);
    }

    // TXS
    fn tx_x_to_sp(&mut self) {
        self.cpu.sp = self.cpu.x;
    }

    // TYA
    fn tx_y_to_acc(&mut self) {
        self.cpu.acc = self.cpu.y;
        self.update_flags(self.cpu.acc);
    }
}

impl<BusType> Clocked<BusType> for Ricoh2A03
where
    BusType: Bus,
{
    fn clock(&mut self, bus: &mut BusType) {
        use crate::instructions;
        use crate::instructions::AddressingMode::*;
        use crate::instructions::InstrName::*;
        //println("-- Starting CPU cycle {}", self.cycle);

        self.cycle += 1;
        if self.noop_cycles > 0 {
            //println("-- Running No-Op instruction, {} left", self.noop_cycles);
            self.noop_cycles -= 1;
            return;
        }

        let opcode = bus.read(self.pc as usize);
        let instr = instructions::get_from(opcode);

        // 1 cycle we use to execute the instruction
        self.noop_cycles = instr.cycles() - 1;
        // println!(
        //     "-- Running {:#X} {:?} from {:#X} for {} cycles",
        //     opcode,
        //     &instr,
        //     self.pc,
        //     self.noop_cycles + 1
        // );
        self.incr_pc(1);

        match instr.name() {
            // BRANCHES
            BPL => self.connect(bus).branch_if_pos(&instr.mode()),
            BMI => self.connect(bus).branch_if_neg(&instr.mode()),
            BVC => self.connect(bus).branch_if_overflow_clear(&instr.mode()),
            BVS => self.connect(bus).branch_if_overflow_set(&instr.mode()),
            BCC => self.connect(bus).branch_if_carry_clear(&instr.mode()),
            BCS => self.connect(bus).branch_if_carry_set(&instr.mode()),
            BNE => self.connect(bus).branch_if_not_zero(&instr.mode()),
            BEQ => self.connect(bus).branch_if_zero(&instr.mode()),
            ADC => self.connect(bus).add_with_carry(&instr.mode()),
            AND => self.connect(bus).and_with_acc(&instr.mode()),
            SBC => self.connect(bus).sub_with_carry(&instr.mode()),
            ORA => self.connect(bus).or_acc(&instr.mode()),
            LDY => self.connect(bus).load_y_with_mem(&instr.mode()),
            LDX => self.connect(bus).load_x_with_mem(&instr.mode()),
            LDA => self.connect(bus).load_acc_with_mem(&instr.mode()),
            EOR => self.connect(bus).xor_acc(&instr.mode()),
            CPY => self.connect(bus).cmp_with_y(&instr.mode()),
            CPX => self.connect(bus).cmp_with_x(&instr.mode()),
            CMP => self.connect(bus).cmp_with_acc(&instr.mode()),
            BIT => self.connect(bus).test_bits(&instr.mode()),

            ASL => self.connect(bus).shift_left(&instr.mode()),
            LSR => self.connect(bus).shift_right(&instr.mode()),
            JSR => self.connect(bus).jump_save_ret(&instr.mode()),
            JMP => self.connect(bus).jump_to(&instr.mode()),
            STY => self.connect(bus).store_y_mem(&instr.mode()),
            STX => self.connect(bus).store_x_mem(&instr.mode()),
            STA => self.connect(bus).store_acc_mem(&instr.mode()),
            ROL => self.connect(bus).rotate_left(&instr.mode()),
            ROR => self.connect(bus).rotate_right(&instr.mode()),
            INC => self.connect(bus).inc_mem(&instr.mode()),
            DEC => self.connect(bus).dec_mem(&instr.mode()),

            CLV => self.connect(bus).clear_overflow(),
            CLI => self.connect(bus).clear_interrupt(),
            CLC => self.connect(bus).clear_carry(),
            CLD => self.connect(bus).clear_decimal(),
            DEX => self.connect(bus).dec_x(),
            DEY => self.connect(bus).dec_y(),
            INY => self.connect(bus).inc_y(),
            INX => self.connect(bus).inc_x(),
            TAY => self.connect(bus).tx_acc_to_y(),
            TAX => self.connect(bus).tx_acc_to_x(),
            TYA => self.connect(bus).tx_y_to_acc(),
            TXA => self.connect(bus).tx_x_to_acc(),
            TXS => self.connect(bus).tx_x_to_sp(),
            TSX => self.connect(bus).tx_sp_to_x(),
            SEI => self.connect(bus).set_interrupt(),
            SED => self.connect(bus).set_decimal(),
            SEC => self.connect(bus).set_carry(),
            RTS => self.connect(bus).ret_from_subr(),
            RTI => self.connect(bus).ret_from_interrupt(),
            PLP => self.connect(bus).pull_status(),
            PLA => self.connect(bus).pull_acc(),
            PHP => self.connect(bus).push_status(),
            PHA => self.connect(bus).push_acc(),
            BRK => self.connect(bus).force_break(),

            NOP => self.connect(bus).nop(),
            _ => {
                let last_pc = self.pc - 1;
                unreachable!(
                    "-- Invalid Instruction. Surrounding instructions: {:?}",
                    bus.read_n((last_pc - 2) as usize, 5)
                );
            }
        }

        match instr.mode() {
            ZeroPage => self.incr_pc(1),
            ZeroPageX => self.incr_pc(1),
            ZeroPageY => self.incr_pc(1),
            Absolute => self.incr_pc(2),
            AbsoluteX => self.incr_pc(2),
            AbsoluteY => self.incr_pc(2),
            Indirect => self.incr_pc(2),
            IndirectX => self.incr_pc(1),
            IndirectY => self.incr_pc(1),
            Immediate | Relative => self.incr_pc(1),
            Accumulator | Invalid => {}
        }
    }
}

#[cfg(test)]
#[path = "./tcpu.rs"]
mod tests;
