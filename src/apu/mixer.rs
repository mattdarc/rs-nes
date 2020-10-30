// Bilinear transform c2d
//      2 z - 1
// s -> - -----
//      T z + 1
//
// ===
// 90 Hz first-order high-pass filter
//
//   s
// ------ ==> ?
// s + 90
//
// ===
// 440 Hz first-order high-pass filter
//
//   s
// ------ ==> ?
// s + 440
//
// ===
// 14 kHz first-order low-pass filter
//
//   14k
// ------ ==> ?
// s + 14k
//
// 44.1 kHz sample rate

use std::cell::RefCell;

const PI: f64 = 3.14159;
const FS: f64 = 44_100.0; // sample rate

macro_rules! b0_d {
    ($b0_c:expr, $b1_c:expr, $a0_c:expr, $a1_c:expr) => {
        (($b0_c * K) + $b1_c) / (($a0_c * K) + $a1_c)
    };
}

macro_rules! b1_d {
    ($b0_c:expr, $b1_c:expr, $a0_c:expr, $a1_c:expr) => {
        -b0_d!($b0_c, $b1_c, $a0_c, $a1_c)
    };
}

macro_rules! a1_d {
    ($b0_c:expr, $b1_c:expr, $a0_c:expr, $a1_c:expr) => {
        ((-$a0_c * K) + $a1_c) / (($a0_c * K) + $a1_c)
    };
}

macro_rules! c2d {
    ([$b0:literal, $b1:literal], [$a0:literal, $a1:literal], $fs:expr) => {{
        const K: f64 = 2.0 * $fs;
        FilterOrd1 {
            b_0: b0_d!(
                $b0 as f64,
                $b1 as f64 * 2.0 * PI,
                $a0 as f64,
                $a1 as f64 * 2.0 * PI
            ),
            b_1: b1_d!(
                $b0 as f64,
                $b1 as f64 * 2.0 * PI,
                $a0 as f64,
                $a1 as f64 * 2.0 * PI
            ),
            a_1: a1_d!(
                $b0 as f64,
                $b1 as f64 * 2.0 * PI,
                $a0 as f64,
                $a1 as f64 * 2.0 * PI
            ),
            x_1: 0.0,
            y_1: 0.0,
        }
    }};
}

#[derive(Clone, Default)]
struct FilterOrd1 {
    a_1: f64,
    b_0: f64,
    b_1: f64,
    x_1: f64,
    y_1: f64,
}

impl FilterOrd1 {
    fn result(&mut self, x_0: f64) -> f64 {
        let y_0 = self.b_0 * x_0 + self.b_1 * self.x_1 - self.a_1 * self.y_1;
        self.y_1 = y_0;
        self.x_1 = x_0;
        y_0
    }
}

#[derive(Clone, Default)]
pub struct Mixer {
    high_pass_90: RefCell<FilterOrd1>,
    high_pass_440: RefCell<FilterOrd1>,
    low_pass_14k: RefCell<FilterOrd1>,
}

impl Mixer {
    pub fn new() -> Mixer {
        Mixer {
            high_pass_90: RefCell::new(c2d!([1, 0], [1, 90], FS)),
            high_pass_440: RefCell::new(c2d!([1, 0], [1, 440], FS)),
            low_pass_14k: RefCell::new(c2d!([0, 14_000.0], [1, 14_000.0], FS)),
        }
    }

    pub fn filter(&self, x_0: f64) -> f64 {
        self.low_pass_14k.borrow_mut().result(
            self.high_pass_440
                .borrow_mut()
                .result(self.high_pass_90.borrow_mut().result(x_0)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-4;

    macro_rules! assert_approx {
        ($act:expr, $exp:expr) => {
            assert!(
                ($act - $exp).abs() < EPSILON,
                "{} != {} with epsilon {}",
                $act,
                $exp,
                EPSILON
            );
        };
    }

    #[test]
    fn filters() {
        let mixer = Mixer::new();
        let f90 = mixer.high_pass_90.borrow();
        let f440 = mixer.high_pass_440.borrow();
        let f14k = mixer.low_pass_14k.borrow();

        assert_approx!(f90.b_0, 0.9936);
        assert_approx!(f90.b_1, -0.9936);
        assert_approx!(f90.a_1, -0.9873);

        assert_approx!(f440.b_0, 0.9696);
        assert_approx!(f440.b_1, -0.9696);
        assert_approx!(f440.a_1, -0.9392);

        assert_approx!(f14k.b_0, 0.4993);
        assert_approx!(f14k.b_1, -0.4993);
        assert_approx!(f14k.a_1, -0.00136);
    }
}
