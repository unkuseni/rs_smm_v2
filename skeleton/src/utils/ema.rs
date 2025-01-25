use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct EMA {
    window: usize,
    alpha: f64,
    value: f64,
    initialized: bool,
    history: Option<VecDeque<f64>>,  // Now memory-efficient
}

impl EMA {
    /// Creates a new EMA with history disabled by default
    pub fn new(window: usize) -> Self {
        let window = window.max(1);
        let alpha = 2.0 / (window + 1) as f64;
        
        Self {
            window,
            alpha,
            value: 0.0,
            initialized: false,
            history: None,
        }
    }

    /// Creates EMA with custom alpha (0 < alpha <= 1)
    pub fn with_alpha(alpha: f64) -> Self {
        let alpha = alpha.clamp(f64::EPSILON, 1.0);  // Prevent division by zero
        let window = ((2.0 / alpha) - 1.0).ceil() as usize;
        
        Self {
            window: window.max(1),
            alpha,
            value: 0.0,
            initialized: false,
            history: None,
        }
    }

    /// Enable circular buffer for historical values (opt-in)
    pub fn enable_history(&mut self) {
        if self.history.is_none() {
            self.history = Some(VecDeque::with_capacity(self.window));
        }
    }

    /// Update EMA with new price, O(1) time complexity
    pub fn update(&mut self, price: f64) -> f64 {
        self.value = if !self.initialized {
            self.initialized = true;
            price
        } else {
            self.alpha.mul_add(price, (1.0 - self.alpha) * self.value)
        };

        // Maintain history if enabled
        if let Some(values) = &mut self.history {
            if values.len() == self.window {
                values.pop_front();
            }
            values.push_back(self.value);
        }

        self.value
    }

    /// Get current EMA value
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Get historical values if enabled
    pub fn history(&self) -> Option<&VecDeque<f64>> {
        self.history.as_ref()
    }

    /// Reset to initial state (preserves history config)
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.initialized = false;
        if let Some(values) = &mut self.history {
            values.clear();
        }
    }
}

impl Default for EMA {
    fn default() -> Self {
        Self::new(14)
    }
}