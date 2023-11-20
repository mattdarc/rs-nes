use std::cell::UnsafeCell;
use std::collections::{hash_map::RawEntryMut, HashMap};
use std::hash::{BuildHasher, DefaultHasher, Hash, Hasher};
pub use std::time::Duration;

#[derive(Default)]
struct UnusedHasher;

impl BuildHasher for UnusedHasher {
    type Hasher = DefaultHasher;

    fn build_hasher(&self) -> Self::Hasher {
        DefaultHasher::default()
    }

    fn hash_one<T: Hash>(&self, _x: T) -> u64 {
        panic!("This hasher is unused and should not be called. This may have occurred during a rehash")
    }
}

thread_local! {
    static LOCAL_REGISTRY: UnsafeCell<TimeResultRegistry> = UnsafeCell::new(TimeResultRegistry::default());
}

#[cfg(not(feature = "notimers"))]
macro_rules! timed {
    ($name:literal, $contents:block) => {{
        use lazy_static::lazy_static;
        lazy_static! {
            static ref TIMER: timer::TimerName = timer::TimerName::new($name);
        }

        let _scoped_timer = timer::ScopedTimer::new(&TIMER);
        $contents
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

#[derive(Eq)]
pub struct TimerName(&'static str, u64);

impl TimerName {
    pub fn new(name: &'static str) -> Self {
        let mut hasher = DefaultHasher::default();
        name.hash(&mut hasher);
        let hash = hasher.finish();

        TimerName(name, hash)
    }
}

impl PartialEq for TimerName {
    fn eq(&self, other: &Self) -> bool {
        let same = self.0.as_ptr() == other.0.as_ptr();
        assert!(!same || self.1 == other.1, "Found entries with bad hashes");

        same
    }
}

// Registry of time results across the whole program. These are written to disk or printed on
// thread exit, since we do not to do this while anything is running
struct TimeResultRegistry {
    global_start: FastInstant,
    results: HashMap<&'static str, TimeResult, UnusedHasher>,
}

impl Default for TimeResultRegistry {
    fn default() -> Self {
        // Arbitrary number of timers allowed to prevent rehashes and allocations
        const NUM_TIMERS: usize = 64;

        TimeResultRegistry {
            global_start: FastInstant::now(),
            results: HashMap::with_capacity_and_hasher(64, UnusedHasher::default()),
        }
    }
}

// Name: duration mapping for serializing to disk or printing
#[derive(Default)]
struct TimeResult {
    total_duration: Duration,
    samples: u64,
}

pub struct ScopedTimer {
    name: &'static TimerName,
    start: FastInstant,
}

impl ScopedTimer {
    pub fn new(name: &'static TimerName) -> Self {
        ScopedTimer {
            name,
            start: FastInstant::now(),
        }
    }
}

impl Drop for ScopedTimer {
    fn drop(&mut self) {
        TimeResultRegistry::add(self.name, self.start.elapsed())
    }
}

impl TimeResultRegistry {
    fn add(timer: &TimerName, duration: Duration) {
        LOCAL_REGISTRY.with(|r| {
            // SAFETY: This is safe since we're operating on a thread-local and this is the only way
            // for it to be mutated
            let raw_entry = unsafe { (*r.get()).results.raw_entry_mut() };

            let &TimerName(name, hash) = timer;
            let entry = match raw_entry.from_key_hashed_nocheck(hash, name) {
                RawEntryMut::Occupied(entry) => entry.into_mut(),
                RawEntryMut::Vacant(entry) => {
                    entry
                        .insert_hashed_nocheck(hash, name, TimeResult::default())
                        .1
                }
            };
            entry.total_duration += duration;
            entry.samples += 1;
        });
    }
}

impl Drop for TimeResultRegistry {
    fn drop(&mut self) {
        let global_duration = self.global_start.elapsed().as_micros() as f64;
        let global_duration_div = global_duration / 100.;

        println!("\nTiming (total: {} us)", global_duration);

        let mut sorted_results: Vec<_> = self.results.iter().collect();
        sorted_results.sort_by(|(_, a), (_, b)| b.total_duration.cmp(&a.total_duration));

        for (
            name,
            TimeResult {
                total_duration,
                samples,
            },
        ) in &sorted_results
        {
            let dur = total_duration.as_micros() as f64;
            println!(
                "  {:<30}: {:>10} us ({:>5.2}%), avg: {:>10.02} us, samples {:>10}",
                name,
                dur,
                dur / global_duration_div,
                dur / (*samples as f64),
                samples,
            );
        }
    }
}
