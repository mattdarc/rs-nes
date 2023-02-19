use tracing::Level;
use tracing_subscriber::{fmt, prelude::*, Layer};
use venus::VNES;

const DEBUG_COMPONENT: &'static str = "ppu";

fn init_tracing() {
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
                // FIXME: Make this a runtime-decision with an argument parser
                (metadata.target() == format!("venus::{}", DEBUG_COMPONENT)
                    || metadata.target() == "venus::ppu")
                    && metadata.level() <= &Level::INFO
            }))
            .boxed(),
    ); // use the `Compact` formatting style.

    // Create a `fmt` subscriber that uses our custom event format, and set it
    // as the default.
    tracing_subscriber::registry().with(layers).init();
}

fn main() -> Result<(), String> {
    init_tracing();

    // FIXME: Make this a runtime-decision with an argument parser
    let mut vnes = VNES::new("roms/mario-bros.nes").unwrap();
    vnes.reset();
    let res = vnes.play();

    println!("Exiting VNES");
    res
}
