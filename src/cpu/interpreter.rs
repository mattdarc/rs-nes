use super::*;
use timer;

pub struct Interpreter<T: Bus> {
    pub bus: T,
    instruction: Instruction,
    operands: Vec<u8>,
    extra_cycles: usize,
}

impl<T: Bus> Interpreter<T> {
    pub fn new(bus: T) -> Self {
        Interpreter {
            bus,
            instruction: Instruction::default(),
            operands: Vec::with_capacity(2),
            extra_cycles: 0,
        }
    }

    pub fn interpret(&mut self, state: &mut CpuState) -> usize {
        timer::timed!("interpreter::fetch", { self.fetch_instruction(state) });
        timer::timed!("interpreter::execute", { self.execute_instruction(state) })
    }

    pub fn clock_bus(&mut self, ticks: usize) {
        self.bus.clock(ticks)
    }

    pub fn instruction(&self) -> &Instruction {
        &self.instruction
    }

    pub fn operands(&self) -> &[u8] {
        &self.operands
    }

    fn fetch_instruction(&mut self, state: &mut CpuState) {
        let pc = state.pc;
        let opcode = self.bus.read(pc);
        self.instruction = instructions::decode_instruction(opcode);

        let num_operands = (self.instruction.size() - 1) as usize;
        self.operands.resize(num_operands, 0);
        for i in 0..num_operands {
            self.operands[i] = self.bus.read(pc + (i as u16) + 1)
        }
    }

    fn execute_instruction(&mut self, state: &mut CpuState) -> usize {
        use super::instructions::InstrName::*;

        self.extra_cycles = 0;

        let next_pc = match self.instruction.name() {
            // BRANCHES
            BPL => self.bpl(state),
            BMI => self.bmi(state),
            BVC => self.bvc(state),
            BVS => self.bvs(state),
            BCC => self.bcc(state),
            BCS => self.bcs(state),
            BNE => self.bne(state),
            BEQ => self.beq(state),
            ADC => self.adc(state),
            AND => self.and(state),
            SBC => self.sbc(state),
            ORA => self.ora(state),
            LDY => self.ldy(state),
            LDX => self.ldx(state),
            LDA => self.lda(state),
            EOR => self.eor(state),
            CPY => self.cpy(state),
            CPX => self.cpx(state),
            CMP => self.cmp(state),
            BIT => self.bit(state),

            ASL => self.asl(state),
            LSR => self.lsr(state),
            JSR => self.jsr(state),
            JMP => self.jmp(state),
            STY => self.sty(state),
            STX => self.stx(state),
            STA => self.sta(state),
            ROL => self.rol(state),
            ROR => self.ror(state),
            INC => self.inc(state),
            DEC => self.dec(state),

            CLV => self.clv(state),
            CLI => self.cli(state),
            CLC => self.clc(state),
            CLD => self.cld(state),
            DEX => self.dex(state),
            DEY => self.dey(state),
            INY => self.iny(state),
            INX => self.inx(state),
            TAY => self.tay(state),
            TAX => self.tax(state),
            TYA => self.tya(state),
            TXA => self.txa(state),
            TXS => self.txs(state),
            TSX => self.tsx(state),
            SEI => self.sei(state),
            SED => self.sed(state),
            SEC => self.sec(state),
            RTS => self.rts(state),
            RTI => self.rti(state),
            PLP => self.plp(state),
            PLA => self.pla(state),
            PHP => self.php(state),
            PHA => self.pha(state),
            BRK => self.brk(state),

            ILLEGAL_JAM => self.hlt(state),
            ILLEGAL_SLO => self.slo(state),
            ILLEGAL_RLA => self.rla(state),
            ILLEGAL_SRE => self.sre(state),
            ILLEGAL_RRA => self.rra(state),
            ILLEGAL_SAX => self.sax(state),
            ILLEGAL_SHA => self.sha(state),
            ILLEGAL_LAX => self.lax(state),
            ILLEGAL_DCP => self.dcp(state),
            ILLEGAL_ISC => self.isc(state),
            ILLEGAL_ANC => self.anc(state),
            ILLEGAL_ALR => self.alr(state),
            ILLEGAL_ARR => self.arr(state),
            ILLEGAL_ANE => self.ane(state),
            ILLEGAL_TAS => self.tas(state),
            ILLEGAL_LXA => self.lxa(state),
            ILLEGAL_LAS => self.las(state),
            ILLEGAL_SBX => self.sbx(state),
            ILLEGAL_USBC => self.usbc(state),
            ILLEGAL_SHY => self.shy(state),
            ILLEGAL_SHX => self.shx(state),

            ILLEGAL_NOP | NOP => self.nop(state),
        };

        trace_instruction(state, &self.instruction, &self.operands);

        state.instructions_executed += 1;
        state.pc = next_pc.unwrap_or(state.pc.wrapping_add(self.instruction.size()));

        self.extra_cycles + self.instruction.cycles()
    }

    fn hlt(&self, _state: &mut CpuState) -> ! {
        panic!("HLT");
    }

    fn takes_extra_cycle(&mut self, start_addr: u16, end_addr: u16) -> bool {
        use super::instructions::InstrName;

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

    fn do_branch(&mut self, state: &mut CpuState, dst: u8) -> u16 {
        // FIXME: we can likely implement this as i8
        let pc_before = state.pc.wrapping_add(self.instruction.size());
        let next_pc = if is_negative(dst) {
            pc_before.wrapping_sub(dst.wrapping_neg() as u16)
        } else {
            pc_before.wrapping_add(dst as u16)
        };
        let crossed_page = crosses_page(pc_before, next_pc);

        // Crossing a page adds an extra cycle
        self.extra_cycles += 1 + crossed_page as usize;
        next_pc
    }

    fn calc_addr(&mut self, state: &CpuState) -> u16 {
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
            ZeroPageX => addr_lo.wrapping_add(state.x) as u16,
            ZeroPageY => addr_lo.wrapping_add(state.y) as u16,
            Absolute => addr,
            AbsoluteX => {
                let addr_x = addr.wrapping_add(state.x as u16);
                self.extra_cycles += self.takes_extra_cycle(addr, addr_x) as usize;
                addr_x
            }
            AbsoluteY => {
                let addr_y = addr.wrapping_add(state.y as u16);
                self.extra_cycles += self.takes_extra_cycle(addr, addr_y) as usize;
                addr_y
            }
            Indirect => self.bus.read16(addr),
            IndirectX => self.bus.read16(addr_lo.wrapping_add(state.x) as u16),
            IndirectY => {
                let addr_without_offset = self.bus.read16(addr_lo as u16);
                let addr = addr_without_offset.wrapping_add(state.y as u16);

                self.extra_cycles += self.takes_extra_cycle(addr_without_offset, addr) as usize;
                addr
            }
            _ => u16::MAX,
        }
    }

    fn get_operand(&mut self, state: &mut CpuState) -> u8 {
        use instructions::AddressingMode::*;
        match &self.instruction.mode() {
            Accumulator => state.acc,
            Immediate | Relative => self.operands[0],
            _ => {
                let addr = self.calc_addr(state);
                self.bus.read(addr)
            }
        }
    }

    fn write_memory(&mut self, state: &mut CpuState, addr: TargetAddress, val: u8) {
        match addr {
            TargetAddress::Memory(addr) => self.bus.write(addr, val),
            TargetAddress::Accumulator => state.acc = val,
            TargetAddress::None => panic!("Writing to invalid target address"),
        }
    }

    fn read_memory(&mut self, state: &mut CpuState) -> (TargetAddress, u8) {
        use instructions::AddressingMode::*;
        match &self.instruction.mode() {
            Accumulator => (TargetAddress::Accumulator, state.acc),
            Immediate | Relative => (TargetAddress::None, self.operands[0]),
            _ => {
                let addr = self.calc_addr(state);
                (TargetAddress::Memory(addr), self.bus.read(addr))
            }
        }
    }

    fn add_with_carry_and_overflow(&mut self, state: &mut CpuState, op: u8) -> u8 {
        let carry = state.status.contains(Status::CARRY);
        let (result, carry1) = state.acc.overflowing_add(op);
        let (result, carry2) = result.overflowing_add(carry as u8);

        let overflow = (state.acc ^ op) & 0x80 == 0 && (op ^ result) & 0x80 != 0;
        let carry = carry1 || carry2;

        state.status.set(Status::OVERFLOW, overflow);
        state.status.set(Status::CARRY, carry);
        result
    }

    fn sub_with_carry_and_overflow(&mut self, state: &mut CpuState, op: u8) -> u8 {
        let carry = state.status.contains(Status::CARRY);
        let result = state.acc.wrapping_sub(op).wrapping_sub(!carry as u8);

        // result is positive if acc is negative and operand is positive
        //              --OR--
        // result is negative if acc is positive operand is negative
        let overflow = ((result ^ op) & 0x80) == 0 && ((op ^ state.acc) & 0x80) != 0;

        // Carry (not borrow) happens if a >= b where a - b
        let carry = state.acc > op || (state.acc == op && carry);

        state.status.set(Status::CARRY, carry);
        state.status.set(Status::OVERFLOW, overflow);

        result
    }

    // BRANCHES:
    fn bpl(&mut self, state: &mut CpuState) -> Option<u16> {
        let dst = self.get_operand(state);
        if !state.status.contains(Status::NEGATIVE) {
            return Some(self.do_branch(state, dst));
        }

        None
    }

    fn bmi(&mut self, state: &mut CpuState) -> Option<u16> {
        let dst = self.get_operand(state);
        if state.status.contains(Status::NEGATIVE) {
            return Some(self.do_branch(state, dst));
        }

        None
    }

    fn bvc(&mut self, state: &mut CpuState) -> Option<u16> {
        let dst = self.get_operand(state);
        if !state.status.contains(Status::OVERFLOW) {
            return Some(self.do_branch(state, dst));
        }

        None
    }

    fn bvs(&mut self, state: &mut CpuState) -> Option<u16> {
        let dst = self.get_operand(state);
        if state.status.contains(Status::OVERFLOW) {
            return Some(self.do_branch(state, dst));
        }

        None
    }

    fn bcc(&mut self, state: &mut CpuState) -> Option<u16> {
        let dst = self.get_operand(state);
        if !state.status.contains(Status::CARRY) {
            return Some(self.do_branch(state, dst));
        }

        None
    }

    fn bcs(&mut self, state: &mut CpuState) -> Option<u16> {
        let dst = self.get_operand(state);
        if state.status.contains(Status::CARRY) {
            return Some(self.do_branch(state, dst));
        }

        None
    }

    fn bne(&mut self, state: &mut CpuState) -> Option<u16> {
        let dst = self.get_operand(state);
        if !state.status.contains(Status::ZERO) {
            return Some(self.do_branch(state, dst));
        }

        None
    }

    fn beq(&mut self, state: &mut CpuState) -> Option<u16> {
        let dst = self.get_operand(state);
        if state.status.contains(Status::ZERO) {
            return Some(self.do_branch(state, dst));
        }

        None
    }

    fn adc(&mut self, state: &mut CpuState) -> Option<u16> {
        let operand = self.get_operand(state);
        state.acc = self.add_with_carry_and_overflow(state, operand);
        state.update_nz(state.acc);

        None
    }

    fn and(&mut self, state: &mut CpuState) -> Option<u16> {
        state.acc &= self.get_operand(state);
        state.update_nz(state.acc);

        None
    }

    fn bit(&mut self, state: &mut CpuState) -> Option<u16> {
        let operand = self.get_operand(state);
        state.status.set(Status::OVERFLOW, is_bit_set(operand, 6));
        state.status.set(Status::NEGATIVE, is_negative(operand));
        state.status.set(Status::ZERO, (state.acc & operand) == 0);

        None
    }

    fn brk(&mut self, state: &mut CpuState) -> Option<u16> {
        self.push16(state, state.pc.wrapping_add(2));
        self.push8(
            state,
            state.status.bits() | Status::BRK.bits() | Status::PUSH_IRQ.bits(),
        );
        state.status.set(Status::INT_DISABLE, true);

        Some(self.bus.read16(IRQ_VECTOR_START))
    }

    fn clc(&mut self, state: &mut CpuState) -> Option<u16> {
        state.status.set(Status::CARRY, false);
        None
    }

    fn cld(&mut self, state: &mut CpuState) -> Option<u16> {
        state.status.set(Status::DECIMAL, false);
        None
    }

    fn cli(&mut self, state: &mut CpuState) -> Option<u16> {
        state.status.set(Status::INT_DISABLE, false);
        None
    }

    fn clv(&mut self, state: &mut CpuState) -> Option<u16> {
        state.status.set(Status::OVERFLOW, false);
        None
    }

    fn cmp(&mut self, state: &mut CpuState) -> Option<u16> {
        let operand = self.get_operand(state);
        state.status.set(Status::CARRY, state.acc >= operand);

        let result = state.acc.wrapping_sub(operand);
        state.update_nz(result);
        None
    }

    fn cpx(&mut self, state: &mut CpuState) -> Option<u16> {
        let operand = self.get_operand(state);
        state.status.set(Status::CARRY, state.x >= operand);

        let result = state.x.wrapping_sub(operand);
        state.update_nz(result);
        None
    }

    fn cpy(&mut self, state: &mut CpuState) -> Option<u16> {
        let operand = self.get_operand(state);
        state.status.set(Status::CARRY, state.y >= operand);

        let result = state.y.wrapping_sub(operand);
        state.update_nz(result);
        None
    }

    fn dec(&mut self, state: &mut CpuState) -> Option<u16> {
        let addr = self.calc_addr(state);
        let result = self.bus.read(addr).wrapping_sub(1);

        self.bus.write(addr, result);
        state.update_nz(result);
        None
    }

    fn dex(&mut self, state: &mut CpuState) -> Option<u16> {
        state.x = state.x.wrapping_sub(1);
        state.update_nz(state.x);
        None
    }

    fn dey(&mut self, state: &mut CpuState) -> Option<u16> {
        state.y = state.y.wrapping_sub(1);
        state.update_nz(state.y);
        None
    }

    fn eor(&mut self, state: &mut CpuState) -> Option<u16> {
        let operand = self.get_operand(state);
        state.acc ^= operand;
        state.update_nz(state.acc);
        None
    }

    fn inc(&mut self, state: &mut CpuState) -> Option<u16> {
        let addr = self.calc_addr(state);
        let result = self.bus.read(addr).wrapping_add(1);
        self.bus.write(addr, result);
        state.update_nz(result);
        None
    }

    fn inx(&mut self, state: &mut CpuState) -> Option<u16> {
        state.x = state.x.wrapping_add(1);
        state.update_nz(state.x);
        None
    }

    fn iny(&mut self, state: &mut CpuState) -> Option<u16> {
        state.y = state.y.wrapping_add(1);
        state.update_nz(state.y);
        None
    }

    fn jmp(&mut self, state: &mut CpuState) -> Option<u16> {
        Some(self.calc_addr(state))
    }

    fn jsr(&mut self, state: &mut CpuState) -> Option<u16> {
        let pc = state.pc.wrapping_add(2);
        self.push16(state, pc);
        let next_pc = ((self.operands[1] as u16) << 8) | (self.operands[0] as u16);

        Some(next_pc)
    }

    fn rts(&mut self, state: &mut CpuState) -> Option<u16> {
        Some(self.pop16(state) + 1)
    }

    fn lda(&mut self, state: &mut CpuState) -> Option<u16> {
        state.acc = self.get_operand(state);
        state.update_nz(state.acc);

        None
    }

    fn ldx(&mut self, state: &mut CpuState) -> Option<u16> {
        state.x = self.get_operand(state);
        state.update_nz(state.x);

        None
    }

    fn ldy(&mut self, state: &mut CpuState) -> Option<u16> {
        state.y = self.get_operand(state);
        state.update_nz(state.y);

        None
    }

    fn lsr(&mut self, state: &mut CpuState) -> Option<u16> {
        let (addr, operand) = self.read_memory(state);

        state.status.set(Status::CARRY, operand & 0x01 != 0);
        let shift = operand >> 1;

        self.write_memory(state, addr, shift);
        state.update_nz(shift);

        None
    }

    fn asl(&mut self, state: &mut CpuState) -> Option<u16> {
        let (addr, operand) = self.read_memory(state);

        state.status.set(Status::CARRY, operand & 0x80 != 0);
        let shift = operand << 1;

        self.write_memory(state, addr, shift);
        state.update_nz(shift);

        None
    }

    fn nop(&mut self, state: &mut CpuState) -> Option<u16> {
        // Some (illegal) NOPs have an extra cycle due to the address calculation. use calc_addr to
        // account for this.
        let _ = self.calc_addr(state);
        None
    }

    fn ora(&mut self, state: &mut CpuState) -> Option<u16> {
        state.acc |= self.get_operand(state);
        state.update_nz(state.acc);

        None
    }

    fn pha(&mut self, state: &mut CpuState) -> Option<u16> {
        self.push8(state, state.acc);
        None
    }

    fn pla(&mut self, state: &mut CpuState) -> Option<u16> {
        state.acc = self.pop8(state);
        state.update_nz(state.acc);

        None
    }

    fn php(&mut self, state: &mut CpuState) -> Option<u16> {
        self.push8(state, (state.status | Status::BRK).bits());

        None
    }

    fn plp(&mut self, state: &mut CpuState) -> Option<u16> {
        state.status =
            Status::from_bits(self.pop8(state) & !Status::BRK.bits() | Status::PUSH_IRQ.bits())
                .expect("All bits are covered in Status");

        None
    }

    fn rol(&mut self, state: &mut CpuState) -> Option<u16> {
        let (addr, operand) = self.read_memory(state);

        let carry = state.status.contains(Status::CARRY);
        state.status.set(Status::CARRY, (operand & 0x80) != 0);
        let shift = (operand << 1) | (carry as u8);

        self.write_memory(state, addr, shift);
        state.update_nz(shift);

        None
    }

    fn ror(&mut self, state: &mut CpuState) -> Option<u16> {
        let (addr, operand) = self.read_memory(state);

        let carry = state.status.contains(Status::CARRY);
        state.status.set(Status::CARRY, (operand & 0x01) != 0);
        let shift = (operand >> 1) | ((carry as u8) << 7);

        self.write_memory(state, addr, shift);
        state.update_nz(shift);

        None
    }

    fn rti(&mut self, state: &mut CpuState) -> Option<u16> {
        state.status = Status::from_bits(self.pop8(state) & !Status::BRK.bits())
            .expect("All bits are covered in Status");
        state.status.set(Status::PUSH_IRQ, true);

        Some(self.pop16(state))
    }

    fn sbc(&mut self, state: &mut CpuState) -> Option<u16> {
        let operand = self.get_operand(state);
        state.acc = self.sub_with_carry_and_overflow(state, operand);
        state.update_nz(state.acc);

        None
    }

    fn sec(&mut self, state: &mut CpuState) -> Option<u16> {
        state.status.set(Status::CARRY, true);
        None
    }

    fn sed(&mut self, state: &mut CpuState) -> Option<u16> {
        state.status.set(Status::DECIMAL, true);
        None
    }

    fn sei(&mut self, state: &mut CpuState) -> Option<u16> {
        state.status.set(Status::INT_DISABLE, true);
        None
    }

    fn sta(&mut self, state: &mut CpuState) -> Option<u16> {
        let addr = self.calc_addr(state);
        self.bus.write(addr, state.acc);
        None
    }

    fn stx(&mut self, state: &mut CpuState) -> Option<u16> {
        let addr = self.calc_addr(state);
        self.bus.write(addr, state.x);
        None
    }

    fn sty(&mut self, state: &mut CpuState) -> Option<u16> {
        let addr = self.calc_addr(state);
        self.bus.write(addr, state.y);
        None
    }

    fn tax(&mut self, state: &mut CpuState) -> Option<u16> {
        state.x = state.acc;
        state.update_nz(state.x);
        None
    }

    fn tay(&mut self, state: &mut CpuState) -> Option<u16> {
        state.y = state.acc;
        state.update_nz(state.y);
        None
    }

    fn tsx(&mut self, state: &mut CpuState) -> Option<u16> {
        state.x = state.sp;
        state.update_nz(state.x);
        None
    }

    fn txa(&mut self, state: &mut CpuState) -> Option<u16> {
        state.acc = state.x;
        state.update_nz(state.acc);
        None
    }

    fn txs(&mut self, state: &mut CpuState) -> Option<u16> {
        state.sp = state.x;
        None
    }

    fn tya(&mut self, state: &mut CpuState) -> Option<u16> {
        state.acc = state.y;
        state.update_nz(state.acc);
        None
    }

    // Illegal instructions
    fn alr(&mut self, state: &mut CpuState) -> Option<u16> {
        let operand = self.get_operand(state);

        state.acc &= operand;
        state.status.set(Status::CARRY, (state.acc & 0x01) != 0);
        state.acc >>= 1;
        state.update_nz(state.acc);

        None
    }

    fn anc(&mut self, state: &mut CpuState) -> Option<u16> {
        let operand = self.get_operand(state);

        state.acc &= operand;
        state.status.set(Status::CARRY, (state.acc & 0x80) != 0);
        state.update_nz(state.acc);

        None
    }

    fn ane(&mut self, state: &mut CpuState) -> Option<u16> {
        let operand = self.get_operand(state);

        state.acc &= state.x & operand;
        state.update_nz(state.acc);

        None
    }

    fn arr(&mut self, state: &mut CpuState) -> Option<u16> {
        let operand = self.get_operand(state);
        let carry = state.status.contains(Status::CARRY);

        state.acc &= operand;
        state.acc = (state.acc >> 1) | ((carry as u8) << 7);

        let ovfl = xor((state.acc & 0x40) != 0, (state.acc & 0x20) != 0);
        state.status.set(Status::OVERFLOW, ovfl);
        state.status.set(Status::CARRY, (state.acc & 0x40) != 0);

        state.update_nz(state.acc);

        None
    }

    fn dcp(&mut self, state: &mut CpuState) -> Option<u16> {
        let addr = self.calc_addr(state);
        let dec = self.bus.read(addr).wrapping_sub(1);
        self.bus.write(addr, dec);

        let result = state.acc.wrapping_sub(dec);
        state.update_nz(result);
        state.status.set(Status::CARRY, state.acc >= dec);

        None
    }

    fn isc(&mut self, state: &mut CpuState) -> Option<u16> {
        let addr = self.calc_addr(state);
        let result = self.bus.read(addr).wrapping_add(1);
        self.bus.write(addr, result);
        state.acc = self.sub_with_carry_and_overflow(state, result);
        state.update_nz(state.acc);

        None
    }

    fn las(&mut self, state: &mut CpuState) -> Option<u16> {
        let operand = self.get_operand(state);

        state.acc = operand;
        state.x = state.sp;
        state.update_nz(state.x);
        None
    }

    fn lax(&mut self, state: &mut CpuState) -> Option<u16> {
        let addr = self.calc_addr(state);
        let operand = self.bus.read(addr);

        state.acc = operand;
        state.x = operand;
        state.update_nz(state.acc);
        None
    }

    fn lxa(&mut self, state: &mut CpuState) -> Option<u16> {
        let operand = self.get_operand(state);

        state.acc = operand;
        state.x = state.acc;

        state.update_nz(state.acc);
        None
    }

    fn rla(&mut self, state: &mut CpuState) -> Option<u16> {
        let addr = self.calc_addr(state);
        let operand = self.bus.read(addr);

        let carry = state.status.contains(Status::CARRY);
        state.status.set(Status::CARRY, (operand & 0x80) != 0);
        let shift = (operand << 1) | (carry as u8);

        self.bus.write(addr, shift);
        state.acc &= shift;
        state.update_nz(state.acc);

        None
    }

    fn rra(&mut self, state: &mut CpuState) -> Option<u16> {
        let addr = self.calc_addr(state);
        let operand = self.bus.read(addr);

        let carry = state.status.contains(Status::CARRY);
        state.status.set(Status::CARRY, (operand & 0x01) != 0);

        let shift = (operand >> 1) | ((carry as u8) << 7);
        self.bus.write(addr, shift);

        state.acc = self.add_with_carry_and_overflow(state, shift);
        state.update_nz(state.acc);

        None
    }

    fn sax(&mut self, state: &mut CpuState) -> Option<u16> {
        let ax = state.acc & state.x;
        let addr = self.calc_addr(state);
        self.bus.write(addr, ax);

        None
    }

    fn sbx(&mut self, state: &mut CpuState) -> Option<u16> {
        let operand = self.get_operand(state);
        let ax = state.acc & state.x;
        state.x = ax.wrapping_sub(operand);

        state.status.set(Status::CARRY, ax >= operand);
        state.update_nz(state.x);

        None
    }

    fn sha(&mut self, state: &mut CpuState) -> Option<u16> {
        let ax = state.acc & state.x;
        let addr = self.calc_addr(state);
        let high = ((addr >> 8) + 1) as u8;
        self.bus.write(addr, ax & high);

        None
    }

    // Re: SHX/SHY
    //
    // If we cross over into a new page, then the calculated addr_high + 1 gets used, but for these
    // opcodes addr_high + 1 is corrupted (prolly due to the result being put on the same bus as
    // x/y), and the target address for e.g. SYA becomes ((y & (addr_high + 1)) << 8) | addr_low
    // instead of the normal ((addr_high + 1) << 8) | addr_low. If we don't wrap to a new page,
    // then the corrupted value doesn't get used for the address, so nothing special happens.
    fn shx(&mut self, state: &mut CpuState) -> Option<u16> {
        let mut addr = self.calc_addr(state);
        let hi = ((addr >> 8) as u8).wrapping_add(1);
        if crosses_page(addr, addr.wrapping_sub(state.x.into())) {
            addr = (hi as u16) << 8 | (addr & 0xff);
        }

        self.bus.write(addr, state.x & hi);
        None
    }

    fn shy(&mut self, state: &mut CpuState) -> Option<u16> {
        let mut addr = self.calc_addr(state);
        let hi = ((addr >> 8) as u8).wrapping_add(1);
        if crosses_page(addr, addr.wrapping_sub(state.y.into())) {
            addr = (hi as u16) << 8 | (addr & 0xff);
        }

        self.bus.write(addr, state.y & hi);
        None
    }

    fn slo(&mut self, state: &mut CpuState) -> Option<u16> {
        let addr = self.calc_addr(state);
        let mem = self.bus.read(addr);
        state.status.set(Status::CARRY, mem & 0x80 != 0);
        let shift = mem << 1;
        self.bus.write(addr, shift);

        state.acc |= shift;
        state.update_nz(state.acc);

        None
    }

    fn sre(&mut self, state: &mut CpuState) -> Option<u16> {
        let addr = self.calc_addr(state);
        let mem = self.bus.read(addr);
        state.status.set(Status::CARRY, mem & 0x1 != 0);
        let shift = mem >> 1;
        self.bus.write(addr, shift);

        state.acc ^= shift;
        state.update_nz(state.acc);

        None
    }

    fn tas(&mut self, state: &mut CpuState) -> Option<u16> {
        let ax = state.acc & state.x;
        self.push8(state, ax);
        let addr = self.calc_addr(state);
        let high = ((addr + 1) >> 8) as u8;
        self.bus.write(addr, ax & high);

        None
    }

    fn usbc(&mut self, state: &mut CpuState) -> Option<u16> {
        self.sbc(state)
    }

    pub fn reset(&mut self, state: &mut CpuState) {
        let pc = self.bus.read16(RESET_VECTOR_START);
        event!(Level::DEBUG, "reset PC {:#x} -> {:#x}", state.pc, pc);

        state.pc = pc;
        state.status = Status::default();
        state.sp = 0xFD;
    }

    pub fn handle_nmi(&mut self, state: &mut CpuState) -> Option<usize> {
        let nmi = self.bus.pop_nmi();
        if nmi.is_none() {
            return None;
        }

        self.push16(state, state.pc);
        self.push8(state, state.status.bits());
        state.status.set(Status::INT_DISABLE, true);

        // Load address of interrupt handler, set PC to execute there
        state.pc = self.bus.read16(NMI_VECTOR_START);
        event!(Level::TRACE, "IRQ: {:#04X}", state.pc);

        const NMI_CYCLES: usize = 2;
        Some(NMI_CYCLES)
    }

    // FIXME: At some point, these should not use the Bus. But I'm not sure how to get the
    // dispatching right at the moment so we don't need to sprinkle the address map everywhere
    fn push16(&mut self, state: &mut CpuState, v: u16) {
        self.push8(state, (v >> 8) as u8);
        self.push8(state, (0xFF & v) as u8);
    }

    fn push8(&mut self, state: &mut CpuState, v: u8) {
        self.poke(state, v);
        state.sp = state.sp.wrapping_sub(1);
        if state.sp == 0xFF {
            event!(Level::DEBUG, "WARNING: Stack overflow!");
        }
    }

    fn pop16(&mut self, state: &mut CpuState) -> u16 {
        let low = self.pop8(state) as u16;
        ((self.pop8(state) as u16) << 8) | low
    }

    fn pop8(&mut self, state: &mut CpuState) -> u8 {
        if state.sp == 0xFF {
            event!(Level::DEBUG, "WARNING: Tried to pop empty stack!");
        }

        state.sp = state.sp.wrapping_add(1);
        self.peek(state)
    }

    fn peek(&mut self, state: &mut CpuState) -> u8 {
        let ptr = (state.sp as u16).wrapping_add(STACK_BEGIN);
        self.bus.read(ptr)
    }

    fn poke(&mut self, state: &mut CpuState, val: u8) {
        let ptr = (state.sp as u16).wrapping_add(STACK_BEGIN);
        self.bus.write(ptr, val);
    }
}
