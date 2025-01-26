use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct RollingVolatility {
    window_size: usize,
    returns: VecDeque<f64>,
    sum: f64,
    sum_squares: f64,
    last_price: Option<f64>,
   pub current_vol: f64,
}

impl RollingVolatility {
    /// Creates a new RollingVolatility with specified window size
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            returns: VecDeque::with_capacity(window_size),
            sum: 0.0,
            sum_squares: 0.0,
            last_price: None,
            current_vol: 0.0,
        }
    }

    /// Update with new price and return current volatility and Z-score if available
    pub fn update(&mut self, price: f64) -> Option<(f64, f64)> {
        if let Some(prev_price) = self.last_price.replace(price) {
            // Calculate log return: ln(price/prev_price)
            let ret = (price / prev_price).ln();

            // Maintain rolling window
            if self.returns.len() == self.window_size {
                if let Some(old_ret) = self.returns.pop_front() {
                    self.sum -= old_ret;
                    self.sum_squares -= old_ret.powi(2);
                }
            }

            self.returns.push_back(ret);
            self.sum += ret;
            self.sum_squares += ret.powi(2);
        }

        // Only calculate volatility and Z-score when window has enough data
        if self.returns.len() >= 2 {
            let n = self.returns.len() as f64;
            let mean = self.sum / n;
            let vol = self.calculate_volatility();

            // Safely unwrap because we have at least 2 returns
            let latest_ret = *self.returns.back().unwrap();

            // Handle division by zero if volatility is 0
            let z_score = if vol == 0.0 {
                0.0
            } else {
                (latest_ret - mean) / vol
            };
            self.current_vol = vol;
            Some((vol, z_score))
        } else {
            None
        }
    }

    /// Calculate current volatility (non-annualized)
    fn calculate_volatility(&self) -> f64 {
        let n = self.returns.len() as f64;
        let mean = self.sum / n;
        let variance = (self.sum_squares / n) - mean.powi(2);
        variance.sqrt().max(0.0) // Ensure non-negative
    }

    /// Get current number of observations in window
    pub fn current_count(&self) -> usize {
        self.returns.len()
    }

    /// Clear all historical data
    pub fn reset(&mut self) {
        self.returns.clear();
        self.sum = 0.0;
        self.sum_squares = 0.0;
        self.last_price = None;
        self.current_vol = 0.0;
    }
}
