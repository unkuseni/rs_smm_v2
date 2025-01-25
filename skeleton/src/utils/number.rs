use num_traits::{Float, NumCast};
use std::iter::successors;

/// Optimized square root with error checking
pub fn nbsqrt<T: Float>(num: T) -> Result<T, T> {
    if num.is_nan() {
        Err(T::zero())
    } else {
        let abs = num.abs();
        Ok(abs.sqrt().copysign(num))
    }
}

/// Exponential decay with precomputed constants
pub fn decay<T: Float>(value: T, rate: Option<T>) -> T {
    match rate {
        Some(v) => (-v * value).exp(),
        None => (-T::from(0.5).unwrap() * value).exp(),
    }
}

/// Geometric weights using iterative multiplication
pub fn geometric_weights(ratio: f64, n: usize, reverse: bool) -> Vec<f64> {
    assert!((0.0..=1.0).contains(&ratio), "Ratio must be 0-1");

    if ratio == 1.0 {
        let val = 1.0 / n as f64;
        return vec![val; n];
    }

    let sum = (1.0 - ratio.powi(n as i32)) / (1.0 - ratio);
    let mut current = 1.0;
    let mut weights = Vec::with_capacity(n);

    if reverse {
        current = ratio.powi(n as i32 - 1);
        weights.push(current / sum);
        for _ in 1..n {
            current /= ratio;
            weights.push(current / sum);
        }
    } else {
        weights.push(current / sum);
        for _ in 1..n {
            current *= ratio;
            weights.push(current / sum);
        }
    }

    weights
}

/// Optimized linear space using iterator
pub fn linspace<T: Float + NumCast>(start: T, end: T, n: usize) -> Vec<T> {
    assert!(n > 1, "n must be > 1");
    assert!(!start.is_nan() && !end.is_nan(), "NaN values prohibited");

    let n_minus_1 = T::from(n - 1).unwrap();
    let step = (end - start) / n_minus_1;

    let mut result: Vec<T> = (0..n).map(|i| start + T::from(i).unwrap() * step).collect();

    // Ensure exact endpoint
    *result.last_mut().unwrap() = end;
    result
}

/// Optimized geometric space with precomputed inverses
pub fn geomspace<T: Float + NumCast>(start: T, end: T, n: usize) -> Vec<T> {
    assert!(n > 1, "n must be > 1");
    assert!(!start.is_nan() && !end.is_nan(), "NaN values prohibited");
    assert!(!start.is_zero() && !end.is_zero(), "Zero values prohibited");
    assert!(start.signum() == end.signum(), "Sign mismatch");

    if n == 2 {
        return vec![start, end];
    }

    let log_start = start.ln();
    let log_end = end.ln();
    let log_diff = log_end - log_start;
    let n_minus_1 = T::from(n - 1).unwrap();
    let inv_n_minus_1 = T::one() / n_minus_1;

    let mut result = Vec::with_capacity(n);
    result.push(start);

    for i in 1..n - 1 {
        let t = T::from(i).unwrap() * inv_n_minus_1;
        result.push((log_start + log_diff * t).exp());
    }

    result.push(end);
    result
}

/// Fast rounding using scaled integers
pub fn round_step<T: Float>(value: T, step: T) -> T {
    (value / step).round() * step
}

pub trait Round<T> {
    fn round_to(&self, digit: u8) -> T;
    fn clip(&self, min: T, max: T) -> T;
    fn count_decimal_places(&self) -> usize;
}

impl Round<f64> for f64 {
    /// Optimized rounding using bitmask technique
    fn round_to(&self, digits: u8) -> f64 {
        assert!(digits <= 17, "Max 17 digits supported");
        let factor = 10.0f64.powi(digits as i32);
        (self * factor + 0.5).trunc() / factor
    }

    /// Branchless clipping
    fn clip(&self, min: f64, max: f64) -> f64 {
        self.min(max).max(min)
    }

    /// Arithmetic decimal place counting
    fn count_decimal_places(&self) -> usize {
        if self.is_nan() || self.is_infinite() || self.fract() == 0.0 {
            return 0;
        }

        let mut value = self.abs();
        value -= value.trunc();

        successors(Some(value), |v| {
            let next = v * 10.0;
            (next.fract() > 1e-10).then_some(next)
        })
        .take(20)
        .count()
    }
}
