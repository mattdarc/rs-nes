mod instructions;
mod status;

use {
    crate::bus::Bus,
    crate::ExitStatus,
    instructions::{decode_instruction, Instruction},
    status::Status,
    std::fs::File,
    std::io::prelude::*,
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

pub struct CPU<BusType: Bus> {
    acc: u8,
    x: u8,
    y: u8,
    pc: u16,
    sp: u8,
    status: Status,
    bus: BusType,

    // cache them in the NES for logging
    operands: [u8; 2],
    instruction: Instruction,
    cycles: u8,
    reset_vector: u16,

    exit_status: ExitStatus,
    log_file: Option<File>,
    logging_enabled: bool,
}

impl<BusType: Bus> CPU<BusType> {
    pub fn new(bus: BusType, reset_vector: u16) -> Self {
        event!(Level::DEBUG, "new cpu: reset vector 0x{:04X}", reset_vector);
        CPU {
            acc: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: 0xFD,
            status: Status::empty(),
            bus,
            instruction: Instruction::nop(),
            exit_status: ExitStatus::Continue,
            operands: [0; 2],
            cycles: 0,
            reset_vector,

            log_file: None,
            logging_enabled: false,
        }
    }

    pub fn reset_override(&mut self, pc: u16) {
        event!(Level::DEBUG, "reset to vector 0x{:04}", self.reset_vector);
        self.pc = pc
    }

    pub fn reset(&mut self) {
        event!(Level::DEBUG, "reset to vector 0x{:04}", self.reset_vector);
        self.pc = self.bus.read16(self.reset_vector)
    }

    pub fn clock(&mut self) -> ExitStatus {
        let _enter = span!(Level::TRACE, "Clock", cycles = self.bus.cycles());
        self.execute_instruction();
        if let Some(status) = self.bus.pop_nmi() {
            event!(Level::INFO, NMI.status = status);
            self.handle_nmi(status);
        }

        self.bus.clock(self.cycles);
        self.exit_status.clone()
    }

    pub fn log_cpu_state(&mut self) {
        const LOG_CPU_STATE: bool = true;
        if !LOG_CPU_STATE {
            return;
        }

        let log_file = self.log_file.get_or_insert_with(|| {
            File::create("test/nestest.log").expect("Error creating log file")
        });

        let opcode = self.instruction.opcode();
        let instruction = decode_instruction(opcode);
        let mut operand_str = String::new();
        for i in 0..instruction.size() - 1 {
            operand_str += &format!("{:0>2X} ", self.operands[i as usize]);
        }

        // NOTE: Do not modify this. It is in the same format as the nestest log
        // pc instr arg0 arg1 decoded
        // C000  4C F5 C5  JMP $C5F5                       A:00 X:00 Y:00 P:24 SP:FD PPU:  0, 21 CYC:7
        let cpu_state = format!(
            "{:0>4X}  {:0>2X} {:<6} {:?}                             A:{:0>2X} X:{:0>2X} Y:{:0>2X} P:{:0>2X} SP:{:0>2X}             CYC:{}\n",
            self.pc,
            opcode,
            operand_str,
            self.instruction.name(),
            self.acc,
            self.x,
            self.y,
            self.status.bits(),
            self.sp,
            self.bus.cycles(),
        );
        log_file
            .write_all(cpu_state.as_bytes())
            .or_else::<std::io::Error, _>(|io_err: std::io::Error| {
                event!(Level::ERROR, "Failed to write to file\n\t {:?}", io_err);
                Ok(())
            })
            .unwrap();
    }

    fn handle_nmi(&mut self, _status: u8) {
        let _enter_nmi = span!(
            Level::TRACE,
            "Handling NMI",
            PC = self.pc,
            STATUS = self.status.bits()
        );

        self.push16(self.pc);
        self.push8(self.status.bits());
        self.status.set(Status::INT_DISABLE, true);

        // Load address of interrupt handler, set PC to execute there
        self.bus.clock(2);
        self.pc = self.bus.read16(IRQ_HANDLER_ADDR);
        event!(Level::TRACE, "IRQ: 0x{:>4X}", self.pc);
    }

    fn execute_instruction(&mut self) {
        use instructions::InstrName::*;

        let opcode = self.bus.read(self.pc);
        self.instruction = instructions::decode_instruction(opcode);

        // 1 cycle we use to execute the instruction
        self.cycles = self.instruction.cycles();

        let num_operands = (self.instruction.size() - 1) as usize;
        for i in 0..num_operands {
            self.operands[i] = self.bus.read(self.pc + (i as u16) + 1);
        }

        let mut operand_str = String::new();
        for i in 0..num_operands {
            operand_str += &format!("{:#02X} ", self.operands[i]);
        }

        event!(
            Level::INFO,
            "0x{:04X}> {:?} {}",
            self.pc,
            &self.instruction,
            operand_str,
        );

        self.log_cpu_state();
        // FIXME: incrementing the pc should go at the end
        self.pc += self.instruction.size();

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
        // TODO: we can likely implement this as i8
        let pc = if is_negative(dst) {
            self.pc.wrapping_sub(dst.wrapping_neg() as u16)
        } else {
            self.pc.wrapping_add(dst as u16)
        };
        let crossed_page = crosses_page(self.pc, pc);
        event!(
            Level::INFO,
            "0x{:>4X}> branch taken -> {:#X} (cross page {})",
            self.pc,
            pc,
            crossed_page
        );

        // Crossing a page adds an extra cycle
        self.cycles += 1 + crossed_page as u8;
        self.pc = pc;
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
    fn update_flags(&mut self, v: u8) {
        // NOTE: anything greater than 127 is negative since it is a 2's complement format
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

        let low_byte = self.operands[0];
        let high_byte = self.operands[1];
        let concat_bytes = |low, high| ((high as u16) << 8) | low as u16;

        match self.instruction.mode() {
            ZeroPage => low_byte as u16,
            ZeroPageX => low_byte.wrapping_add(self.x) as u16,
            ZeroPageY => low_byte.wrapping_add(self.y) as u16,
            Absolute => concat_bytes(low_byte, high_byte),
            AbsoluteX => {
                let addr_without_offset = concat_bytes(low_byte, high_byte);
                let addr = addr_without_offset.wrapping_add(self.x as u16);

                self.cycles += self.takes_extra_cycle(addr_without_offset, addr) as u8;
                addr
            }
            AbsoluteY => {
                let addr_without_offset = concat_bytes(low_byte, high_byte);
                let addr = addr_without_offset.wrapping_add(self.y as u16);

                self.cycles += self.takes_extra_cycle(addr_without_offset, addr) as u8;
                addr
            }
            Indirect => self.bus.read16(concat_bytes(low_byte, high_byte)),
            IndirectX => self.bus.read16(low_byte.wrapping_add(self.x) as u16),
            IndirectY => {
                let addr_without_offset = self.bus.read16(low_byte as u16);
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
        let result = a.wrapping_add(b).wrapping_add(carry as u8);

        let overflow = (a ^ b) & 0x80 == 0 && (b ^ result) & 0x80 != 0;
        let carry = (carry && ((a | b) & 0x80 != 0)) || (a & b & 0x80 != 0);

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
        self.update_flags(self.acc);
    }

    fn and(&mut self) {
        self.acc &= self.get_operand();
        self.update_flags(self.acc);
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
        self.update_flags(result);
    }

    fn cpx(&mut self) {
        let operand = self.get_operand();
        self.status.set(Status::CARRY, self.x >= operand);

        let result = self.x.wrapping_sub(operand);
        self.update_flags(result);
    }

    fn cpy(&mut self) {
        let operand = self.get_operand();
        self.status.set(Status::CARRY, self.y >= operand);

        let result = self.y.wrapping_sub(operand);
        self.update_flags(result);
    }

    fn dec(&mut self) {
        let addr = self.calc_addr();
        let result = self.bus.read(addr).wrapping_sub(1);

        self.bus.write(addr, result);
        self.update_flags(result);
    }

    fn dex(&mut self) {
        self.x = self.x.wrapping_sub(1);
        self.update_flags(self.x);
    }

    fn dey(&mut self) {
        self.y = self.y.wrapping_sub(1);
        self.update_flags(self.y);
    }

    fn eor(&mut self) {
        let operand = self.get_operand();
        self.acc ^= operand;
        self.update_flags(self.acc);
    }

    fn inc(&mut self) {
        let addr = self.calc_addr();
        let result = self.bus.read(addr).wrapping_add(1);
        self.bus.write(addr, result);
        self.update_flags(result);
    }

    fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.update_flags(self.x);
    }

    fn iny(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.update_flags(self.y);
    }

    fn jmp(&mut self) {
        let addr = self.calc_addr();
        event!(Level::INFO, "{:#04X}> JMP -> {:#04X}", self.pc, TO = addr);
        self.pc = addr;
    }

    fn jsr(&mut self) {
        let pc = self.pc - 1;
        self.push16(pc);
        self.pc = ((self.operands[1] as u16) << 8) | (self.operands[0] as u16);
        event!(Level::INFO, "{:#04X}> JSR -> {:#04X}", pc, TO = self.pc);
    }

    fn rts(&mut self) {
        let pc = self.pc;
        self.pc = self.pop16() + 1;
        event!(Level::INFO, "{:#04X}> RTS -> {:#04X}", pc, TO = self.pc);
    }

    fn lda(&mut self) {
        self.acc = self.get_operand();
        self.update_flags(self.acc);
    }

    fn ldx(&mut self) {
        self.x = self.get_operand();
        self.update_flags(self.x);
    }

    fn ldy(&mut self) {
        self.y = self.get_operand();
        self.update_flags(self.y);
    }

    fn lsr(&mut self) {
        let (addr, operand) = self.read_memory();

        self.status.set(Status::CARRY, operand & 0x01 != 0);
        let shift = operand >> 1;

        self.write_memory(addr, shift);
        self.update_flags(shift);
    }

    fn asl(&mut self) {
        let (addr, operand) = self.read_memory();

        self.status.set(Status::CARRY, operand & 0x80 != 0);
        let shift = operand << 1;

        self.write_memory(addr, shift);
        self.update_flags(shift);
    }

    fn nop(&mut self) {
        // Some (illegal) NOPs have an extra cycle due to the address calculation. use calc_addr to
        // account for this. TODO this could probably be made clearer, but the check requires us to
        // inspect the intermediate stages of the address calculation
        let _ = self.calc_addr();
    }

    fn ora(&mut self) {
        self.acc |= self.get_operand();
        self.update_flags(self.acc);
    }

    fn pha(&mut self) {
        self.push8(self.acc);
    }

    fn pla(&mut self) {
        self.acc = self.pop8();
        self.update_flags(self.acc);
    }

    fn php(&mut self) {
        self.push8((self.status | Status::BRK).bits());
    }

    fn plp(&mut self) {
        self.status =
            Status::from_bits(self.pop8() & !Status::BRK.bits() | Status::PUSH_IRQ.bits())
                .expect("All bits are covered in Status");
        event!(
            Level::INFO,
            "{:#>4}> STATUS {:X}",
            self.pc,
            self.status.bits()
        );
    }

    fn rol(&mut self) {
        let (addr, operand) = self.read_memory();

        let carry = self.status.contains(Status::CARRY);
        self.status.set(Status::CARRY, (operand & 0x80) != 0);
        let shift = (operand << 1) | (carry as u8);

        self.write_memory(addr, shift);
        self.update_flags(shift);
    }

    fn ror(&mut self) {
        let (addr, operand) = self.read_memory();

        let carry = self.status.contains(Status::CARRY);
        self.status.set(Status::CARRY, (operand & 0x01) != 0);
        let shift = (operand >> 1) | ((carry as u8) << 7);

        self.write_memory(addr, shift);
        self.update_flags(shift);
    }

    fn rti(&mut self) {
        self.plp();
        self.pc = self.pop16();
    }

    fn sbc(&mut self) {
        let operand = self.get_operand();
        self.acc = self.sub_with_carry_and_overflow(self.acc, operand);
        self.update_flags(self.acc);
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
        event!(
            Level::INFO,
            "0x{:>4X}: STA 0x{:X} -> 0x{:>4X}",
            self.pc,
            ACC = self.acc,
            MEM = addr,
        );
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
        self.update_flags(self.x);
    }

    fn tay(&mut self) {
        self.y = self.acc;
        self.update_flags(self.y);
    }

    fn tsx(&mut self) {
        self.x = self.sp;
        self.update_flags(self.x);
    }

    fn txa(&mut self) {
        self.acc = self.x;
        self.update_flags(self.acc);
    }

    fn txs(&mut self) {
        self.sp = self.x;
    }

    fn tya(&mut self) {
        self.acc = self.y;
        self.update_flags(self.acc);
    }

    // Illegal instructions
    fn alr(&mut self) {
        let addr = self.calc_addr();
        let operand = self.bus.read(addr);

        self.status.set(Status::CARRY, operand & 0x01 != 0);
        let shift = operand >> 1;

        self.acc &= operand;
        self.bus.write(addr, shift);
        self.update_flags(shift);
    }

    // TODO this doesn't seem right
    fn anc(&mut self) {
        let addr = self.calc_addr();
        let operand = self.bus.read(addr);

        self.status.set(Status::CARRY, operand & 0x80 != 0);
        let shift = operand << 1;

        self.acc &= operand;
        self.bus.write(addr, shift);
        self.update_flags(self.acc);
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
        self.update_flags(self.acc);
    }

    fn dcp(&mut self) {
        let addr = self.calc_addr();
        let dec = self.bus.read(addr).wrapping_sub(1);
        self.bus.write(addr, dec);

        let result = self.acc.wrapping_sub(dec);
        self.update_flags(result);
        self.status.set(Status::CARRY, self.acc >= dec);
    }

    fn isc(&mut self) {
        let addr = self.calc_addr();
        let result = self.bus.read(addr).wrapping_add(1);
        self.bus.write(addr, result);
        self.acc = self.sub_with_carry_and_overflow(self.acc, result);
        self.update_flags(self.acc);
    }

    fn las(&mut self) {
        let operand = self.get_operand();

        self.acc = operand;
        self.x = self.sp;
        self.update_flags(self.x);
    }

    fn lax(&mut self) {
        let addr = self.calc_addr();
        let operand = self.bus.read(addr);

        self.acc = operand;
        self.x = operand;
        self.update_flags(self.acc);
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
        self.update_flags(self.acc);
    }

    fn rra(&mut self) {
        let addr = self.calc_addr();
        let operand = self.bus.read(addr);

        let carry = self.status.contains(Status::CARRY);
        self.status.set(Status::CARRY, (operand & 0x01) != 0);

        let shift = (operand >> 1) | ((carry as u8) << 7);
        self.bus.write(addr, shift);

        self.acc = self.add_with_carry_and_overflow(self.acc, shift);
        self.update_flags(self.acc);
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
        self.update_flags(self.x);
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
        self.update_flags(self.acc);
    }

    fn sre(&mut self) {
        let addr = self.calc_addr();
        let mem = self.bus.read(addr);
        self.status.set(Status::CARRY, mem & 0x1 != 0);
        let shift = mem >> 1;
        self.bus.write(addr, shift);

        self.acc ^= shift;
        self.update_flags(self.acc);
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
