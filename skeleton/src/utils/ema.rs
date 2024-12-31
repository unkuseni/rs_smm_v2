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

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

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