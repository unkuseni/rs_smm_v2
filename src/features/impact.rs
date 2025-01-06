/// Calculates the expected value of a trade given the old price, current price, and imbalance.
///
/// The formula used is as follows: EV = curr_price + (curr_price - old_price) * imbalance
pub fn expected_value(old_price: f64, curr_price: f64, imbalance: f64) -> f64 {
    curr_price + ((curr_price - old_price) * imbalance)
}

/// This function improves upon the simple expected value calculation by
/// using a more advanced model to predict future price movements based on
/// imbalance.
///
/// The formula used is as follows: EV = curr_price + (curr_price - old_price) * abs(imbalance) * sqrt(abs(imbalance)) * sgn(imbalance)
///
/// This function is faster than the simple expected value calculation since
/// it uses a faster approximation for pow(1.5).
pub fn improved_expected_value(old_price: f64, curr_price: f64, imbalance: f64) -> f64 {
    debug_assert!(
        (-1.0..=1.0).contains(&imbalance),
        "Imbalance must be between -1 and 1"
    );

    // Using faster approximation for pow(1.5)
    let price_diff = curr_price - old_price;
    let abs_imb = imbalance.abs();
    curr_price + (price_diff * abs_imb * (abs_imb).sqrt() * imbalance.signum())
}

/// Calculates the difference between the current and old mid prices.
///
/// This function is inlined for performance.
#[inline(always)]
pub fn mid_price_diff(old_price: f64, curr_price: f64) -> f64 {
    curr_price - old_price
}

/// Calculates the average of the old and current mid prices.
///
/// This function is inlined for performance.
///
/// # Arguments
///
/// * `old_mid` - The old mid price.
/// * `curr_mid` - The current mid price.
///
/// # Returns
///
/// The average of the old and current mid prices.
#[inline(always)]
pub fn mid_price_avg(old_mid: f64, curr_mid: f64) -> f64 {
    (old_mid + curr_mid) * 0.5
}

/// Calculates the expected return in percent, given the old and current prices.
///
/// The formula used is as follows: ER = ln(old_price / curr_price) * 100
///
/// # Arguments
///
/// * `old_price` - The old price.
/// * `curr_price` - The current price.
///
/// # Returns
///
/// The expected return in percent.
#[inline(always)]
pub fn expected_return(old_price: f64, curr_price: f64) -> f64 {
    (curr_price / old_price).ln() * 100.0
}

#[inline(always)]
pub fn expected_return_bps(old_price: f64, curr_price: f64) -> f64 {
    (curr_price / old_price).ln() * 10000.0
}

#[inline(always)]
pub fn rate_of_change(old_price: f64, curr_price: f64) -> f64 {
    ((curr_price - old_price) / old_price) * 100.0
}
