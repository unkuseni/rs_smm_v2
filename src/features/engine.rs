use std::collections::VecDeque;

use skeleton::{
    exchange::exchange::TradeType,
    utils::{localorderbook::OrderBook, models::BybitBook, vol::RollingVolatility},
};

use super::{
    impact::{mid_price_avg, rate_of_change},
    trade::{avg_trade_price, trade_imbalance},
};
#[derive(Debug, Clone)]
pub struct Engine {
    pub timestamp: u64,
    pub bba_imbalance: f64,
    pub deep_imbalance: Vec<f64>,
    pub voi: f64,
    pub ofi: f64,
    pub trade_imbalance: f64,
    pub price_impact: f64,
    pub volatility: RollingVolatility,
    pub rate_of_change: ROC,
    pub mpb: MPB,
    pub skew: f64,
}

impl Engine {
    pub fn new(tick_window: usize) -> Self {
        Self {
            timestamp: 0,
            bba_imbalance: 0.0,
            deep_imbalance: Vec::new(),
            voi: 0.0,
            ofi: 0.0,
            trade_imbalance: 0.0,
            price_impact: 0.0,
            volatility: RollingVolatility::new(tick_window),
            rate_of_change: ROC::new(tick_window),
            mpb: MPB::new(tick_window),
            skew: 0.0,
        }
    }

    fn set_bba_imbalance(&mut self, imbalance: f64) {
        self.bba_imbalance = imbalance;
    }

    pub fn get_bba_imbalance(&self) -> f64 {
        self.bba_imbalance
    }

    fn set_deep_imbalance(&mut self, imbalance: Vec<f64>) {
        self.deep_imbalance = imbalance;
    }

    pub fn get_deep_imbalance(&self) -> Vec<f64> {
        self.deep_imbalance.clone()
    }

    fn set_voi(&mut self, voi: f64) {
        self.voi = voi;
    }

    pub fn get_voi(&self) -> f64 {
        self.voi
    }

    fn set_ofi(&mut self, ofi: f64) {
        self.ofi = ofi;
    }

    pub fn get_ofi(&self) -> f64 {
        self.ofi
    }

    fn set_trade_imbalance(&mut self, imbalance: f64) {
        self.trade_imbalance = imbalance;
    }

    pub fn get_trade_imbalance(&self) -> f64 {
        self.trade_imbalance
    }

    fn set_price_impact(&mut self, impact: f64) {
        self.price_impact = impact;
    }

    pub fn get_price_impact(&self) -> f64 {
        self.price_impact
    }

    pub fn get_volatility(&self) -> f64 {
        self.volatility.clone().current_vol
    }

    pub fn get_rate_of_change(&self) -> ROC {
        self.rate_of_change.clone()
    }

    pub fn get_mpb(&self) -> MPB {
        self.mpb.clone()
    }

    pub fn get_skew(&self) -> f64 {
        self.skew
    }

    pub fn update_engine(
        &mut self,
        current_book: BybitBook,
        previous_book: &BybitBook,
        current_trades: &TradeType,
        previous_trades: &TradeType,
        prev_avg_trade_price: f64,
        depth: Vec<usize>,
    ) {
        if self.timestamp == 0 || (self.timestamp - current_book.last_update) >= 1000 {
            self.set_bba_imbalance(current_book.imbalance_ratio(None));

            let deep_imbalance = depth[0..]
                .iter()
                .map(|x| current_book.imbalance_ratio(Some(*x)))
                .collect();

            self.set_deep_imbalance(deep_imbalance);

            let voi = current_book.voi(&previous_book, None);

            self.set_voi(voi);

            let ofi = current_book.ofi(&previous_book, None);

            self.set_ofi(ofi);

            self.set_trade_imbalance(trade_imbalance(current_trades));

            let impact = current_book.price_impact(&previous_book, None);
            self.set_price_impact(impact);

            self.volatility.update(current_book.get_mid_price());

            self.rate_of_change.update(rate_of_change(
                previous_book.get_mid_price(),
                current_book.get_mid_price(),
            ));

            let avg_trade_price = avg_trade_price(
                current_book.get_mid_price(),
                Some(previous_trades),
                current_trades,
                prev_avg_trade_price,
            );
            self.mpb.update_basis(
                avg_trade_price
                    - mid_price_avg(previous_book.get_mid_price(), current_book.get_mid_price()),
            );

            self.generate_skew();
        }
    }

    pub fn generate_skew(&mut self) {
        // 1. Order Flow Signal
        let order_flow = if self.ofi > 0.0 && self.voi > 0.0 {
            1.0 // Strong buying pressure
        } else if self.ofi < 0.0 && self.voi < 0.0 {
            -1.0 // Strong selling pressure
        } else {
            0.5 // Neutral/mixed signals
        };

        // 2. Core components for skew calculation
        let trade_skew = self.trade_imbalance.clamp(-1.0, 1.0);
        let book_skew = self.bba_imbalance.clamp(-1.0, 1.0);

        // 3. Depth-weighted imbalance gradient
        let depth_mean = if self.deep_imbalance.is_empty() {
            0.0
        } else {
            self.deep_imbalance.iter().sum::<f64>() / self.deep_imbalance.len() as f64
        };

        // 4. Volatility-adjusted basis skew
        let basis_skew = if self.mpb.std_dev() > 0.0 {
            self.mpb.z_score().tanh() // Normalize z-score to [-1, 1]
        } else {
            self.mpb.current_basis().signum()
        };

        // 5. Momentum-adjusted weighting factors
        let momentum_factor = self.rate_of_change.z_score().tanh().abs();
        let volatility_factor = 1.0 / (self.volatility.current_vol.max(0.001));

        // 6. Composite skew calculation with order flow
        let raw_skew = 0.3 * trade_skew
            + 0.25 * book_skew
            + 0.2 * depth_mean
            + 0.15 * basis_skew
            + 0.1 * order_flow;

        // 7. Apply momentum and volatility scaling
        self.skew = (raw_skew * momentum_factor * volatility_factor)
            .tanh() // Ensure final value stays in [-1, 1]
            .clamp(-1.0, 1.0);
    }
}

#[derive(Debug, Clone)]
pub struct ROC {
    window_size: usize,
    values: VecDeque<f64>,
    sum: f64,
    sum_squares: f64,
}

impl ROC {
    /// Creates a new ROC calculator with guaranteed minimum window size of 2
    pub fn new(window_size: usize) -> Self {
        let window_size = window_size.max(2); // Need at least 2 values for meaningful z-score
        Self {
            window_size,
            values: VecDeque::with_capacity(window_size),
            sum: 0.0,
            sum_squares: 0.0,
        }
    }

    /// Updates the ROC with a new value in O(1) time
    pub fn update(&mut self, new_value: f64) {
        // Maintain sliding window invariant
        if self.values.len() == self.window_size {
            if let Some(old_value) = self.values.pop_front() {
                self.sum -= old_value;
                self.sum_squares -= old_value.powi(2);
            }
        }
        self.values.push_back(new_value);
        self.sum += new_value;
        self.sum_squares += new_value.powi(2);
    }

    /// Gets current rate of change in O(1) time
    pub fn current(&self) -> f64 {
        self.values.back().copied().unwrap_or(0.0)
    }

    /// Calculates mean ROC in O(1) time
    pub fn mean(&self) -> f64 {
        if self.values.is_empty() {
            0.0
        } else {
            self.sum / self.values.len() as f64
        }
    }

    /// Calculates population standard deviation in O(1) time
    pub fn std_dev(&self) -> f64 {
        if self.values.len() < 2 {
            return 0.0;
        }

        let n = self.values.len() as f64;
        let mean = self.mean();
        let variance = (self.sum_squares / n) - mean.powi(2);

        variance.max(0.0).sqrt() // Prevent negative variance from floating point errors
    }
    /// Calculates Z-score for current value in O(1) time
    pub fn z_score(&self) -> f64 {
        let current = self.current();
        let mean = self.mean();
        let std_dev = self.std_dev();

        if std_dev == 0.0 {
            0.0
        } else {
            (current - mean) / std_dev
        }
    }

    /// Gets full history slice for advanced analysis
    pub fn history(&self) -> Vec<f64> {
        let mut values = self.values.clone();
        values.make_contiguous();
        values.into()
    }
}

#[derive(Debug, Clone)]
pub struct MPB {
    mid_price_basis: f64,
    basis_array: VecDeque<f64>,
    sum: f64,
    sum_squares: f64,
}

impl MPB {
    pub fn new(tick_window: usize) -> Self {
        Self {
            mid_price_basis: 0.0,
            basis_array: VecDeque::with_capacity(tick_window),
            sum: 0.0,
            sum_squares: 0.0,
        }
    }

    pub fn update_basis(&mut self, new_basis: f64) {
        // Maintain sliding window
        if self.basis_array.len() == self.basis_array.capacity() {
            if let Some(old_value) = self.basis_array.pop_front() {
                self.sum -= old_value;
                self.sum_squares -= old_value.powi(2);
            }
        }

        self.basis_array.push_back(new_basis);
        self.sum += new_basis;
        self.sum_squares += new_basis.powi(2);
        self.mid_price_basis = new_basis;
    }

    // Get current basis value
    pub fn current_basis(&self) -> f64 {
        self.mid_price_basis
    }

    // Calculate mean in O(1) time
    pub fn mean(&self) -> f64 {
        if self.basis_array.is_empty() {
            0.0
        } else {
            self.sum / self.basis_array.len() as f64
        }
    }

    // Calculate population standard deviation in O(1) time
    pub fn std_dev(&self) -> f64 {
        if self.basis_array.len() < 2 {
            return 0.0;
        }

        let n = self.basis_array.len() as f64;
        let mean = self.mean();
        let variance = (self.sum_squares / n) - mean.powi(2);

        variance.max(0.0).sqrt()
    }

    // Calculate Z-score for current basis
    pub fn z_score(&self) -> f64 {
        let std_dev = self.std_dev();
        if std_dev == 0.0 {
            0.0
        } else {
            (self.mid_price_basis - self.mean()) / std_dev
        }
    }

    // Get full history for external analysis
    pub fn history(&self) -> &VecDeque<f64> {
        &self.basis_array
    }
}
