use tracing::Level;
use tracing_subscriber::fmt;
use venus::VNES;

fn main() -> Result<(), String> {
    // Configure a custom event formatter
    let format = fmt::format()
        .with_level(true) // include levels in formatted output
        .with_target(false) // don't include targets
        .with_thread_ids(false) // include the thread ID of the current thread
        .with_thread_names(false) // include the name of the current thread
        .without_time()
        .compact(); // use the `Compact` formatting style.

    // Create a `fmt` subscriber that uses our custom event format, and set it
    // as the default.
    tracing_subscriber::fmt()
        .event_format(format)
        .with_max_level(Level::DEBUG)
        .init();

    let mut vnes = VNES::new("donkey-kong.nes").unwrap();
    vnes.reset();
    let res = vnes.play();

    println!("Exiting VNES");
    res
}
