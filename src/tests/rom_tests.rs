use lazy_static::lazy_static;
use regex::Regex;
use tracing::{event, Level};
use venus::VNES;
use venus::{
    cpu::{instructions::Instruction, CpuState, CpuStateBuilder},
    ExitStatus,
};

// Run the test rom from http://www.qmtpro.com/~nes/misc/
// Compare the output of nestest-log.txt to gold
//
// Expected diff:
// < 0001  FF 00 00  ISB                     A:00 X:FF Y:15 P:27 SP:FF             CYC:26560
// < 0004  00        BRK                     A:B9 X:FF Y:15 P:A4 SP:FF             CYC:26567

struct NestestParser {
    cpu_states: Vec<CpuState>,
}

impl NestestParser {
    pub fn new(filename: &str) -> Result<Self, String> {
        let mut parser = NestestParser {
            cpu_states: Vec::new(),
        };
        parser.parse_file(filename)?;
        Ok(parser)
    }

    fn parse_file(&mut self, filename: &str) -> Result<(), String> {
        use std::fs::File;
        use std::io::{prelude::*, BufReader};

        lazy_static! {
            static ref LOG_LINE_RE: Regex = Regex::new(concat!(
                r"^(?P<pc>[A-F0-9]{4})\s+",
                r"(?P<opcode>[A-F0-9]{2})\s+",
                r"(?P<args>([A-F0-9]{2}\s)*)",
            ))
            .unwrap();
            static ref REG_RE: Regex =
                Regex::new(r"(?P<name>[A-Z][A-Z]?):(?P<value>[A-F0-9]{2})").unwrap();
            static ref CYC_RE: Regex = Regex::new(r"CYC:(?P<cycles>[0-9]+)").unwrap();
        }

        let nestest_log = File::open(filename).or_else(|e| Err(e.to_string()))?;
        let reader = BufReader::new(nestest_log);

        for (i, line) in reader.lines().enumerate() {
            let line = &line.or_else(|e| Err(e.to_string()))?;
            let parsed = match LOG_LINE_RE.captures(line) {
                Some(p) => p,
                None => panic!("Failed to parse line {}:\n\t{}", i, line),
            };
            let pc = u16::from_str_radix(parsed.name("pc").unwrap().as_str(), 16)
                .expect("PC is not numeric");
            let opcode = parsed
                .name("opcode")
                .unwrap()
                .as_str()
                .parse::<Instruction>()
                .expect("Instruction is not hex format");
            let args = parsed
                .name("args")
                .map_or("", |m| m.as_str())
                .split(" ")
                .filter(|s| !s.is_empty())
                .map(|d| u8::from_str_radix(d, 16).expect("args not numeric"))
                .collect::<Vec<_>>();
            let cycles = CYC_RE
                .captures(line)
                .unwrap()
                .name("cycles")
                .unwrap()
                .as_str()
                .parse::<usize>()
                .expect("Cycles not numeric");

            let mut builder = CpuStateBuilder::new()
                .pc(pc)
                .instruction(opcode)
                .operands(args)
                .cycles(cycles);

            for reg in REG_RE.captures_iter(line) {
                let value = u8::from_str_radix(reg.name("value").unwrap().as_str(), 16).unwrap();
                builder = match reg.name("name").unwrap().as_str() {
                    "X" => builder.x(value),
                    "Y" => builder.y(value),
                    "SP" => builder.sp(value),
                    "A" => builder.acc(value),
                    name => {
                        event!(Level::DEBUG, "Unknown register {}: {}", name, value);
                        builder
                    }
                };
            }

            self.cpu_states.push(builder.build());
        }

        Ok(())
    }
}

#[cfg(test)]
fn nestest() {
    const GOLD_FILE: &str = "test/nestest.log.gold";
    let nestest_state = NestestParser::new(GOLD_FILE).expect("Error parsing gold file");

    let mut nes = VNES::new("test/nestest.nes").expect("Could not load nestest ROM");
    const NESTEST_AUTOMATED_START: u16 = 0xC000;
    nes.reset_override(NESTEST_AUTOMATED_START);

    for s in nestest_state.cpu_states.iter() {
        if nes.run_once() != ExitStatus::Continue {
            panic!();
        }

        assert_eq!(nes.state(), s);
    }
}
