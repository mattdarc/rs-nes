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

pub fn get_from(opcode: u8) -> Instruction {
    OPCODES[opcode as usize]
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AddressingMode {
    ZeroPage,    // 1 byte
    ZeroPageX,   // 2 byte
    ZeroPageY,   // 2 byte
    Absolute,    // 3 byte
    AbsoluteX,   // 3 byte
    AbsoluteY,   // 3 byte
    Indirect,    // 2 byte
    IndirectX,   // 2 byte
    IndirectY,   // 2 byte
    Relative,    // 2 byte
    Accumulator, // 1 byte
    Immediate,   // 2 byte
    Invalid,     // Used for invalid opcodes
}

#[derive(Debug, Copy, Clone, PartialEq)]
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

    INV, // INVALID
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Instruction {
    name: InstrName,
    mode: AddressingMode,
    cycles: u8,
}

impl Instruction {
    pub const fn new(name: InstrName, mode: AddressingMode, cycles: u8) -> Instruction {
        Instruction { name, mode, cycles }
    }

    pub const fn invalid() -> Instruction {
        Instruction {
            name: InstrName::INV,
            mode: AddressingMode::Invalid,
	    cycles: 0,
        }
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
}

// Use opcodes as indices into 256 element array of function pointers - mem overhead would be higher, but still low
// - Opcodes are eight-bits long and have the general form AAABBBCC, where AAA and CC define the opcode,
//   and BBB defines the addressing mode
const fn create_opcode_table() -> [Instruction; 256] {
    let mut tbl: [Instruction; 256] = [Instruction::invalid(); 256];
    use crate::instructions::AddressingMode::*;
    use crate::instructions::InstrName::*;

    macro_rules! create_instr {
	($opcode:literal; $name:ident, $mode:ident, $c:literal) => {
	    tbl[$opcode] = Instruction::new($name, $mode, $c);
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
    create_instr!(0x9D; STA, AbsoluteX, 4);
    create_instr!(0x99; STA, AbsoluteY, 4);
    create_instr!(0x81; STA, IndirectX, 6);
    create_instr!(0x91; STA, IndirectY, 5);

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

    create_instr!(0xE8; INX, Invalid, 2);
    create_instr!(0xC8; INY, Invalid, 2);

    create_instr!(0xC6; DEC, ZeroPage, 5);
    create_instr!(0xD6; DEC, ZeroPageX, 6);
    create_instr!(0xCE; DEC, Absolute, 6);
    create_instr!(0xDE; DEC, AbsoluteX, 7);

    create_instr!(0xCA; DEX, Invalid, 2);
    create_instr!(0x88; DEY, Invalid, 2);

    create_instr!(0x0A; ASL, Accumulator, 2);
    create_instr!(0x06; ASL, ZeroPage, 5);
    create_instr!(0x16; ASL, ZeroPageX, 6);
    create_instr!(0x0E; ASL, Absolute, 6);
    create_instr!(0x1E; ASL, AbsoluteX, 7);

    create_instr!(0xA2; LDX, Immediate, 2);
    create_instr!(0xA6; LDX, ZeroPage, 3);
    create_instr!(0xB6; LDX, ZeroPageX, 4);
    create_instr!(0xAE; LDX, Absolute, 4);
    create_instr!(0xBE; LDX, AbsoluteX, 4);

    create_instr!(0xA0; LDY, Immediate, 2);
    create_instr!(0xA4; LDY, ZeroPage, 3);
    create_instr!(0xB4; LDY, ZeroPageX, 4);
    create_instr!(0xAE; LDY, Absolute, 4);
    create_instr!(0xBE; LDY, AbsoluteX, 4);

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

    create_instr!(0x40; RTI, Invalid, 6);
    create_instr!(0x60; RTS, Invalid, 6);

    create_instr!(0xEA; NOP, Invalid, 2);

    create_instr!(0x4C; JMP, Absolute, 3);
    create_instr!(0x6C; JMP, Indirect, 5);
    create_instr!(0x20; JSR, Absolute, 6);

    create_instr!(0x48; PHA, Invalid, 3);
    create_instr!(0x08; PHP, Invalid, 3);
    create_instr!(0x68; PLA, Invalid, 4);
    create_instr!(0x28; PLP, Invalid, 4);

    create_instr!(0x86; STX, ZeroPage, 3);
    create_instr!(0x96; STX, ZeroPageY, 4);
    create_instr!(0x8E; STX, Absolute, 4);

    create_instr!(0x84; STY, ZeroPage, 3);
    create_instr!(0x94; STY, ZeroPageX, 4);
    create_instr!(0x8C; STY, Absolute, 4);

    // Transfers
    create_instr!(0xAA; TAX, Invalid, 2);
    create_instr!(0xA8; TAY, Invalid, 2);
    create_instr!(0xBA; TSX, Invalid, 2);
    create_instr!(0x8A; TXA, Invalid, 2);
    create_instr!(0x9A; TXS, Invalid, 2);
    create_instr!(0x98; TYA, Invalid, 2);

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
    create_instr!(0x18; CLC, Invalid, 2);
    create_instr!(0xD8; CLD, Invalid, 2);
    create_instr!(0x58; CLI, Invalid, 2);
    create_instr!(0xB8; CLV, Invalid, 2);
    create_instr!(0x38; SEC, Invalid, 2);
    create_instr!(0xF8; SED, Invalid, 2);
    create_instr!(0x78; SEI, Invalid, 2);

    create_instr!(0x00; BRK, Invalid, 7);

    create_instr!(0x24; BIT, ZeroPage, 3);
    create_instr!(0x2C; BIT, Absolute, 4);

    tbl
}

const OPCODES: [Instruction; 256] = create_opcode_table();
