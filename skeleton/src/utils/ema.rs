use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct EMA {
    window: usize,
    alpha: f64,
    value: f64,
    initialized: bool,
    values: VecDeque<f64>,  // Optional: only if you need history
}

impl EMA {
    /// Creates a new `EMA` with the given window size.
    ///
    /// The alpha value will be calculated as 2 / (window + 1).
    ///
    /// # Returns
    ///
    /// A new `EMA` instance.
    pub fn new(window: usize) -> Self {
        // Ensure window size is at least 1
        let window = window.max(1);
        let alpha = 2.0 / (window + 1) as f64;
        
        Self {
            window,
            alpha,
            value: 0.0,
            initialized: false,
            values: VecDeque::with_capacity(window),
        }
    }

    /// Creates a new `EMA` with the given alpha value.
    ///
    /// The alpha value is used to calculate the window size using the formula:
    ///
    /// window = (2 / alpha) - 1
    ///
    /// # Returns
    ///
    /// A new `EMA` instance.
    pub fn with_alpha(alpha: f64) -> Self {
        // Ensure alpha is between 0 and 1
        let alpha = alpha.clamp(0.0, 1.0);
        
        Self {
            window: ((2.0 / alpha) - 1.0) as usize,
            alpha,
            value: 0.0,
            initialized: false,
            values: VecDeque::new(),
        }
    }

    /// Updates the EMA with the given price.
    ///
    /// If the EMA has not been initialized yet, the price is set as the initial value.
    /// Otherwise, the EMA is calculated using the formula:
    ///
    /// EMA = alpha * price + (1 - alpha) * previous_ema
    ///
    /// # Returns
    ///
    /// The updated EMA value.
    pub fn update(&mut self, price: f64) -> f64 {
        if !self.initialized {
            self.value = price;
            self.initialized = true;
        } else {
            self.value = self.alpha * price + (1.0 - self.alpha) * self.value;
        }

        // Only if you need to maintain history
        if self.values.len() == self.window {
            self.values.pop_front();
        }
        self.values.push_back(self.value);

        self.value
    }

    /// Returns the current EMA value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns whether the EMA has been initialized yet.
    ///
    /// The EMA is considered initialized after the first call to `update`.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Resets the EMA to its initial state.
    ///
    /// Sets the current value to 0.0, sets `initialized` to false, and clears the history.
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.initialized = false;
        self.values.clear();
    }

    // Optional: Get the history of EMA values
    pub fn history(&self) -> &VecDeque<f64> {
        &self.values
    }
}

// Implement Default if needed
impl Default for EMA {
    fn default() -> Self {
        Self::new(14) // Common default window size
    }
}