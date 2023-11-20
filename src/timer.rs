use std::cell::{RefCell, UnsafeCell};
use std::collections::HashMap;
use std::rc::Rc;
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
// thread exit, since we do not to do this while anything is running
struct TimeResultRegistry {
    global_start: FastInstant,
    results: HashMap<&'static str, Vec<Rc<RefCell<TimeResult>>>>,
}

impl Default for TimeResultRegistry {
    fn default() -> Self {
        TimeResultRegistry {
            global_start: FastInstant::now(),
            results: HashMap::new(),
        }
    }
}

// Name: Duration mapping for serializing to disk or printing
#[derive(Default)]
struct TimeResult {
    total_duration: Duration,
    samples: u64,
}

pub struct Timer {
    name: &'static str,
    start: FastInstant,
    result: Rc<RefCell<TimeResult>>,
}

impl Timer {
    pub fn new(name: &'static str) -> Self {
        let result = Rc::new(RefCell::new(TimeResult::default()));

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

        let mut result = self.result.borrow_mut();
        result.total_duration += elapsed;
        result.samples += 1;
    }
}

impl TimeResultRegistry {
    fn add_timer(name: &'static str, result: Rc<RefCell<TimeResult>>) {
        thread_local! {
            static LOCAL_REGISTRY: UnsafeCell<TimeResultRegistry> = (|| {
                UnsafeCell::new(TimeResultRegistry::default())
            })();
        }

        LOCAL_REGISTRY.with(|r| unsafe {
            // SAFETY:
            //
            // Safe as we're operating on a thread-local and this is the only way for it to be
            // mutated or referenced
            (*r.get())
                .results
                .entry(name)
                .or_insert_with(|| Vec::new())
                .push(result);
        });
    }
}

impl Drop for TimeResultRegistry {
    fn drop(&mut self) {
        let global_duration = self.global_start.elapsed().as_micros() as f64;
        let global_duration_div = global_duration / 100.;

        let mut sorted_results: Vec<_> = self
            .results
            .iter()
            .map(|(name, times)| {
                (
                    name,
                    times.iter().fold(TimeResult::default(), |a, b| TimeResult {
                        total_duration: a.total_duration + b.borrow().total_duration,
                        samples: a.samples + b.borrow().samples,
                    }),
                )
            })
            .collect();
        sorted_results.sort_by(|(_, a), (_, b)| b.total_duration.cmp(&a.total_duration));

        println!("\nTiming (total: {} us)", global_duration);
        for (name, result) in &sorted_results {
            let dur = result.total_duration.as_micros() as f64;
            println!(
                "  {:<30}: {:>10} us ({:>5.2}%), avg: {:>10.02} us, samples {:>10}",
                name,
                dur,
                dur / global_duration_div,
                dur / (result.samples as f64),
                result.samples,
            );
        }
        println!();
    }
}
