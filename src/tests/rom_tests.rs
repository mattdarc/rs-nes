use tracing::{event, Level};
use venus::{cartridge::Cartridge, graphics::nop::NOPRenderer, ExitStatus, NesCPU};

// Run the test rom from http://www.qmtpro.com/~nes/misc/
// Compare the output of nestest-log.txt to gold
//
// Expected diff:
// < 0001  FF 00 00  ISB                     A:00 X:FF Y:15 P:27 SP:FF             CYC:26560
// < 0004  00        BRK                     A:B9 X:FF Y:15 P:A4 SP:FF             CYC:26567

pub struct TestVNES {
    cpu: NesCPU,
}

impl TestVNES {
    fn new(rom: &str) -> Self {
        const NESTEST_AUTOMATED_START: u16 = 0xC000;
        let game = Cartridge::load(rom)?;
        let bus = NesBus::new(game, Box::new(NOPRenderer::new()));

        Ok(TestVNES {
            cpu: NesCPU::new(bus, NESTEST_AUTOMATED_START),
        })
    }

    pub fn init(&mut self) {
        self.cpu.init();
    }

    // TODO: merge with non-test implementation. Perhaps this should be part of CPU or some other
    // common code that handles the event loop
    pub fn play(&mut self) -> Result<(), NesError> {
        loop {
            match self.cpu.clock() {
                ExitStatus::Continue => {}
                ExitStatus::ExitSuccess => return Ok(()),
                ExitStatus::ExitError(e) => {
                    event!(Level::ERROR, %e, "Exiting");
                    return Ok(());
                }
                ExitStatus::ExitInterrupt => {
                    event!(Level::INFO, "Exiting from software interrupt");
                    return Ok(());
                }
            }
        }
    }
}

#[test]
fn nestest() {
    use std::fs;

    let mut nes = TestVNES::new("test/nestest.nes").expect("Could not load nestest ROM");
    nes.nestest_init();
    nes.play().expect("Error running game");

    const GOLD_FILE: &str = "test/nestest.log.gold";
    let gold_txt = fs::read_to_string(GOLD_FILE).expect("Error reading gold file");

    const LOG_FILE: &str = "test/nestest.log";
    let log_txt = fs::read_to_string(LOG_FILE).expect("Error reading log file");

    for (line_no, (gold, log)) in gold_txt.lines().zip(log_txt.lines()).enumerate() {
        assert_eq!(gold, log, "Line {}:\n- {}\n+ {}", line_no, gold, log);
    }
}
