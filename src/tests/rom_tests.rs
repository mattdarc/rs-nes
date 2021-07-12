use venus::VNES;

#[test]
fn nestest() -> std::io::Result<()> {
    let mut nes = VNES::new("nestest.nes")?;
    nes.enable_logging(true);
    nes.play().expect("Error running game");

    Ok(())
}
