use num_traits::{Float, NumCast};

/// Computes the square root of a number.
///
/// This function is similar to the standard library's `f64::sqrt` function, but
/// returns an error if the input is NaN.
///
/// # Errors
///
/// Returns an error if the input is NaN.
///
/// # Examples
///
///
///
pub fn nbsqrt<T: Float>(num: T) -> Result<T, T> {
    if num.is_nan() {
        return Err(T::zero());
    }
    Ok(num.signum() * num.abs().sqrt())
}

/// Applies exponential decay to a value.
///
/// If `decay` is `Some(x)`, the decay is calculated as `e^(-x * value)`.
/// If `decay` is `None`, the decay is calculated as `e^(-0.5 * value)`.
pub fn decay<T: Float>(value: T, rate: Option<T>) -> T {
    match rate {
        Some(v) => (-v * value).exp(),
        None => {
            let half = T::from(0.5).unwrap();
            (-half * value).exp()
        }
    }
}

/// Creates a vector of length `n` with geometric weights.
///
/// The weights are calculated by repeatedly multiplying the `ratio` parameter by itself,
/// and then normalizing the results to add up to 1.0.
///
/// If `reverse == true`, the weights are calculated in reverse order.
///
/// # Example
///
///
pub fn geometric_weights(ratio: f64, n: usize, reverse: bool) -> Vec<f64> {
    assert!(
        ratio >= 0.0 && ratio <= 1.0,
        "Ratio must be between 0 and 1"
    );
    let weights: Vec<f64> = if reverse {
        (0..n).map(|i| ratio.powi((n - 1 - i) as i32)).collect()
    } else {
        (0..n).map(|i| ratio.powi(i as i32)).collect()
    };

    let sum: f64 = weights.iter().sum();
    weights.into_iter().map(|w| w / sum).collect()
}

/// Generates a vector of `n` evenly spaced values over a range from
/// `start` to `end` (inclusive).
///
/// # Panics
///
/// Panics if `n` is less than or equal to 1, or if `start` or `end` is NaN.
///
/// # Examples
///
///
pub fn linspace<T: Float + NumCast>(start: T, end: T, n: usize) -> Vec<T> {
    assert!(n > 1, "n must be greater than 1");
    assert!(
        !start.is_nan() && !end.is_nan(),
        "Input values must not be NaN"
    );

    let mut result = Vec::with_capacity(n);

    // Handle special case for n = 2
    if n == 2 {
        return vec![start, end];
    }

    // Pre-calculate constants
    let n_minus_1 = T::from(n - 1).unwrap();

    // Use linear interpolation for better numerical stability
    for i in 0..n {
        let t = T::from(i).unwrap() / n_minus_1;
        result.push(start + (end - start) * t);
    }

    // Ensure exact endpoints
    if let Some(last) = result.last_mut() {
        *last = end;
    }

    result
}

/// Generates a vector of `n` evenly spaced values over a geometric range
/// from `start` to `end` (inclusive).
///
/// # Panics
///
/// Panics if `n` is less than or equal to 1, or if `start` or `end` is NaN.
///
/// # Examples
///
///
pub fn geomspace<T: Float + NumCast>(start: T, end: T, n: usize) -> Vec<T> {
    assert!(n > 1, "n must be greater than 1");
    assert!(
        !start.is_nan() && !end.is_nan(),
        "Input values must not be NaN"
    );
    assert!(
        !start.is_zero() && !end.is_zero(),
        "Start and end must be non-zero"
    );

    let mut result = Vec::with_capacity(n);

    // Handle special case for n = 2
    if n == 2 {
        return vec![start, end];
    }

    // Handle special case when start and end have different signs
    assert!(
        start.signum() == end.signum(),
        "Start and end must have the same sign"
    );

    let log_start = start.ln();
    let log_end = end.ln();
    let n_minus_1 = T::from(n - 1).unwrap();

    // Use linear interpolation in log space for better numerical stability
    for i in 0..n {
        let t = T::from(i).unwrap() / n_minus_1;
        let log_value = log_start + (log_end - log_start) * t;
        result.push(log_value.exp());
    }

    // Ensure exact endpoints
    result[0] = start;
    result[n - 1] = end;

    result
}

/// Rounds `value` to the nearest multiple of `step`.
///
/// # Example
///
///
pub fn round_step<T: Float>(value: T, step: T) -> T {
    (value / step).round() * step
}

pub trait Round<T> {
    fn round_to(&self, digit: u8) -> T;
    fn clip(&self, min: T, max: T) -> T;
    fn count_decimal_places(&self) -> usize;
}
impl Round<f64> for f64 {
    /// Rounds the given float to the specified number of decimal places.
    ///
    /// # Panics
    ///
    /// Panics if the number of digits is greater than 17 (the maximum precision for a `f64`).
    ///
    /// # Examples
    ///
    ///
    fn round_to(&self, digits: u8) -> f64 {
        assert!(digits <= 17, "Maximum precision for f64 is 17 digits");
        let factor = 10.0f64.powi(digits as i32);
        (self * factor).trunc() / factor
    }

    /// Clips a value to a given range.
    ///
    /// # Examples
    ///
    ///
    fn clip(&self, min: f64, max: f64) -> f64 {
        self.max(min).min(max)
    }

    /// Counts the number of decimal places in the given float.
    ///
    /// # Notes
    ///
    /// This function uses a naive approach to count the decimal places. It works
    /// by repeatedly multiplying the number by 10 until the fractional part is
    /// 0.0, then counting the number of multiplications that took place.
    ///
    /// # Limitations
    ///
    /// This function has a hard-coded limit of 20 decimal places. If the number
    /// has more than 20 decimal places, the function will return 20.
    fn count_decimal_places(&self) -> usize {
        if self.is_nan() || self.is_infinite() || self.fract() == 0.0 {
            return 0;
        }

        let s = format!("{}", self);
        match s.find('.') {
            Some(pos) => s[pos + 1..].trim_end_matches('0').len(),
            None => 0,
        }
    }
}
