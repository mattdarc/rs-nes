use lazy_static::lazy_static;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::sync::{atomic::AtomicU64, atomic::Ordering, Arc, Mutex};
pub use std::time::Duration;

#[cfg(not(feature = "notimers"))]
macro_rules! timed {
    ($name:literal, $contents:block) => {{
        use std::cell::UnsafeCell;

        thread_local! {
            static TIMER: UnsafeCell<timer::Timer> = UnsafeCell::new(timer::Timer::new($name));
        }

        // Both are safe as
        //   a) This timer is only referenced in this scope, so there can be no references outside
        //      of this scope.
        //   b) No references leave the thread-local scope
        TIMER.with(|t| unsafe { (*t.get()).start() });
        let ret = $contents;
        TIMER.with(|t| unsafe { (*t.get()).stop() });

        ret
    }};
}

#[cfg(feature = "notimers")]
macro_rules! timed {
    ($name:literal, $contents:block) => {{
        $contents
    }};
}
pub(crate) use timed;

pub struct FastInstant(u64);

type TimeResultRef = Arc<TimeResult>;

#[cfg(target_os = "macos")]
extern "system" {
    fn clock_gettime_nsec_np(clk_id: libc::clockid_t) -> u64;
}

impl FastInstant {
    pub fn elapsed(&self) -> Duration {
        let now = FastInstant::now();
        Duration::from_nanos(now.0 - self.0)
    }

    #[cfg(target_os = "macos")]
    pub fn now() -> Self {
        const CLOCK_MONOTONIC_RAW_APPROX: libc::clockid_t = 5;
        let nsec = unsafe { clock_gettime_nsec_np(CLOCK_MONOTONIC_RAW_APPROX) };

        FastInstant(nsec)
    }
}

// Registry of time results across the whole program. These are written to disk or printed on
// thread exit, since we do not want to do this while anything is running
struct TimeResultRegistry {
    global_start: FastInstant,
    results: HashMap<&'static str, Vec<TimeResultRef>>,
}

impl Default for TimeResultRegistry {
    fn default() -> Self {
        // No safety concerns here since the function we're calling simply prints the timers
        use libc::atexit;
        let ret = unsafe { atexit(show_timers_at_exit) };
        assert_eq!(ret, 0);

        TimeResultRegistry {
            global_start: FastInstant::now(),
            results: HashMap::new(),
        }
    }
}

// Name: Duration mapping for serializing to disk or printing
#[derive(Default)]
struct TimeResult {
    total_duration: UnsafeCell<Duration>,
    samples: AtomicU64,
}

pub struct Timer {
    name: &'static str,
    start: FastInstant,
    result: TimeResultRef,
}

impl Timer {
    pub fn new(name: &'static str) -> Self {
        let result = Arc::new(TimeResult::default());

        TimeResultRegistry::add_timer(name, result.clone());

        Timer {
            name,
            result,
            start: FastInstant::now(),
        }
    }

    pub fn start(&mut self) {
        self.start = FastInstant::now();
    }

    pub fn stop(&mut self) {
        let elapsed = self.start.elapsed();

        // This is safe as this value is only ever modified from the one thread, and the release
        // fetch_add means the value will be visible on other threads
        unsafe { *self.result.total_duration.get() += elapsed };
        self.result.samples.fetch_add(1, Ordering::Release);
    }
}

unsafe impl Sync for TimeResult {}

lazy_static! {
    static ref GLOBAL_REGISTRY: Mutex<TimeResultRegistry> =
        Mutex::new(TimeResultRegistry::default());
}

extern "C" fn show_timers_at_exit() {
    GLOBAL_REGISTRY.lock().unwrap().show_timers();
}

impl TimeResultRegistry {
    fn add_timer(name: &'static str, result: TimeResultRef) {
        GLOBAL_REGISTRY
            .lock()
            .unwrap()
            .results
            .entry(name)
            .or_insert_with(|| Vec::new())
            .push(result);
    }

    fn show_timers(&mut self) {
        let global_duration = self.global_start.elapsed().as_micros() as f64;
        let global_duration_div = global_duration / 100.;

        let mut sorted_results: Vec<_> = self
            .results
            .iter()
            .map(|(name, times)| {
                (
                    name,
                    times.iter().fold((Duration::default(), 0), |a, b| {
                        // Acquire load here means any corresponding timer duration on another
                        // thread will be visible if the sample count was incremented
                        let samples = b.samples.load(Ordering::Acquire);
                        let duration = unsafe { *b.total_duration.get() };
                        (a.0 + duration, a.1 + samples)
                    }),
                )
            })
            .collect();
        sorted_results.sort_by(|(_, a), (_, b)| b.0.cmp(&a.0));

        println!("\nTiming (total: {} us)", global_duration);
        for (name, result) in &sorted_results {
            let dur = result.0.as_micros() as f64;
            println!(
                "  {:<30}: {:>10} us ({:>5.2}%), avg: {:>10.02} us, samples {:>10}",
                name,
                dur,
                dur / global_duration_div,
                dur / (result.1 as f64),
                result.1,
            );
        }
        println!();
    }
}
