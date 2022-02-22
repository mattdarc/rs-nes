use venus::VNES;

// Run the test rom from http://www.qmtpro.com/~nes/misc/
// Compare the output of nestest-log.txt to gold
//
// Expected diff:
// < 0001  FF 00 00  ISB                     A:00 X:FF Y:15 P:27 SP:FF             CYC:26560
// < 0004  00        BRK                     A:B9 X:FF Y:15 P:A4 SP:FF             CYC:26567

#[test]
fn nestest() -> std::io::Result<()> {
    let mut nes = VNES::new("nestest/nestest.nes")?;
    nes.nestest_init();
    nes.play().expect("Error running game");
    Ok(())
}
