/// Calculates the expected price movement using a linear model.
/// 
/// Formula: `curr_price + (curr_price - old_price) * imbalance`
#[inline]
pub fn expected_value(old_price: f64, curr_price: f64, imbalance: f64) -> f64 {
    curr_price + ((curr_price - old_price) * imbalance)
}

/// Calculates expected price using a non-linear model with signed power scaling.
/// 
/// Formula: `curr_price + (price_diff * imbalance * |imbalance|^0.5)`
/// 
/// # Panics
/// In debug builds, panics if imbalance is not in [-1.0, 1.0]
#[inline]
pub fn improved_expected_value(old_price: f64, curr_price: f64, imbalance: f64) -> f64 {
    debug_assert!(
        (-1.0..=1.0).contains(&imbalance),
        "Imbalance must be between -1 and 1"
    );
    
    let price_diff = curr_price - old_price;
    let abs_imb = imbalance.abs();
    
    // Equivalent to: price_diff * imbalance * abs_imb.sqrt()
    // Maintains correct sign while applying non-linear scaling
    curr_price + price_diff * imbalance * abs_imb.sqrt()
}

/// Calculates price difference between current and old mid prices
#[inline(always)]
pub fn mid_price_diff(old_price: f64, curr_price: f64) -> f64 {
    curr_price - old_price
}

/// Calculates average price between two mid prices
#[inline(always)]
pub fn mid_price_avg(old_mid: f64, curr_mid: f64) -> f64 {
    (old_mid + curr_mid) * 0.5
}

/// Calculates logarithmic return percentage between two prices
/// 
/// Formula: `ln(curr_price / old_price) * 100`
#[inline(always)]
pub fn log_return_percent(old_price: f64, curr_price: f64) -> f64 {
    debug_assert!(old_price > 0.0 && curr_price > 0.0, "Prices must be positive");
    (curr_price / old_price).ln() * 100.0
}

/// Calculates logarithmic return in basis points (1/100th of a percent)
#[inline(always)]
pub fn log_return_bps(old_price: f64, curr_price: f64) -> f64 {
    debug_assert!(old_price > 0.0 && curr_price > 0.0, "Prices must be positive");
    (curr_price / old_price).ln() * 10_000.0
}

/// Calculates simple rate of return percentage
#[inline(always)]
pub fn rate_of_change(old_price: f64, curr_price: f64) -> f64 {
    debug_assert!(old_price > 0.0, "Old price must be positive");
    ((curr_price - old_price) / old_price) * 100.0
}