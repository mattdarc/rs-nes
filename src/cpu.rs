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

use crate::bus::Bus;

use crate::cartridge::*;
use crate::common::*;
use crate::instructions::*;

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

#[derive(Clone)]
pub struct Ricoh2A03<'a> {
    // CPU State
    pc: u16,
    acc: u8,
    x: u8,
    y: u8,
    sp: u8,
    status: Status,

    noop_cycles: u8,
    cycle: usize,

    // Memory
    bus: Bus<'a>,
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

impl<'a> Ricoh2A03<'a> {
    const STACK_BEGIN: u16 = 0x0100;

    pub fn with(cartridge: &'a Cartridge) -> Ricoh2A03<'a> {
        let mut cpu = Ricoh2A03 {
            pc: 0,
            acc: 0,
            x: 0,
            y: 0,
            sp: 0,
            status: Status::from(0),

            cycle: 0,
            noop_cycles: 0,

            bus: Bus::new(),
        };
        cpu.bus.init(&cartridge).unwrap();
        cpu
    }

    pub fn new() -> Ricoh2A03<'a> {
        Ricoh2A03 {
            pc: 0,
            acc: 0,
            x: 0,
            y: 0,
            sp: 0,
            status: Status::from(0),

            cycle: 0,
            noop_cycles: 0,

            bus: Bus::new(),
        }
    }

    pub fn insert(
        &mut self, cartridge: &'a Cartridge,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.bus.init(cartridge)
    }
}

impl Ricoh2A03<'_> {
    pub fn init(&mut self) {
        //println("-- INITIALIZING");
        self.reset();
    }

    pub fn run(&mut self) -> u8 {
        while !self.done() {
            self.clock();
        }
        self.status.into()
    }

    pub fn run_for(&mut self, cycles: usize) -> u8 {
        let mut count = 0;
        while !self.done() && count < cycles {
            self.clock();
            count += 1;
        }
        self.status.into()
    }

    pub fn exit(&mut self) {
        self.status = Status::from(0);
    }

    fn done(&self) -> bool {
        false
    }

    fn reset(&mut self) {
        self.pc = self.bus.read16(RESET_VECTOR_START as usize);
        //println("-- START VECTOR: : {:#X?}", self.pc);
    }

    fn incr_pc(&mut self, v: u16) {
        // let pc = self.pc;
        self.pc = self.pc.wrapping_add(v);
        // pc
    }

    // get_addr, get_mem, read_mem_mut should all use the mapper. Based on that address
    // we read the ram/ppu/apu
    fn get_addr(&mut self, addr_mode: &AddressingMode) -> u16 {
        use crate::instructions::AddressingMode::*;
        let ptr = self.pc as usize;
        match &addr_mode {
            ZeroPage => self.bus.read(ptr) as u16,
            ZeroPageX => {
                let low = self.bus.read(ptr) as u16;
                low.wrapping_add(self.x as u16)
            }
            ZeroPageY => (self.bus.read(ptr) as u16).wrapping_add(self.y as u16),
            Absolute => self.bus.read16(ptr),
            AbsoluteX => {
                let base = self.bus.read16(ptr);
                let addr = base.wrapping_add(self.x as u16);
                self.noop_cycles += crosses_page(base, addr) as u8;
                addr
            }
            AbsoluteY => {
                let base = self.bus.read16(ptr);
                let addr = base.wrapping_add(self.y as u16);
                self.noop_cycles += crosses_page(base, addr) as u8;
                addr
            }
            Indirect => {
                let addr = self.bus.read16(ptr) as usize;
                //println("-- Indirect address {:#X}", addr);
                self.bus.read16(addr)
            }
            IndirectX => {
                let addr = self.bus.read(ptr).wrapping_add(self.x) as usize;
                self.bus.read16(addr)
            }
            IndirectY => {
                let low = self.bus.read(ptr) as usize;
                let base = self.bus.read16(low);
                let addr = base.wrapping_add(self.y as u16);
                self.noop_cycles += crosses_page(base, addr) as u8;
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
            Accumulator => self.acc = val,
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
            Accumulator => self.acc,
            Immediate | Relative => self.bus.read(self.pc as usize),
            Invalid => unreachable!("Invalid AddressingMode"),
        }
    }

    // HELPERS:
    fn do_branch(&mut self, dst: u8) {
        let pc = if is_negative(dst) {
            self.pc.wrapping_sub(dst.wrapping_neg() as u16)
        } else {
            self.pc.wrapping_add(dst as u16)
        };
        //println("-- Taking branch from {:#X} to {:#X}", self.pc, pc);

        // add 1 if same page, 2 if different
        self.noop_cycles += 1 + crosses_page(self.pc, pc) as u8;
        self.pc = pc;
    }

    fn peek(&mut self) -> u8 {
        let ptr = (self.sp as u16).wrapping_add(Ricoh2A03::STACK_BEGIN) as usize;
        self.bus.read(ptr)
    }

    fn poke(&mut self, val: u8) {
        let ptr = (self.sp as u16).wrapping_add(Ricoh2A03::STACK_BEGIN) as usize;
        self.bus.write(ptr, val);
    }

    // Update the CPU flags based on the accumulator
    fn update_flags(&mut self, v: u8) {
        // NOTE: anything greater than 127 is negative since it is a 2's compliment format
        self.status.set(is_negative(v), Status::NEGATIVE);
        self.status.set(v == 0, Status::ZERO);
    }

    fn push16(&mut self, v: u16) {
        self.push8((v >> 8) as u8);
        self.push8((0xF & v) as u8);
    }

    fn push8(&mut self, v: u8) {
        self.poke(v);
        self.sp = self.sp.wrapping_add(1);
        assert!(self.sp != 0, "Stack overflow!");
    }

    fn pop16(&mut self) -> u16 {
        let low = self.pop8() as u16;
        ((self.pop8() as u16) << 8) | low
    }

    fn pop8(&mut self) -> u8 {
        assert!(self.sp != 0, "Tried to pop empty stack!");
        self.sp = self.sp.wrapping_sub(1);
        self.peek()
    }

    // BRANCHES:
    // BPL
    fn branch_if_pos(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if !self.status.get(Status::NEGATIVE) {
            self.do_branch(dst);
        }
    }

    // BMI
    fn branch_if_neg(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if self.status.get(Status::NEGATIVE) {
            self.do_branch(dst);
        }
    }

    // BVC
    fn branch_if_overflow_clear(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if !self.status.get(Status::OVERFLOW) {
            self.do_branch(dst);
        }
    }

    // BVS
    fn branch_if_overflow_set(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if self.status.get(Status::OVERFLOW) {
            self.do_branch(dst);
        }
    }

    // BCC
    fn branch_if_carry_clear(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if !self.status.get(Status::OVERFLOW) {
            self.do_branch(dst);
        }
    }

    // BCS
    fn branch_if_carry_set(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if self.status.get(Status::CARRY) {
            self.do_branch(dst);
        }
    }

    // BNE
    fn branch_if_not_zero(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if !self.status.get(Status::ZERO) {
            self.do_branch(dst);
        }
    }

    // BEQ
    fn branch_if_zero(&mut self, mode: &AddressingMode) {
        let dst = self.read_mem(mode);
        if self.status.get(Status::ZERO) {
            self.do_branch(dst);
        }
    }

    // ADC
    fn add_with_carry(&mut self, mode: &AddressingMode) {
        let operand = self.read_mem(mode);
        let (result, over1) = operand.overflowing_add(self.acc);
        let (result, over2) =
            result.overflowing_add(self.status.get(Status::CARRY) as u8);

        let over_carry = over1 || over2;
        self.status.set(over_carry, Status::CARRY);
        self.status.set(over_carry, Status::OVERFLOW);
        self.acc = result;
        self.update_flags(self.acc);
    }

    // AND
    fn and_with_acc(&mut self, mode: &AddressingMode) {
        let operand = self.read_mem(mode);
        self.acc &= operand;
        self.update_flags(self.acc);
    }

    // ASL
    fn shift_left(&mut self, mode: &AddressingMode) {
        let val = self.read_mem(mode);
        self.write_mem(mode, val << 1);
        self.update_flags(val);
        self.status.set(is_negative(val), Status::CARRY);
    }

    // BIT
    fn test_bits(&mut self, mode: &AddressingMode) {
        let operand = self.read_mem(mode);
        //println("-- Test bits {} & {}", operand, self.acc);
        self.status.set(bit_set!(operand, 6), Status::OVERFLOW);
        self.status.set(is_negative(operand), Status::NEGATIVE);
        self.status.set((self.acc & operand) == 0, Status::ZERO);
    }

    // BRK
    fn force_break(&mut self) {
        self.push16(self.pc.wrapping_add(2));
        self.push_status();
        self.status.set(true, Status::IRQ);
    }

    // CLC
    fn clear_carry(&mut self) {
        self.status.set(false, Status::CARRY);
    }

    // CLD
    fn clear_decimal(&mut self) {
        self.status.set(false, Status::DECIMAL);
    }

    // CLI
    fn clear_interrupt(&mut self) {
        self.status.set(false, Status::IRQ);
    }

    // CLV
    fn clear_overflow(&mut self) {
        self.status.set(false, Status::OVERFLOW);
    }

    // CMP
    fn cmp_with_acc(&mut self, mode: &AddressingMode) {
        let operand = self.read_mem(mode);
        let (result, carry) = self.acc.overflowing_sub(operand);
        self.status.set(carry, Status::CARRY);
        self.update_flags(result);
    }

    // CPX
    fn cmp_with_x(&mut self, mode: &AddressingMode) {
        let operand = self.read_mem(mode);
        let (result, carry) = self.x.overflowing_sub(operand);
        self.status.set(carry, Status::CARRY);
        self.update_flags(result);
    }

    // CPY
    fn cmp_with_y(&mut self, mode: &AddressingMode) {
        let operand = self.read_mem(mode);
        let (result, carry) = self.y.overflowing_sub(operand);
        self.status.set(carry, Status::CARRY);
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
        self.x = self.x.wrapping_sub(1);
        self.update_flags(self.x);
    }

    // DEY
    fn dec_y(&mut self) {
        self.y = self.y.wrapping_sub(1);
        self.update_flags(self.y);
    }

    // EOR
    fn xor_acc(&mut self, mode: &AddressingMode) {
        let mem = self.read_mem(mode);
        self.acc ^= mem;
        self.update_flags(self.acc);
    }

    // INC
    fn inc_mem(&mut self, mode: &AddressingMode) {
        let val = self.read_mem(mode).wrapping_add(1);
        self.write_mem(mode, val);
        self.update_flags(val);
    }

    // INX
    fn inc_x(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.update_flags(self.x);
    }

    // INY
    fn inc_y(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.update_flags(self.y);
    }

    // JMP
    fn jump_to(&mut self, mode: &AddressingMode) {
        let addr = self.get_addr(mode) as usize;
        //println("-- PC Destination from {:#X}", addr);
        self.pc = self.bus.read16(addr);
        //println("-- Jump to {}", self.pc);
    }

    // JSR
    fn jump_save_ret(&mut self, mode: &AddressingMode) {
        let pc = self.pc;
        self.push16(pc);
        let addr = self.get_addr(mode) as usize;
        self.pc = self.bus.read16(addr);
    }

    // LDA
    fn load_acc_with_mem(&mut self, mode: &AddressingMode) {
        let mem = self.read_mem(mode);
        self.acc = mem;
        self.update_flags(self.acc);
    }

    // LDX
    fn load_x_with_mem(&mut self, mode: &AddressingMode) {
        let mem = self.read_mem(mode);
        self.x = mem;
        self.update_flags(self.x);
    }

    // LDY
    fn load_y_with_mem(&mut self, mode: &AddressingMode) {
        let mem = self.read_mem(mode);
        self.y = mem;
        self.update_flags(self.y);
    }

    // LSR
    fn shift_right(&mut self, mode: &AddressingMode) {
        let mut mem = self.read_mem(mode);
        let carry = (mem & 0x01) != 0;
        mem >>= 1;
        self.write_mem(mode, mem);
        self.update_flags(mem);
        self.status.set(carry, Status::CARRY);
    }

    // NOP
    fn nop(&self) {}

    // ORA
    fn or_acc(&mut self, mode: &AddressingMode) {
        let mem = self.read_mem(mode);
        self.acc |= mem;
        self.update_flags(self.acc);
    }

    // PHA
    fn push_acc(&mut self) {
        //println("-- Pushing {} onto stack", self.acc);
        self.push8(self.acc);
    }

    // PHP
    fn push_status(&mut self) {
        self.push8(self.status.into());
    }

    // PLA
    fn pull_acc(&mut self) {
        self.acc = self.pop8();
    }

    // PLP
    fn pull_status(&mut self) {
        self.status = Status::from(self.pop8());
    }

    // ROL
    fn rotate_left(&mut self, mode: &AddressingMode) {
        let carry = self.status.get(Status::CARRY);
        let mem = self.read_mem(mode);
        self.status.set((mem & 0x80) != 0, Status::CARRY);
        let val = (mem << 1) | (carry as u8);
        self.write_mem(mode, val);
        self.update_flags(val);
    }

    // ROR
    fn rotate_right(&mut self, mode: &AddressingMode) {
        let carry = self.status.get(Status::CARRY);
        let mem = self.read_mem(mode);
        self.status.set((mem & 0x01) != 0, Status::CARRY);
        let val = (mem >> 1) | ((carry as u8) << 7);
        self.write_mem(mode, val);
        self.update_flags(val);
    }

    // RTI
    fn ret_from_interrupt(&mut self) {
        self.pull_status();
        self.pc = self.pop16();
    }

    // RTS
    fn ret_from_subr(&mut self) {
        self.pc = self.pop16().wrapping_add(1);
    }

    // SBC
    fn sub_with_carry(&mut self, mode: &AddressingMode) {
        let mem = self.read_mem(mode);
        let (result, over1) = self.acc.overflowing_sub(mem);
        let (result, over2) =
            result.overflowing_sub(self.status.get(Status::CARRY) as u8);

        let over_carry = over1 || over2;
        self.status.set(over_carry, Status::CARRY);
        self.status.set(over_carry, Status::OVERFLOW);
        self.acc = result;
        self.update_flags(self.acc);
    }

    // SEC
    fn set_carry(&mut self) {
        self.status.set(true, Status::CARRY);
    }

    // SED
    fn set_decimal(&mut self) {
        self.status.set(true, Status::DECIMAL);
    }

    // SEI
    fn set_interrupt(&mut self) {
        self.status.set(true, Status::IRQ);
    }

    // STA
    fn store_acc_mem(&mut self, mode: &AddressingMode) {
        self.write_mem(mode, self.acc);
    }

    // STX
    fn store_x_mem(&mut self, mode: &AddressingMode) {
        self.write_mem(mode, self.x);
    }

    // STY
    fn store_y_mem(&mut self, mode: &AddressingMode) {
        self.write_mem(mode, self.y);
    }

    // TAX
    fn tx_acc_to_x(&mut self) {
        self.x = self.acc;
        self.update_flags(self.x);
    }

    // TAY
    fn tx_acc_to_y(&mut self) {
        self.y = self.acc;
        self.update_flags(self.y);
    }

    // TSX
    fn tx_sp_to_x(&mut self) {
        self.x = self.sp;
        self.update_flags(self.x);
    }

    // TXA
    fn tx_x_to_acc(&mut self) {
        self.acc = self.x;
        self.update_flags(self.acc);
    }

    // TXS
    fn tx_x_to_sp(&mut self) {
        self.sp = self.x;
    }

    // TYA
    fn tx_y_to_acc(&mut self) {
        self.acc = self.y;
        self.update_flags(self.acc);
    }
}

impl Clocked for Ricoh2A03<'_> {
    fn clock(&mut self) {
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

        let opcode = self.bus.read(self.pc as usize);
        let instr = instructions::get_from(opcode);
        self.noop_cycles = instr.cycles() - 1; // 1 cycle we use to execute the instruction
                                               //        println!(
                                               //            "-- Running {:#X} {:?} from {:#X} for {} cycles",
                                               //            opcode,
                                               //            &instr,
                                               //            self.pc,
                                               //            self.noop_cycles + 1
                                               //        );
        self.incr_pc(1);

        match instr.name() {
            // BRANCHES
            BPL => self.branch_if_pos(&instr.mode()),
            BMI => self.branch_if_neg(&instr.mode()),
            BVC => self.branch_if_overflow_clear(&instr.mode()),
            BVS => self.branch_if_overflow_set(&instr.mode()),
            BCC => self.branch_if_carry_clear(&instr.mode()),
            BCS => self.branch_if_carry_set(&instr.mode()),
            BNE => self.branch_if_not_zero(&instr.mode()),
            BEQ => self.branch_if_zero(&instr.mode()),
            ADC => self.add_with_carry(&instr.mode()),
            AND => self.and_with_acc(&instr.mode()),
            SBC => self.sub_with_carry(&instr.mode()),
            ORA => self.or_acc(&instr.mode()),
            LDY => self.load_y_with_mem(&instr.mode()),
            LDX => self.load_x_with_mem(&instr.mode()),
            LDA => self.load_acc_with_mem(&instr.mode()),
            EOR => self.xor_acc(&instr.mode()),
            CPY => self.cmp_with_y(&instr.mode()),
            CPX => self.cmp_with_x(&instr.mode()),
            CMP => self.cmp_with_acc(&instr.mode()),
            BIT => self.test_bits(&instr.mode()),

            ASL => self.shift_left(&instr.mode()),
            LSR => self.shift_right(&instr.mode()),
            JSR => self.jump_save_ret(&instr.mode()),
            JMP => self.jump_to(&instr.mode()),
            STY => self.store_y_mem(&instr.mode()),
            STX => self.store_x_mem(&instr.mode()),
            STA => self.store_acc_mem(&instr.mode()),
            ROL => self.rotate_left(&instr.mode()),
            ROR => self.rotate_right(&instr.mode()),
            INC => self.inc_mem(&instr.mode()),
            DEC => self.dec_mem(&instr.mode()),

            CLV => self.clear_overflow(),
            CLI => self.clear_interrupt(),
            CLC => self.clear_carry(),
            CLD => self.clear_decimal(),
            DEX => self.dec_x(),
            DEY => self.dec_y(),
            INY => self.inc_y(),
            INX => self.inc_x(),
            TAY => self.tx_acc_to_y(),
            TAX => self.tx_acc_to_x(),
            TYA => self.tx_y_to_acc(),
            TXA => self.tx_x_to_acc(),
            TXS => self.tx_x_to_sp(),
            TSX => self.tx_sp_to_x(),
            SEI => self.set_interrupt(),
            SED => self.set_decimal(),
            SEC => self.set_carry(),
            RTS => self.ret_from_subr(),
            RTI => self.ret_from_interrupt(),
            PLP => self.pull_status(),
            PLA => self.pull_acc(),
            PHP => self.push_status(),
            PHA => self.push_acc(),
            BRK => self.force_break(),

            NOP => self.nop(),
            _ => {
                let last_pc = self.pc - 1;
                unreachable!(
                    "-- Invalid Instruction. Surrounding instructions: {:?}",
                    self.bus.read_n((last_pc - 2) as usize, 5)
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
        // todo!("Need to clock everything timed by the CPU clock, but not execute instructions
        // until the previous is fully processed");
        self.bus.clock();
    }
}

#[cfg(test)]
#[path = "./tcpu.rs"]
mod tests;
