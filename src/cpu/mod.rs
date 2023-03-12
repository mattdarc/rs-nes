pub mod instructions;
mod status;

use {
    crate::bus::Bus,
    crate::ExitStatus,
    instructions::Instruction,
    status::Status,
    std::cell::RefCell,
    std::stringify,
    tracing::{event, span, Level},
};

#[inline]
fn is_negative(v: u8) -> bool {
    is_bit_set(v, 7)
}

#[inline]
fn is_bit_set(v: u8, bit: u8) -> bool {
    (v & (1 << bit)) != 0
}

#[inline]
fn crosses_page(src: u16, dst: u16) -> bool {
    (src & 0xFF00) != (dst & 0xFF00)
}

const STACK_BEGIN: u16 = 0x0100;
const IRQ_HANDLER_ADDR: u16 = 0xFFFA;
pub const NTSC_CLOCK: u32 = 1_789_773;
pub const PAL_CLOCK: u32 = 1_662_607;

// Exported for use in tests
pub const RESET_VECTOR_START: u16 = 0xFFFC;

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
                CpuStateBuilder::default()
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

buildable!(CpuState; CpuStateBuilder {
    cycles: usize,
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

pub type CpuTask<'a> = Box<dyn FnMut(&dyn CpuInterface) + 'a>;
type TaskList<'a> = RefCell<Vec<CpuTask<'a>>>;

pub trait CpuInterface {
    fn read_state(&self) -> CpuState;
}

impl<'a, BusType: Bus> CpuInterface for CPU<'a, BusType> {
    fn read_state(&self) -> CpuState {
        let (scanline, ppu_cycle) = self.bus.ppu_state();

        CpuState {
            cycles: self.bus.cycles(),
            instruction: self.instruction,
            operands: self.operands.clone(),
            acc: self.acc,
            x: self.x,
            y: self.y,
            pc: self.old_pc,
            sp: self.sp,
            status: self.status.to_u8(),
            scanline: scanline + 1, // nestest starts the PPU at the first scanline
            ppu_cycle,
        }
    }
}

pub struct CPU<'a, BusType: Bus> {
    bus: BusType,

    acc: u8,
    x: u8,
    y: u8,
    old_pc: u16,
    pc: u16,
    sp: u8,
    status: Status,

    operands: Vec<u8>,
    instruction: Instruction,
    cycles: u8,

    instructions_executed: usize,
    exit_status: ExitStatus,

    pre_execute_tasks: TaskList<'a>,
    post_execute_tasks: TaskList<'a>,
}

impl<'a, BusType: Bus> CPU<'a, BusType> {
    pub fn new(bus: BusType) -> Self {
        CPU {
            bus,
            acc: 0,
            x: 0,
            y: 0,
            old_pc: 0,
            pc: 0,
            sp: 0xFD,
            status: Status::empty(),
            instruction: Instruction::nop(),
            exit_status: ExitStatus::Continue,
            operands: Vec::new(),
            cycles: 0,
            instructions_executed: 0,
            pre_execute_tasks: TaskList::new(Vec::new()),
            post_execute_tasks: TaskList::new(Vec::new()),
        }
    }

    pub fn add_pre_execute_task(&mut self, task: CpuTask<'a>) {
        self.pre_execute_tasks.borrow_mut().push(task);
    }

    pub fn add_post_execute_task(&mut self, task: CpuTask<'a>) {
        self.post_execute_tasks.borrow_mut().push(task);
    }

    fn run_pre_execute_tasks(&mut self) {
        for task in self.pre_execute_tasks.borrow_mut().iter_mut() {
            task(self);
        }
    }

    fn run_post_execute_tasks(&mut self) {
        for task in self.post_execute_tasks.borrow_mut().iter_mut() {
            task(self);
        }
    }

    pub fn pc(&self) -> u16 {
        self.pc
    }

    pub fn nestest_reset_override(&mut self, pc: u16) {
        event!(Level::DEBUG, "reset PC {:#x} -> {:#x}", self.pc, pc);
        self.pc = pc;
        self.status = Status::default();
        self.sp = 0xFD;

        // nestest starts with 7 cycles initially... not sure why
        self.bus.clock(7);
    }

    pub fn reset(&mut self) {
        let pc = self.bus.read16(RESET_VECTOR_START);
        event!(Level::DEBUG, "reset PC {:#x} -> {:#x}", self.pc, pc);

        self.pc = pc;
        self.status = Status::default();
        self.sp = 0xFD;
    }

    pub fn clock(&mut self) -> ExitStatus {
        let cpu_span = span!(
            target: "cpu",
            Level::TRACE,
            "clock",
            cycles = self.bus.cycles()
        );
        {
            let _enter = cpu_span.enter();

            if let Some(status) = self.bus.pop_nmi() {
                self.handle_nmi(status);
            } else {
                self.fetch_instruction();

                self.run_pre_execute_tasks();
                self.execute_instruction();
                self.run_post_execute_tasks();

                self.instructions_executed += 1;
            }
        }

        self.bus.clock(self.cycles);

        self.exit_status.clone()
    }

    fn handle_nmi(&mut self, _status: u8) {
        self.push16(self.pc);
        self.push8(self.status.bits());
        self.status.set(Status::INT_DISABLE, true);

        // Load address of interrupt handler, set PC to execute there
        self.bus.clock(2);
        self.pc = self.bus.read16(IRQ_HANDLER_ADDR);
        event!(Level::TRACE, "IRQ: {:#04X}", self.pc);
    }

    fn trace_instruction(&self) {
        let (scanline, ppu_cycle) = self.bus.ppu_state();

        let operands_as_str = || {
            // FIXME: This should be a stack-allocated string. In the hot loop like this we're
            // waiting on malloc for most of the time
            let mut operands_str = String::new();
            for op in &self.operands {
                operands_str.push_str(&format!("{:02X} ", op));
            }

            operands_str
        };

        event!(
            Level::DEBUG,
            "[{:>10}]  {:<04X}  {:<2X} {:<8} {:<4}  {:>10}  A:{:02X}  X:{:02X}  Y:{:02X}  P:{:02X}  SP:{:02X}  CYC:{:>3}  SL:{:>3}",
            self.instructions_executed,
            self.old_pc,
            self.instruction.opcode(),
            operands_as_str(),
            self.instruction.name(),
            format!("{:?}", self.instruction.mode()),
            self.acc,
            self.x,
            self.y,
            self.status.bits(),
            self.sp,
            ppu_cycle,
            scanline,
        );
    }

    fn fetch_instruction(&mut self) {
        let opcode = self.bus.read(self.pc);
        self.instruction = instructions::decode_instruction(opcode);

        // 1 cycle we use to execute the instruction
        self.cycles = self.instruction.cycles();

        let num_operands = (self.instruction.size() - 1) as usize;
        self.operands.resize(num_operands, 0);
        for i in 0..num_operands {
            self.operands[i] = self.bus.read(self.pc + (i as u16) + 1);
        }

        self.old_pc = self.pc;
        self.pc += self.instruction.size();
    }

    fn execute_instruction(&mut self) {
        use instructions::InstrName::*;

        // FIXME: The address calculation and "real" operand decoding should be moved to before the
        // fetch so we can print them
        self.trace_instruction();
        match self.instruction.name() {
            // BRANCHES
            BPL => self.bpl(),
            BMI => self.bmi(),
            BVC => self.bvc(),
            BVS => self.bvs(),
            BCC => self.bcc(),
            BCS => self.bcs(),
            BNE => self.bne(),
            BEQ => self.beq(),
            ADC => self.adc(),
            AND => self.and(),
            SBC => self.sbc(),
            ORA => self.ora(),
            LDY => self.ldy(),
            LDX => self.ldx(),
            LDA => self.lda(),
            EOR => self.eor(),
            CPY => self.cpy(),
            CPX => self.cpx(),
            CMP => self.cmp(),
            BIT => self.bit(),

            ASL => self.asl(),
            LSR => self.lsr(),
            JSR => self.jsr(),
            JMP => self.jmp(),
            STY => self.sty(),
            STX => self.stx(),
            STA => self.sta(),
            ROL => self.rol(),
            ROR => self.ror(),
            INC => self.inc(),
            DEC => self.dec(),

            CLV => self.clv(),
            CLI => self.cli(),
            CLC => self.clc(),
            CLD => self.cld(),
            DEX => self.dex(),
            DEY => self.dey(),
            INY => self.iny(),
            INX => self.inx(),
            TAY => self.tay(),
            TAX => self.tax(),
            TYA => self.tya(),
            TXA => self.txa(),
            TXS => self.txs(),
            TSX => self.tsx(),
            SEI => self.sei(),
            SED => self.sed(),
            SEC => self.sec(),
            RTS => self.rts(),
            RTI => self.rti(),
            PLP => self.plp(),
            PLA => self.pla(),
            PHP => self.php(),
            PHA => self.pha(),
            BRK => self.brk(),

            ILLEGAL_JAM => self.hlt(),
            ILLEGAL_SLO => self.slo(),
            ILLEGAL_RLA => self.rla(),
            ILLEGAL_SRE => self.sre(),
            ILLEGAL_RRA => self.rra(),
            ILLEGAL_SAX => self.sax(),
            ILLEGAL_SHA => self.sha(),
            ILLEGAL_LAX => self.lax(),
            ILLEGAL_DCP => self.dcp(),
            ILLEGAL_ISC => self.isc(),
            ILLEGAL_ANC => self.anc(),
            ILLEGAL_ALR => self.alr(),
            ILLEGAL_ARR => self.arr(),
            ILLEGAL_ANE => self.ane(),
            ILLEGAL_TAS => self.tas(),
            ILLEGAL_LXA => self.lxa(),
            ILLEGAL_LAS => self.las(),
            ILLEGAL_SBX => self.sbx(),
            ILLEGAL_USBC => self.usbc(),
            ILLEGAL_SHY => self.shy(),
            ILLEGAL_SHX => self.shx(),

            ILLEGAL_NOP | NOP => self.nop(),
        }
    }

    fn hlt(&self) -> ! {
        panic!("HLT");
    }

    fn takes_extra_cycle(&mut self, start_addr: u16, end_addr: u16) -> bool {
        use instructions::InstrName;

        match self.instruction.name() {
            InstrName::STA
            | InstrName::ILLEGAL_ALR
            | InstrName::ILLEGAL_ANC
            | InstrName::ILLEGAL_ANE
            | InstrName::ILLEGAL_ARR
            | InstrName::ILLEGAL_DCP
            | InstrName::ILLEGAL_ISC
            | InstrName::ILLEGAL_LXA
            | InstrName::ILLEGAL_RLA
            | InstrName::ILLEGAL_RRA
            | InstrName::ILLEGAL_SAX
            | InstrName::ILLEGAL_SBX
            | InstrName::ILLEGAL_SHA
            | InstrName::ILLEGAL_SHX
            | InstrName::ILLEGAL_SHY
            | InstrName::ILLEGAL_SLO
            | InstrName::ILLEGAL_SRE
            | InstrName::ILLEGAL_TAS
            | InstrName::ILLEGAL_USBC => false,
            _ => crosses_page(start_addr, end_addr),
        }
    }

    fn do_branch(&mut self, dst: u8) {
        // FIXME: we can likely implement this as i8
        let next_pc = if is_negative(dst) {
            self.pc.wrapping_sub(dst.wrapping_neg() as u16)
        } else {
            self.pc.wrapping_add(dst as u16)
        };
        let crossed_page = crosses_page(self.pc, next_pc);

        // Crossing a page adds an extra cycle
        self.cycles += 1 + crossed_page as u8;
        self.pc = next_pc;
    }

    fn peek(&mut self) -> u8 {
        let ptr = (self.sp as u16).wrapping_add(STACK_BEGIN);
        self.bus.read(ptr)
    }

    fn poke(&mut self, val: u8) {
        let ptr = (self.sp as u16).wrapping_add(STACK_BEGIN);
        self.bus.write(ptr, val);
    }

    // Update the CPU flags based on the accumulator
    fn update_nz(&mut self, v: u8) {
        self.status.set(Status::NEGATIVE, is_negative(v));
        self.status.set(Status::ZERO, v == 0);
    }

    fn push16(&mut self, v: u16) {
        self.push8((v >> 8) as u8);
        self.push8((0xFF & v) as u8);
    }

    fn push8(&mut self, v: u8) {
        self.poke(v);
        self.sp = self.sp.wrapping_sub(1);
        assert!(self.sp != 0xFF, "Stack overflow!");
    }

    fn pop16(&mut self) -> u16 {
        let low = self.pop8() as u16;
        ((self.pop8() as u16) << 8) | low
    }

    fn pop8(&mut self) -> u8 {
        assert!(self.sp != 0xFF, "Tried to pop empty stack!");
        self.sp = self.sp.wrapping_add(1);
        self.peek()
    }

    fn calc_addr(&mut self) -> u16 {
        use instructions::AddressingMode::*;

        let op_or_zero = |i| {
            if self.operands.len() > i {
                self.operands[i]
            } else {
                0
            }
        };

        let addr_lo = op_or_zero(0);
        let addr_hi = op_or_zero(1);
        let addr = ((addr_hi as u16) << 8) | addr_lo as u16;

        match self.instruction.mode() {
            ZeroPage => addr_lo as u16,
            ZeroPageX => addr_lo.wrapping_add(self.x) as u16,
            ZeroPageY => addr_lo.wrapping_add(self.y) as u16,
            Absolute => addr,
            AbsoluteX => {
                let addr_x = addr.wrapping_add(self.x as u16);
                self.cycles += self.takes_extra_cycle(addr, addr_x) as u8;
                addr_x
            }
            AbsoluteY => {
                let addr_y = addr.wrapping_add(self.y as u16);
                self.cycles += self.takes_extra_cycle(addr, addr_y) as u8;
                addr_y
            }
            Indirect => self.bus.read16(addr),
            IndirectX => self.bus.read16(addr_lo.wrapping_add(self.x) as u16),
            IndirectY => {
                let addr_without_offset = self.bus.read16(addr_lo as u16);
                let addr = addr_without_offset.wrapping_add(self.y as u16);

                self.cycles += self.takes_extra_cycle(addr_without_offset, addr) as u8;
                addr
            }
            _ => u16::MAX,
        }
    }

    fn get_operand(&mut self) -> u8 {
        use instructions::AddressingMode::*;
        match &self.instruction.mode() {
            Accumulator => self.acc,
            Immediate | Relative => self.operands[0],
            _ => {
                let addr = self.calc_addr();
                self.bus.read(addr)
            }
        }
    }

    fn write_memory(&mut self, addr: TargetAddress, val: u8) {
        match addr {
            TargetAddress::Memory(addr) => self.bus.write(addr, val),
            TargetAddress::Accumulator => self.acc = val,
            TargetAddress::None => panic!("Writing to invalid target address"),
        }
    }

    fn read_memory(&mut self) -> (TargetAddress, u8) {
        use instructions::AddressingMode::*;
        match &self.instruction.mode() {
            Accumulator => (TargetAddress::Accumulator, self.acc),
            Immediate | Relative => (TargetAddress::None, self.operands[0]),
            _ => {
                let addr = self.calc_addr();
                (TargetAddress::Memory(addr), self.bus.read(addr))
            }
        }
    }

    fn add_with_carry_and_overflow(&mut self, a: u8, b: u8) -> u8 {
        let carry = self.status.contains(Status::CARRY);
        let (result, carry1) = a.overflowing_add(b);
        let (result, carry2) = result.overflowing_add(carry as u8);

        let overflow = (a ^ b) & 0x80 == 0 && (b ^ result) & 0x80 != 0;
        let carry = carry1 || carry2;

        self.status.set(Status::OVERFLOW, overflow);
        self.status.set(Status::CARRY, carry);
        result
    }

    fn sub_with_carry_and_overflow(&mut self, a: u8, b: u8) -> u8 {
        let carry = self.status.contains(Status::CARRY);
        let result = a.wrapping_sub(b).wrapping_sub(!carry as u8);

        // result is positive if acc is negative and operand is positive
        //              --OR--
        // result is negative if acc is positive operand is negative
        let overflow = ((result ^ b) & 0x80) == 0 && ((b ^ a) & 0x80) != 0;

        // Carry (not borrow) happens if a >= b where a - b
        let carry = a > b || (a == b && carry);

        self.status.set(Status::CARRY, carry);
        self.status.set(Status::OVERFLOW, overflow);

        result
    }

    // BRANCHES:
    fn bpl(&mut self) {
        let dst = self.get_operand();
        if !self.status.contains(Status::NEGATIVE) {
            self.do_branch(dst);
        }
    }

    fn bmi(&mut self) {
        let dst = self.get_operand();
        if self.status.contains(Status::NEGATIVE) {
            self.do_branch(dst);
        }
    }

    fn bvc(&mut self) {
        let dst = self.get_operand();
        if !self.status.contains(Status::OVERFLOW) {
            self.do_branch(dst);
        }
    }

    fn bvs(&mut self) {
        let dst = self.get_operand();
        if self.status.contains(Status::OVERFLOW) {
            self.do_branch(dst);
        }
    }

    fn bcc(&mut self) {
        let dst = self.get_operand();
        if !self.status.contains(Status::CARRY) {
            self.do_branch(dst);
        }
    }

    fn bcs(&mut self) {
        let dst = self.get_operand();
        if self.status.contains(Status::CARRY) {
            self.do_branch(dst);
        }
    }

    fn bne(&mut self) {
        let dst = self.get_operand();
        if !self.status.contains(Status::ZERO) {
            self.do_branch(dst);
        }
    }

    fn beq(&mut self) {
        let dst = self.get_operand();
        if self.status.contains(Status::ZERO) {
            self.do_branch(dst);
        }
    }

    fn adc(&mut self) {
        let operand = self.get_operand();
        self.acc = self.add_with_carry_and_overflow(self.acc, operand);
        self.update_nz(self.acc);
    }

    fn and(&mut self) {
        self.acc &= self.get_operand();
        self.update_nz(self.acc);
    }

    fn bit(&mut self) {
        let operand = self.get_operand();
        self.status.set(Status::OVERFLOW, is_bit_set(operand, 6));
        self.status.set(Status::NEGATIVE, is_negative(operand));
        self.status.set(Status::ZERO, (self.acc & operand) == 0);
    }

    fn brk(&mut self) {
        self.push16(self.pc.wrapping_add(2));
        self.php();
        self.status.set(Status::INT_DISABLE, true);
        self.exit_status = ExitStatus::ExitInterrupt;
    }

    fn clc(&mut self) {
        self.status.set(Status::CARRY, false);
    }

    fn cld(&mut self) {
        self.status.set(Status::DECIMAL, false);
    }

    fn cli(&mut self) {
        self.status.set(Status::INT_DISABLE, false);
    }

    fn clv(&mut self) {
        self.status.set(Status::OVERFLOW, false);
    }

    fn cmp(&mut self) {
        let operand = self.get_operand();
        self.status.set(Status::CARRY, self.acc >= operand);

        let result = self.acc.wrapping_sub(operand);
        self.update_nz(result);
    }

    fn cpx(&mut self) {
        let operand = self.get_operand();
        self.status.set(Status::CARRY, self.x >= operand);

        let result = self.x.wrapping_sub(operand);
        self.update_nz(result);
    }

    fn cpy(&mut self) {
        let operand = self.get_operand();
        self.status.set(Status::CARRY, self.y >= operand);

        let result = self.y.wrapping_sub(operand);
        self.update_nz(result);
    }

    fn dec(&mut self) {
        let addr = self.calc_addr();
        let result = self.bus.read(addr).wrapping_sub(1);

        self.bus.write(addr, result);
        self.update_nz(result);
    }

    fn dex(&mut self) {
        self.x = self.x.wrapping_sub(1);
        self.update_nz(self.x);
    }

    fn dey(&mut self) {
        self.y = self.y.wrapping_sub(1);
        self.update_nz(self.y);
    }

    fn eor(&mut self) {
        let operand = self.get_operand();
        self.acc ^= operand;
        self.update_nz(self.acc);
    }

    fn inc(&mut self) {
        let addr = self.calc_addr();
        let result = self.bus.read(addr).wrapping_add(1);
        self.bus.write(addr, result);
        self.update_nz(result);
    }

    fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.update_nz(self.x);
    }

    fn iny(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.update_nz(self.y);
    }

    fn jmp(&mut self) {
        let addr = self.calc_addr();
        self.pc = addr;
    }

    fn jsr(&mut self) {
        let pc = self.pc - 1;
        self.push16(pc);
        self.pc = ((self.operands[1] as u16) << 8) | (self.operands[0] as u16);
    }

    fn rts(&mut self) {
        self.pc = self.pop16() + 1;
    }

    fn lda(&mut self) {
        self.acc = self.get_operand();
        self.update_nz(self.acc);
    }

    fn ldx(&mut self) {
        self.x = self.get_operand();
        self.update_nz(self.x);
    }

    fn ldy(&mut self) {
        self.y = self.get_operand();
        self.update_nz(self.y);
    }

    fn lsr(&mut self) {
        let (addr, operand) = self.read_memory();

        self.status.set(Status::CARRY, operand & 0x01 != 0);
        let shift = operand >> 1;

        self.write_memory(addr, shift);
        self.update_nz(shift);
    }

    fn asl(&mut self) {
        let (addr, operand) = self.read_memory();

        self.status.set(Status::CARRY, operand & 0x80 != 0);
        let shift = operand << 1;

        self.write_memory(addr, shift);
        self.update_nz(shift);
    }

    fn nop(&mut self) {
        // Some (illegal) NOPs have an extra cycle due to the address calculation. use calc_addr to
        // account for this. TODO this could probably be made clearer, but the check requires us to
        // inspect the intermediate stages of the address calculation
        let _ = self.calc_addr();
    }

    fn ora(&mut self) {
        self.acc |= self.get_operand();
        self.update_nz(self.acc);
    }

    fn pha(&mut self) {
        self.push8(self.acc);
    }

    fn pla(&mut self) {
        self.acc = self.pop8();
        self.update_nz(self.acc);
    }

    fn php(&mut self) {
        self.push8((self.status | Status::BRK).bits());
    }

    fn plp(&mut self) {
        self.status =
            Status::from_bits(self.pop8() & !Status::BRK.bits() | Status::PUSH_IRQ.bits())
                .expect("All bits are covered in Status");
    }

    fn rol(&mut self) {
        let (addr, operand) = self.read_memory();

        let carry = self.status.contains(Status::CARRY);
        self.status.set(Status::CARRY, (operand & 0x80) != 0);
        let shift = (operand << 1) | (carry as u8);

        self.write_memory(addr, shift);
        self.update_nz(shift);
    }

    fn ror(&mut self) {
        let (addr, operand) = self.read_memory();

        let carry = self.status.contains(Status::CARRY);
        self.status.set(Status::CARRY, (operand & 0x01) != 0);
        let shift = (operand >> 1) | ((carry as u8) << 7);

        self.write_memory(addr, shift);
        self.update_nz(shift);
    }

    fn rti(&mut self) {
        self.plp();
        self.pc = self.pop16();
    }

    fn sbc(&mut self) {
        let operand = self.get_operand();
        self.acc = self.sub_with_carry_and_overflow(self.acc, operand);
        self.update_nz(self.acc);
    }

    fn sec(&mut self) {
        self.status.set(Status::CARRY, true);
    }

    fn sed(&mut self) {
        self.status.set(Status::DECIMAL, true);
    }

    fn sei(&mut self) {
        self.status.set(Status::INT_DISABLE, true);
    }

    fn sta(&mut self) {
        let addr = self.calc_addr();
        self.bus.write(addr, self.acc);
    }

    fn stx(&mut self) {
        let addr = self.calc_addr();
        self.bus.write(addr, self.x);
    }

    fn sty(&mut self) {
        let addr = self.calc_addr();
        self.bus.write(addr, self.y);
    }

    fn tax(&mut self) {
        self.x = self.acc;
        self.update_nz(self.x);
    }

    fn tay(&mut self) {
        self.y = self.acc;
        self.update_nz(self.y);
    }

    fn tsx(&mut self) {
        self.x = self.sp;
        self.update_nz(self.x);
    }

    fn txa(&mut self) {
        self.acc = self.x;
        self.update_nz(self.acc);
    }

    fn txs(&mut self) {
        self.sp = self.x;
    }

    fn tya(&mut self) {
        self.acc = self.y;
        self.update_nz(self.acc);
    }

    // Illegal instructions
    fn alr(&mut self) {
        let addr = self.calc_addr();
        let operand = self.bus.read(addr);

        self.status.set(Status::CARRY, operand & 0x01 != 0);
        let shift = operand >> 1;

        self.acc &= operand;
        self.bus.write(addr, shift);
        self.update_nz(shift);
    }

    // TODO this doesn't seem right
    fn anc(&mut self) {
        let addr = self.calc_addr();
        let operand = self.bus.read(addr);

        self.status.set(Status::CARRY, operand & 0x80 != 0);
        let shift = operand << 1;

        self.acc &= operand;
        self.bus.write(addr, shift);
        self.update_nz(self.acc);
    }

    fn ane(&mut self) {
        panic!("unstable ANE");
    }

    fn arr(&mut self) {
        let addr = self.calc_addr();
        let operand = self.bus.read(addr);

        let carry = self.status.contains(Status::CARRY);
        self.status.set(Status::CARRY, (operand & 0x01) != 0);
        let result = (operand >> 1) | ((carry as u8) << 7);

        self.acc &= operand;
        self.bus.write(addr, result);
        self.update_nz(self.acc);
    }

    fn dcp(&mut self) {
        let addr = self.calc_addr();
        let dec = self.bus.read(addr).wrapping_sub(1);
        self.bus.write(addr, dec);

        let result = self.acc.wrapping_sub(dec);
        self.update_nz(result);
        self.status.set(Status::CARRY, self.acc >= dec);
    }

    fn isc(&mut self) {
        let addr = self.calc_addr();
        let result = self.bus.read(addr).wrapping_add(1);
        self.bus.write(addr, result);
        self.acc = self.sub_with_carry_and_overflow(self.acc, result);
        self.update_nz(self.acc);
    }

    fn las(&mut self) {
        let operand = self.get_operand();

        self.acc = operand;
        self.x = self.sp;
        self.update_nz(self.x);
    }

    fn lax(&mut self) {
        let addr = self.calc_addr();
        let operand = self.bus.read(addr);

        self.acc = operand;
        self.x = operand;
        self.update_nz(self.acc);
    }

    fn lxa(&mut self) {
        panic!("unstable LXA");
    }

    fn rla(&mut self) {
        let addr = self.calc_addr();
        let operand = self.bus.read(addr);

        let carry = self.status.contains(Status::CARRY);
        self.status.set(Status::CARRY, (operand & 0x80) != 0);
        let shift = (operand << 1) | (carry as u8);

        self.bus.write(addr, shift);
        self.acc &= shift;
        self.update_nz(self.acc);
    }

    fn rra(&mut self) {
        let addr = self.calc_addr();
        let operand = self.bus.read(addr);

        let carry = self.status.contains(Status::CARRY);
        self.status.set(Status::CARRY, (operand & 0x01) != 0);

        let shift = (operand >> 1) | ((carry as u8) << 7);
        self.bus.write(addr, shift);

        self.acc = self.add_with_carry_and_overflow(self.acc, shift);
        self.update_nz(self.acc);
    }

    fn sax(&mut self) {
        let ax = self.acc & self.x;
        let addr = self.calc_addr();
        self.bus.write(addr, ax);
    }

    fn sbx(&mut self) {
        let operand = self.get_operand();
        let ax = self.acc & self.x;

        self.x = ax - operand;
        self.update_nz(self.x);
    }

    fn sha(&mut self) {
        let ax = self.acc & self.x;
        let addr = self.calc_addr();
        let high = ((addr >> 8) + 1) as u8;
        self.bus.write(addr, ax & high);
    }

    fn shx(&mut self) {
        let addr = self.calc_addr();
        let high_x = self.x & ((addr >> 8) + 1) as u8;
        self.bus.write(addr, high_x);
    }

    fn shy(&mut self) {
        let addr = self.calc_addr();
        let high_y = self.y & ((addr >> 8) + 1) as u8;
        self.bus.write(addr, high_y);
    }

    fn slo(&mut self) {
        let addr = self.calc_addr();
        let mem = self.bus.read(addr);
        self.status.set(Status::CARRY, mem & 0x80 != 0);
        let shift = mem << 1;
        self.bus.write(addr, shift);

        self.acc |= shift;
        self.update_nz(self.acc);
    }

    fn sre(&mut self) {
        let addr = self.calc_addr();
        let mem = self.bus.read(addr);
        self.status.set(Status::CARRY, mem & 0x1 != 0);
        let shift = mem >> 1;
        self.bus.write(addr, shift);

        self.acc ^= shift;
        self.update_nz(self.acc);
    }

    fn tas(&mut self) {
        let ax = self.acc & self.x;
        self.push8(ax);
        let addr = self.calc_addr();
        let high = ((addr + 1) >> 8) as u8;
        self.bus.write(addr, ax & high);
    }

    fn usbc(&mut self) {
        self.sbc();
    }
}

#[cfg(test)]
mod tests;
