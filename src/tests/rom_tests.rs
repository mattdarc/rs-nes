use lazy_static::lazy_static;
use regex::Regex;
use tracing::{event, Level};
use tracing_subscriber::{fmt, prelude::*, Layer};
use venus::VNES;
use venus::{
    cpu::{instructions::Instruction, CpuInterface, NESSnapshot, SnapshotBuilder},
    ExitStatus,
};

struct NestestParser {
    cpu_states: Vec<NESSnapshot>,
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
            static ref PPU_RE: Regex =
                Regex::new(r"PPU:\s*(?P<scanline>[0-9]+),\s*(?P<cycle>[0-9]+)").unwrap();
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
            let ppu_cycle = PPU_RE
                .captures(line)
                .unwrap()
                .name("cycle")
                .unwrap()
                .as_str()
                .parse::<i16>()
                .expect("PPU cycle not numeric");
            let scanline = PPU_RE
                .captures(line)
                .unwrap()
                .name("scanline")
                .unwrap()
                .as_str()
                .parse::<i16>()
                .expect("Scanline not numeric");

            let mut builder = SnapshotBuilder::new()
                .pc(pc)
                .instruction(opcode)
                .operands(args)
                .total_cycles(cycles)
                .scanline(scanline)
                .ppu_cycle(ppu_cycle);

            for reg in REG_RE.captures_iter(line) {
                let value = u8::from_str_radix(reg.name("value").unwrap().as_str(), 16).unwrap();
                builder = match reg.name("name").unwrap().as_str() {
                    "X" => builder.x(value),
                    "Y" => builder.y(value),
                    "SP" => builder.sp(value),
                    "P" => builder.status(value),
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

fn nestest() {
    const GOLD_FILE: &str = "test/nestest.log.gold";
    let nestest_state = NestestParser::new(GOLD_FILE).expect("Error parsing gold file");

    let mut nes = VNES::new_headless("test/nestest.nes").expect("Could not load nestest ROM");

    const NESTEST_AUTOMATED_START: u16 = 0xC000;
    nes.nestest_reset_override(NESTEST_AUTOMATED_START);

    let num_states = nestest_state.cpu_states.len();

    let mut i = 0;
    nes.add_pre_execute_task(Box::new(move |cpu: &mut dyn CpuInterface| {
        assert_eq!(cpu.read_state(), nestest_state.cpu_states[i]);
        i += 1;
    }));

    for _ in 0..num_states {
        if nes.run_once() != ExitStatus::Continue {
            panic!();
        }
    }
}

fn run_test_rom(s: &str) {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let mut layers = Vec::new();

        // Configure a custom event formatter
        layers.push(
            fmt::layer()
                .with_ansi(false) // No colors
                .with_level(false) // include levels in formatted output
                .with_target(false) // don't include targets
                .with_thread_ids(false) // include the thread ID of the current thread
                .with_thread_names(false) // include the name of the current thread
                .without_time()
                .with_file(false) // No file name in output
                .compact()
                .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                    metadata.target() == "venus::cpu" && metadata.level() <= &Level::INFO
                }))
                .boxed(),
        ); // use the `Compact` formatting style.
        tracing_subscriber::registry().with(layers).init();
    });

    let mut nes = VNES::new_headless(s).expect("Could not load nestest ROM");
    nes.reset();

    let mut test_started = false;
    nes.add_post_execute_task(Box::new(move |cpu: &mut dyn CpuInterface| {
        const TEST_DONE_RESULT_ADDR: u16 = 0x6000;
        const TEST_RUNNING: u8 = 0x80;
        let val = cpu.read_address(TEST_DONE_RESULT_ADDR);
        if val == TEST_RUNNING {
            test_started = true;
        } else if test_started && val != 0x80 {
            cpu.request_stop(val.into());
        }
    }));

    let result = nes.play();
    assert!(result.is_ok(), "{:?}", result);
}

macro_rules! rom_tests {
    ($($name:ident: $rom:literal,)*) => {
    $(
        #[test]
        fn $name() {
            run_test_rom($rom);
        }
    )*
    }
}

rom_tests! {
    nes_instr_test_implied: "nes-test-roms/nes_instr_test/rom_singles/01-implied.nes",
    nes_instr_test_immediate: "nes-test-roms/nes_instr_test/rom_singles/02-immediate.nes",
    nes_instr_test_zero_page: "nes-test-roms/nes_instr_test/rom_singles/03-zero_page.nes",
    nes_instr_test_zp_xy: "nes-test-roms/nes_instr_test/rom_singles/04-zp_xy.nes",
    nes_instr_test_absolute: "nes-test-roms/nes_instr_test/rom_singles/05-absolute.nes",
    nes_instr_test_abs_xy: "nes-test-roms/nes_instr_test/rom_singles/06-abs_xy.nes",
    nes_instr_test_ind_x: "nes-test-roms/nes_instr_test/rom_singles/07-ind_x.nes",
    nes_instr_test_ind_y: "nes-test-roms/nes_instr_test/rom_singles/08-ind_y.nes",
    nes_instr_test_branches: "nes-test-roms/nes_instr_test/rom_singles/09-branches.nes",
    nes_instr_test_stack: "nes-test-roms/nes_instr_test/rom_singles/10-stack.nes",
    nes_instr_test_special: "nes-test-roms/nes_instr_test/rom_singles/11-special.nes",
}
