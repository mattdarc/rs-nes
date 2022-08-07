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
#![allow(non_camel_case_types)]

pub fn decode_instruction(opcode: u8) -> Instruction {
    OPCODES[opcode as usize]
}

pub fn is_branch(inst: &Instruction) -> bool {
    use InstrName::*;

    match *inst.name() {
        BPL | BMI | BVC | BVS | BCC | BCS | BNE | BEQ => true,
        _ => false,
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AddressingMode {
    ZeroPage,    // 1 byte
    ZeroPageX,   // 2 byte
    ZeroPageY,   // 2 byte
    Absolute,    // 3 byte
    AbsoluteX,   // 3 byte
    AbsoluteY,   // 3 byte
    Indirect,    // 3 byte
    IndirectX,   // 2 byte
    IndirectY,   // 2 byte
    Relative,    // 2 byte
    Accumulator, // 1 byte
    Immediate,   // 2 byte
    Implied,     // 1 byte
}

#[derive(Copy, Clone, PartialEq)]
pub enum InstrName {
    // CC == 0b01
    ORA,
    AND,
    EOR,
    ADC,
    STA,
    LDA,
    CMP,
    SBC,

    // CC == 0b10
    ASL,
    ROL,
    LSR,
    ROR,
    STX,
    LDX,
    DEC,
    INC,

    // CC == 0b00
    BIT,
    JMP,
    STY,
    LDY,
    CPY,
    CPX,

    // Branches
    BPL,
    BMI,
    BVC,
    BVS,
    BCC,
    BCS,
    BNE,
    BEQ,

    // Misc
    BRK,
    JSR,
    RTI,
    RTS,
    NOP,
    TYA,
    TXA,
    TAY,
    TAX,
    TXS,
    TSX,
    PLP,
    PLA,
    PHP,
    PHA,
    INX,
    INY,
    DEX,
    DEY,

    // Flags
    SEI,
    SED,
    SEC,
    CLV,
    CLI,
    CLC,
    CLD,

    ILLEGAL_NOP,
    ILLEGAL_JAM,
    ILLEGAL_SLO,
    ILLEGAL_RLA,
    ILLEGAL_SRE,
    ILLEGAL_RRA,
    ILLEGAL_SAX,
    ILLEGAL_SHA,
    ILLEGAL_LAX,
    ILLEGAL_DCP,
    ILLEGAL_ISC,
    ILLEGAL_ANC,
    ILLEGAL_ALR,
    ILLEGAL_ARR,
    ILLEGAL_ANE,
    ILLEGAL_TAS,
    ILLEGAL_LXA,
    ILLEGAL_LAS,
    ILLEGAL_SBX,
    ILLEGAL_USBC,
    ILLEGAL_SHY,
    ILLEGAL_SHX,
}

impl std::fmt::Debug for InstrName {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        use InstrName::*;
        fmt.write_str(match self {
            ORA => "ORA",
            AND => "AND",
            EOR => "EOR",
            ADC => "ADC",
            STA => "STA",
            LDA => "LDA",
            CMP => "CMP",
            SBC => "SBC",
            ASL => "ASL",
            ROL => "ROL",
            LSR => "LSR",
            ROR => "ROR",
            STX => "STX",
            LDX => "LDX",
            DEC => "DEC",
            INC => "INC",
            BIT => "BIT",
            JMP => "JMP",
            STY => "STY",
            LDY => "LDY",
            CPY => "CPY",
            CPX => "CPX",
            BPL => "BPL",
            BMI => "BMI",
            BVC => "BVC",
            BVS => "BVS",
            BCC => "BCC",
            BCS => "BCS",
            BNE => "BNE",
            BEQ => "BEQ",
            BRK => "BRK",
            JSR => "JSR",
            RTI => "RTI",
            RTS => "RTS",
            NOP => "NOP",
            TYA => "TYA",
            TXA => "TXA",
            TAY => "TAY",
            TAX => "TAX",
            TXS => "TXS",
            TSX => "TSX",
            PLP => "PLP",
            PLA => "PLA",
            PHP => "PHP",
            PHA => "PHA",
            INX => "INX",
            INY => "INY",
            DEX => "DEX",
            DEY => "DEY",
            SEI => "SEI",
            SED => "SED",
            SEC => "SEC",
            CLV => "CLV",
            CLI => "CLI",
            CLC => "CLC",
            CLD => "CLD",
            ILLEGAL_NOP => "NOP",
            ILLEGAL_JAM => "*JAM",
            ILLEGAL_SLO => "SLO",
            ILLEGAL_RLA => "RLA",
            ILLEGAL_SRE => "SRE",
            ILLEGAL_RRA => "RRA",
            ILLEGAL_SAX => "SAX",
            ILLEGAL_SHA => "*SHA",
            ILLEGAL_LAX => "LAX",
            ILLEGAL_DCP => "DCP",
            ILLEGAL_ISC => "ISB",
            ILLEGAL_ANC => "*ANC",
            ILLEGAL_ALR => "*ALR",
            ILLEGAL_ARR => "*ARR",
            ILLEGAL_ANE => "*ANE",
            ILLEGAL_TAS => "*TAS",
            ILLEGAL_LXA => "*LXA",
            ILLEGAL_LAS => "*LAS",
            ILLEGAL_SBX => "*SBX",
            ILLEGAL_USBC => "SBC",
            ILLEGAL_SHY => "*SHY",
            ILLEGAL_SHX => "*SHX",
        })
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct Instruction {
    opcode: u8,
    name: InstrName,
    mode: AddressingMode,
    cycles: u8,
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&format!(
            "{:X}: {:?} {:?}",
            self.opcode, self.name, self.mode
        ))
    }
}

impl std::fmt::Debug for Instruction {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "{}", self)
    }
}

impl Instruction {
    pub const fn new(opcode: u8, name: InstrName, mode: AddressingMode, cycles: u8) -> Instruction {
        Instruction {
            opcode,
            name,
            mode,
            cycles,
        }
    }

    pub const fn nop() -> Instruction {
        Instruction {
            opcode: 0,
            name: InstrName::NOP,
            mode: AddressingMode::Implied,
            cycles: 0,
        }
    }

    pub const fn opcode(&self) -> u8 {
        self.opcode
    }

    pub const fn mode(&self) -> &AddressingMode {
        &self.mode
    }

    pub const fn name(&self) -> &InstrName {
        &self.name
    }

    pub const fn cycles(&self) -> u8 {
        self.cycles
    }

    pub const fn size(&self) -> u16 {
        use AddressingMode::*;

        match self.mode() {
            ZeroPage | ZeroPageX | ZeroPageY => 2,
            IndirectY | IndirectX | Relative | Immediate => 2,
            Indirect | Absolute | AbsoluteX | AbsoluteY => 3,
            Accumulator | Implied => 1,
        }
    }
}

// Use opcodes as indices into 256 element array of function pointers - mem overhead would be higher, but still low
// - Opcodes are eight-bits long and have the general form AAABBBCC, where AAA and CC define the opcode,
//   and BBB defines the addressing mode
const fn create_opcode_table() -> [Instruction; 256] {
    let mut tbl: [Instruction; 256] = [Instruction::nop(); 256];
    use AddressingMode::*;
    use InstrName::*;

    macro_rules! create_instr {
        ($opcode:literal; $name:ident, $mode:ident, $c:literal) => {
            tbl[$opcode] = Instruction::new($opcode, $name, $mode, $c);
        };
    }

    create_instr!(0x69; ADC, Immediate, 2);
    create_instr!(0x65; ADC, ZeroPage, 3);
    create_instr!(0x75; ADC, ZeroPageX, 4);
    create_instr!(0x6D; ADC, Absolute, 4);
    create_instr!(0x7D; ADC, AbsoluteX, 4);
    create_instr!(0x79; ADC, AbsoluteY, 4);
    create_instr!(0x61; ADC, IndirectX, 6);
    create_instr!(0x71; ADC, IndirectY, 5);

    create_instr!(0x29; AND, Immediate, 2);
    create_instr!(0x25; AND, ZeroPage, 3);
    create_instr!(0x35; AND, ZeroPageX, 4);
    create_instr!(0x2D; AND, Absolute, 4);
    create_instr!(0x3D; AND, AbsoluteX, 4);
    create_instr!(0x39; AND, AbsoluteY, 4);
    create_instr!(0x21; AND, IndirectX, 6);
    create_instr!(0x31; AND, IndirectY, 5);

    create_instr!(0xC9; CMP, Immediate, 2);
    create_instr!(0xC5; CMP, ZeroPage, 3);
    create_instr!(0xD5; CMP, ZeroPageX, 4);
    create_instr!(0xCD; CMP, Absolute, 4);
    create_instr!(0xDD; CMP, AbsoluteX, 4);
    create_instr!(0xD9; CMP, AbsoluteY, 4);
    create_instr!(0xC1; CMP, IndirectX, 6);
    create_instr!(0xD1; CMP, IndirectY, 5);

    create_instr!(0x49; EOR, Immediate, 2);
    create_instr!(0x45; EOR, ZeroPage, 3);
    create_instr!(0x55; EOR, ZeroPageX, 4);
    create_instr!(0x4D; EOR, Absolute, 4);
    create_instr!(0x5D; EOR, AbsoluteX, 4);
    create_instr!(0x59; EOR, AbsoluteY, 4);
    create_instr!(0x41; EOR, IndirectX, 6);
    create_instr!(0x51; EOR, IndirectY, 5);

    create_instr!(0xA9; LDA, Immediate, 2);
    create_instr!(0xA5; LDA, ZeroPage, 3);
    create_instr!(0xB5; LDA, ZeroPageX, 4);
    create_instr!(0xAD; LDA, Absolute, 4);
    create_instr!(0xBD; LDA, AbsoluteX, 4);
    create_instr!(0xB9; LDA, AbsoluteY, 4);
    create_instr!(0xA1; LDA, IndirectX, 6);
    create_instr!(0xB1; LDA, IndirectY, 5);

    create_instr!(0x09; ORA, Immediate, 2);
    create_instr!(0x05; ORA, ZeroPage, 3);
    create_instr!(0x15; ORA, ZeroPageX, 4);
    create_instr!(0x0D; ORA, Absolute, 4);
    create_instr!(0x1D; ORA, AbsoluteX, 4);
    create_instr!(0x19; ORA, AbsoluteY, 4);
    create_instr!(0x01; ORA, IndirectX, 6);
    create_instr!(0x11; ORA, IndirectY, 5);

    create_instr!(0xE9; SBC, Immediate, 2);
    create_instr!(0xE5; SBC, ZeroPage, 3);
    create_instr!(0xF5; SBC, ZeroPageX, 4);
    create_instr!(0xED; SBC, Absolute, 4);
    create_instr!(0xFD; SBC, AbsoluteX, 4);
    create_instr!(0xF9; SBC, AbsoluteY, 4);
    create_instr!(0xE1; SBC, IndirectX, 6);
    create_instr!(0xF1; SBC, IndirectY, 5);

    create_instr!(0x85; STA, ZeroPage, 3);
    create_instr!(0x95; STA, ZeroPageX, 4);
    create_instr!(0x8D; STA, Absolute, 4);
    create_instr!(0x9D; STA, AbsoluteX, 5);
    create_instr!(0x99; STA, AbsoluteY, 5);
    create_instr!(0x81; STA, IndirectX, 6);
    create_instr!(0x91; STA, IndirectY, 6);

    create_instr!(0xE0; CPX, Immediate, 2);
    create_instr!(0xE4; CPX, ZeroPage, 3);
    create_instr!(0xEC; CPX, Absolute, 4);

    create_instr!(0xC0; CPY, Immediate, 2);
    create_instr!(0xC4; CPY, ZeroPage, 3);
    create_instr!(0xCC; CPY, Absolute, 4);

    create_instr!(0xE6; INC, ZeroPage, 5);
    create_instr!(0xF6; INC, ZeroPageX, 6);
    create_instr!(0xEE; INC, Absolute, 6);
    create_instr!(0xFE; INC, AbsoluteX, 7);

    create_instr!(0xE8; INX, Implied, 2);
    create_instr!(0xC8; INY, Implied, 2);

    create_instr!(0xC6; DEC, ZeroPage, 5);
    create_instr!(0xD6; DEC, ZeroPageX, 6);
    create_instr!(0xCE; DEC, Absolute, 6);
    create_instr!(0xDE; DEC, AbsoluteX, 7);

    create_instr!(0xCA; DEX, Implied, 2);
    create_instr!(0x88; DEY, Implied, 2);

    create_instr!(0x0A; ASL, Accumulator, 2);
    create_instr!(0x06; ASL, ZeroPage, 5);
    create_instr!(0x16; ASL, ZeroPageX, 6);
    create_instr!(0x0E; ASL, Absolute, 6);
    create_instr!(0x1E; ASL, AbsoluteX, 7);

    create_instr!(0xA2; LDX, Immediate, 2);
    create_instr!(0xA6; LDX, ZeroPage, 3);
    create_instr!(0xB6; LDX, ZeroPageY, 4);
    create_instr!(0xAE; LDX, Absolute, 4);
    create_instr!(0xBE; LDX, AbsoluteY, 4);

    create_instr!(0xA0; LDY, Immediate, 2);
    create_instr!(0xA4; LDY, ZeroPage, 3);
    create_instr!(0xB4; LDY, ZeroPageX, 4);
    create_instr!(0xAC; LDY, Absolute, 4);
    create_instr!(0xBC; LDY, AbsoluteX, 4);

    create_instr!(0x4A; LSR, Accumulator, 2);
    create_instr!(0x46; LSR, ZeroPage, 5);
    create_instr!(0x56; LSR, ZeroPageX, 6);
    create_instr!(0x4E; LSR, Absolute, 6);
    create_instr!(0x5E; LSR, AbsoluteX, 7);

    create_instr!(0x2A; ROL, Accumulator, 2);
    create_instr!(0x26; ROL, ZeroPage, 5);
    create_instr!(0x36; ROL, ZeroPageX, 6);
    create_instr!(0x2E; ROL, Absolute, 6);
    create_instr!(0x3E; ROL, AbsoluteX, 7);

    create_instr!(0x6A; ROR, Accumulator, 2);
    create_instr!(0x66; ROR, ZeroPage, 5);
    create_instr!(0x76; ROR, ZeroPageX, 6);
    create_instr!(0x6E; ROR, Absolute, 6);
    create_instr!(0x7E; ROR, AbsoluteX, 7);

    create_instr!(0x40; RTI, Implied, 6);
    create_instr!(0x60; RTS, Implied, 6);

    create_instr!(0xEA; NOP, Implied, 2);

    create_instr!(0x4C; JMP, Absolute, 3);
    create_instr!(0x6C; JMP, Indirect, 5);
    create_instr!(0x20; JSR, Absolute, 6);

    create_instr!(0x48; PHA, Implied, 3);
    create_instr!(0x08; PHP, Implied, 3);
    create_instr!(0x68; PLA, Implied, 4);
    create_instr!(0x28; PLP, Implied, 4);

    create_instr!(0x86; STX, ZeroPage, 3);
    create_instr!(0x96; STX, ZeroPageY, 4);
    create_instr!(0x8E; STX, Absolute, 4);

    create_instr!(0x84; STY, ZeroPage, 3);
    create_instr!(0x94; STY, ZeroPageX, 4);
    create_instr!(0x8C; STY, Absolute, 4);

    // Transfers
    create_instr!(0xAA; TAX, Implied, 2);
    create_instr!(0xA8; TAY, Implied, 2);
    create_instr!(0xBA; TSX, Implied, 2);
    create_instr!(0x8A; TXA, Implied, 2);
    create_instr!(0x9A; TXS, Implied, 2);
    create_instr!(0x98; TYA, Implied, 2);

    // Branches
    create_instr!(0x90; BCC, Relative, 2);
    create_instr!(0xB0; BCS, Relative, 2);
    create_instr!(0xF0; BEQ, Relative, 2);
    create_instr!(0x30; BMI, Relative, 2);
    create_instr!(0xD0; BNE, Relative, 2);
    create_instr!(0x10; BPL, Relative, 2);
    create_instr!(0x50; BVC, Relative, 2);
    create_instr!(0x70; BVS, Relative, 2);

    // Flags
    create_instr!(0x18; CLC, Implied, 2);
    create_instr!(0xD8; CLD, Implied, 2);
    create_instr!(0x58; CLI, Implied, 2);
    create_instr!(0xB8; CLV, Implied, 2);
    create_instr!(0x38; SEC, Implied, 2);
    create_instr!(0xF8; SED, Implied, 2);
    create_instr!(0x78; SEI, Implied, 2);

    create_instr!(0x00; BRK, Implied, 7);

    create_instr!(0x24; BIT, ZeroPage, 3);
    create_instr!(0x2C; BIT, Absolute, 4);

    // Illegal instructions
    create_instr!(0x1A; ILLEGAL_NOP, Implied, 2);
    create_instr!(0x3A; ILLEGAL_NOP, Implied, 2);
    create_instr!(0x5A; ILLEGAL_NOP, Implied, 2);
    create_instr!(0x7A; ILLEGAL_NOP, Implied, 2);
    create_instr!(0xDA; ILLEGAL_NOP, Implied, 2);
    create_instr!(0xFA; ILLEGAL_NOP, Implied, 2);
    create_instr!(0x80; ILLEGAL_NOP, Immediate, 2);
    create_instr!(0x82; ILLEGAL_NOP, Immediate, 2);
    create_instr!(0x89; ILLEGAL_NOP, Immediate, 2);
    create_instr!(0xC2; ILLEGAL_NOP, Immediate, 2);
    create_instr!(0xE2; ILLEGAL_NOP, Immediate, 2);
    create_instr!(0x04; ILLEGAL_NOP, ZeroPage, 3);
    create_instr!(0x44; ILLEGAL_NOP, ZeroPage, 3);
    create_instr!(0x64; ILLEGAL_NOP, ZeroPage, 3);
    create_instr!(0x14; ILLEGAL_NOP, ZeroPageX, 4);
    create_instr!(0x34; ILLEGAL_NOP, ZeroPageX, 4);
    create_instr!(0x54; ILLEGAL_NOP, ZeroPageX, 4);
    create_instr!(0x74; ILLEGAL_NOP, ZeroPageX, 4);
    create_instr!(0xD4; ILLEGAL_NOP, ZeroPageX, 4);
    create_instr!(0xF4; ILLEGAL_NOP, ZeroPageX, 4);
    create_instr!(0x0C; ILLEGAL_NOP, Absolute, 4);
    create_instr!(0x1C; ILLEGAL_NOP, AbsoluteX, 4);
    create_instr!(0x3C; ILLEGAL_NOP, AbsoluteX, 4);
    create_instr!(0x5C; ILLEGAL_NOP, AbsoluteX, 4);
    create_instr!(0x7C; ILLEGAL_NOP, AbsoluteX, 4);
    create_instr!(0xDC; ILLEGAL_NOP, AbsoluteX, 4);
    create_instr!(0xFC; ILLEGAL_NOP, AbsoluteX, 4);

    create_instr!(0x4B; ILLEGAL_ALR, Immediate, 2);
    create_instr!(0x0B; ILLEGAL_ANC, Immediate, 2);
    create_instr!(0x2B; ILLEGAL_ANC, Immediate, 2);
    create_instr!(0x8B; ILLEGAL_ANE, Immediate, 2);
    create_instr!(0x6B; ILLEGAL_ARR, Immediate, 2);

    create_instr!(0xC7; ILLEGAL_DCP, ZeroPage, 5);
    create_instr!(0xD7; ILLEGAL_DCP, ZeroPageX, 6);
    create_instr!(0xCF; ILLEGAL_DCP, Absolute, 6);
    create_instr!(0xDF; ILLEGAL_DCP, AbsoluteX, 7);
    create_instr!(0xDB; ILLEGAL_DCP, AbsoluteY, 7);
    create_instr!(0xC3; ILLEGAL_DCP, IndirectX, 8);
    create_instr!(0xD3; ILLEGAL_DCP, IndirectY, 8);

    create_instr!(0xE7; ILLEGAL_ISC, ZeroPage, 5);
    create_instr!(0xF7; ILLEGAL_ISC, ZeroPageX, 6);
    create_instr!(0xEF; ILLEGAL_ISC, Absolute, 6);
    create_instr!(0xFF; ILLEGAL_ISC, AbsoluteX, 7);
    create_instr!(0xFB; ILLEGAL_ISC, AbsoluteY, 7);
    create_instr!(0xE3; ILLEGAL_ISC, IndirectX, 8);

    // Discrepancy here between nestest and the instruction table. Table says 4 cycles, nestest
    // says 8
    create_instr!(0xF3; ILLEGAL_ISC, IndirectY, 8);

    create_instr!(0xBB; ILLEGAL_LAS, AbsoluteY, 4);

    create_instr!(0xA7; ILLEGAL_LAX, ZeroPage, 3);
    create_instr!(0xB7; ILLEGAL_LAX, ZeroPageY, 4);
    create_instr!(0xAF; ILLEGAL_LAX, Absolute, 4);
    create_instr!(0xBF; ILLEGAL_LAX, AbsoluteY, 4);
    create_instr!(0xA3; ILLEGAL_LAX, IndirectX, 6);
    create_instr!(0xB3; ILLEGAL_LAX, IndirectY, 5);

    create_instr!(0xAB; ILLEGAL_LXA, Immediate, 2);

    create_instr!(0x27; ILLEGAL_RLA, ZeroPage, 5);
    create_instr!(0x37; ILLEGAL_RLA, ZeroPageX, 6);
    create_instr!(0x2F; ILLEGAL_RLA, Absolute, 6);
    create_instr!(0x3F; ILLEGAL_RLA, AbsoluteX, 7);
    create_instr!(0x3B; ILLEGAL_RLA, AbsoluteY, 7);
    create_instr!(0x23; ILLEGAL_RLA, IndirectX, 8);
    create_instr!(0x33; ILLEGAL_RLA, IndirectY, 8);

    create_instr!(0x67; ILLEGAL_RRA, ZeroPage, 5);
    create_instr!(0x77; ILLEGAL_RRA, ZeroPageX, 6);
    create_instr!(0x6F; ILLEGAL_RRA, Absolute, 6);
    create_instr!(0x7F; ILLEGAL_RRA, AbsoluteX, 7);
    create_instr!(0x7B; ILLEGAL_RRA, AbsoluteY, 7);
    create_instr!(0x63; ILLEGAL_RRA, IndirectX, 8);
    create_instr!(0x73; ILLEGAL_RRA, IndirectY, 8);

    create_instr!(0x87; ILLEGAL_SAX, ZeroPage, 3);
    create_instr!(0x97; ILLEGAL_SAX, ZeroPageY, 4);
    create_instr!(0x8F; ILLEGAL_SAX, Absolute, 4);
    create_instr!(0x83; ILLEGAL_SAX, IndirectX, 6);

    create_instr!(0xCB; ILLEGAL_SBX, Immediate, 2);

    create_instr!(0x9F; ILLEGAL_SHA, AbsoluteY, 5);
    create_instr!(0x93; ILLEGAL_SHA, IndirectY, 6);

    create_instr!(0x9E; ILLEGAL_SHX, AbsoluteY, 5);

    create_instr!(0x9C; ILLEGAL_SHY, AbsoluteX, 5);

    create_instr!(0x07; ILLEGAL_SLO, ZeroPage, 5);
    create_instr!(0x17; ILLEGAL_SLO, ZeroPageX, 6);
    create_instr!(0x0F; ILLEGAL_SLO, Absolute, 6);
    create_instr!(0x1F; ILLEGAL_SLO, AbsoluteX, 7);
    create_instr!(0x1B; ILLEGAL_SLO, AbsoluteY, 7);
    create_instr!(0x03; ILLEGAL_SLO, IndirectX, 8);
    create_instr!(0x13; ILLEGAL_SLO, IndirectY, 8);

    create_instr!(0x47; ILLEGAL_SRE, ZeroPage, 5);
    create_instr!(0x57; ILLEGAL_SRE, ZeroPageX, 6);
    create_instr!(0x4F; ILLEGAL_SRE, Absolute, 6);
    create_instr!(0x5F; ILLEGAL_SRE, AbsoluteX, 7);
    create_instr!(0x5B; ILLEGAL_SRE, AbsoluteY, 7);
    create_instr!(0x43; ILLEGAL_SRE, IndirectX, 8);
    create_instr!(0x53; ILLEGAL_SRE, IndirectY, 8);

    create_instr!(0x9B; ILLEGAL_TAS, AbsoluteY, 5);

    create_instr!(0xEB; ILLEGAL_USBC, Immediate, 2);

    create_instr!(0x02; ILLEGAL_JAM, Implied, 1);
    create_instr!(0x12; ILLEGAL_JAM, Implied, 1);
    create_instr!(0x22; ILLEGAL_JAM, Implied, 1);
    create_instr!(0x32; ILLEGAL_JAM, Implied, 1);
    create_instr!(0x42; ILLEGAL_JAM, Implied, 1);
    create_instr!(0x52; ILLEGAL_JAM, Implied, 1);
    create_instr!(0x62; ILLEGAL_JAM, Implied, 1);
    create_instr!(0x72; ILLEGAL_JAM, Implied, 1);
    create_instr!(0x92; ILLEGAL_JAM, Implied, 1);
    create_instr!(0xB2; ILLEGAL_JAM, Implied, 1);
    create_instr!(0xD2; ILLEGAL_JAM, Implied, 1);
    create_instr!(0xF2; ILLEGAL_JAM, Implied, 1);

    tbl
}

const OPCODES: [Instruction; 256] = create_opcode_table();
