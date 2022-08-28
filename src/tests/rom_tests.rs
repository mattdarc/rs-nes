use tracing::Level;
use tracing_subscriber::fmt;
use venus::VNES;

// Run the test rom from http://www.qmtpro.com/~nes/misc/
// Compare the output of nestest-log.txt to gold
//
// Expected diff:
// < 0001  FF 00 00  ISB                     A:00 X:FF Y:15 P:27 SP:FF             CYC:26560
// < 0004  00        BRK                     A:B9 X:FF Y:15 P:A4 SP:FF             CYC:26567

#[test]
fn nestest() {
    use std::fs;

    const GOLD_FILE: &str = "test/nestest.log.gold";
    let nestest_state = NestestParser::read(GOLD_FILE).expect("Error reading gold file");

    let mut nes = VNES::new("test/nestest.nes").expect("Could not load nestest ROM");
    const NESTEST_AUTOMATED_START: u16 = 0xC000;
    nes.reset_override(NESTEST_AUTOMATED_START);
    nes.play().expect("Error running nestest.nes");

    const LOG_FILE: &str = "test/nestest.log";
    let log_txt = fs::read_to_string(LOG_FILE).expect("Error reading log file");

    for (line_no, (gold, log)) in gold_txt.lines().zip(log_txt.lines()).enumerate() {
        assert_eq!(gold, log, "Line {}:\n- {}\n+ {}", line_no, gold, log);
    }
}
