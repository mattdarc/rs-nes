use tracing::Level;
use tracing_subscriber::{fmt, prelude::*, Layer};
use venus::VNES;

const DEBUG_COMPONENT: &'static str = "ppu";

fn init_tracing() {
    let mut layers = Vec::new();

    // Configure a custom event formatter
    layers.push(
        fmt::layer()
            .with_level(true) // include levels in formatted output
            .with_target(false) // don't include targets
            .with_thread_ids(false) // include the thread ID of the current thread
            .with_thread_names(false) // include the name of the current thread
            .without_time()
            .with_file(true)
            .compact()
            .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                metadata.target() == format!("venus::{}", DEBUG_COMPONENT)
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

    let mut vnes = VNES::new("donkey-kong.nes").unwrap();
    vnes.reset();
    let res = vnes.play();

    println!("Exiting VNES");
    res
}
