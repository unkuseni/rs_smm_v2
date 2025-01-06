#[derive(Debug, Clone)]
pub struct Engine {
    pub bba_imbalance: (f64, f64),
    pub deep_imbalance: (Vec<f64>, Vec<f64>),
    pub voi: (f64, f64),
    pub ofi: (f64, f64),
    pub trade_imbalance: (f64, f64),
    pub price_impact: (f64, f64),
    pub expected_return: (ExpectedReturn, ExpectedReturn),
    pub rate_of_change: (ROC, ROC),
    pub mid_price: MPB,
    pub skew: f64,
}

impl Engine {
    pub fn new(tick_window: usize) -> Self {
        Self {
            bba_imbalance: (0.0, 0.0),
            deep_imbalance: (Vec::new(), Vec::new()),
            voi: (0.0, 0.0),
            ofi: (0.0, 0.0),
            trade_imbalance: (0.0, 0.0),
            price_impact: (0.0, 0.0),
            expected_return: (
                ExpectedReturn::new(tick_window),
                ExpectedReturn::new(tick_window),
            ),
            rate_of_change: (ROC::new(tick_window), ROC::new(tick_window)),
            mid_price: MPB::new(tick_window),
            skew: 0.0,
        }
    }

    pub fn set_bba_imbalance(&mut self, imbalance: (f64, f64)) {
        self.bba_imbalance = imbalance;
    }

    pub fn get_bba_imbalance(&self) -> (f64, f64) {
        self.bba_imbalance
    }

    pub fn set_deep_imbalance(&mut self, imbalance: (Vec<f64>, Vec<f64>)) {
        self.deep_imbalance = imbalance;
    }

    pub fn get_deep_imbalance(&self) -> (Vec<f64>, Vec<f64>) {
        self.deep_imbalance.clone()
    }

    pub fn set_voi(&mut self, voi: (f64, f64)) {
        self.voi = voi;
    }

    pub fn get_voi(&self) -> (f64, f64) {
        self.voi
    }

    pub fn set_ofi(&mut self, ofi: (f64, f64)) {
        self.ofi = ofi;
    }

    pub fn get_ofi(&self) -> (f64, f64) {
        self.ofi
    }

    pub fn set_trade_imbalance(&mut self, imbalance: (f64, f64)) {
        self.trade_imbalance = imbalance;
    }

    pub fn get_trade_imbalance(&self) -> (f64, f64) {
        self.trade_imbalance
    }

    pub fn set_price_impact(&mut self, impact: (f64, f64)) {
        self.price_impact = impact;
    }

    pub fn get_price_impact(&self) -> (f64, f64) {
        self.price_impact
    }

    pub fn set_expected_return(&mut self, expected_return: (ExpectedReturn, ExpectedReturn)) {
        self.expected_return = expected_return;
    }

    pub fn get_expected_return(&self) -> (ExpectedReturn, ExpectedReturn) {
        self.expected_return.clone()
    }

    pub fn set_rate_of_change(&mut self, rate_of_change: (ROC, ROC)) {
        self.rate_of_change = rate_of_change;
    }

    pub fn get_rate_of_change(&self) -> (ROC, ROC) {
        self.rate_of_change.clone()
    }

    pub fn set_mid_price(&mut self, mid_price: MPB) {
        self.mid_price = mid_price;
    }

    pub fn get_mid_price(&self) -> MPB {
        self.mid_price.clone()
    }

    pub fn generate_skew(&mut self) {}
}

#[derive(Debug, Clone)]
pub struct ROC {
    pub current_rate_of_change: f64,
    pub rate_of_change: Vec<f64>,
}

impl ROC {
    pub fn new(tick_window: usize) -> Self {
        Self {
            current_rate_of_change: 0.0,
            rate_of_change: Vec::with_capacity(tick_window),
        }
    }

    pub fn set_current_rate_of_change(&mut self) {
        if let Some(last) = self.rate_of_change.last() {
            self.current_rate_of_change = *last;
        } else {
            self.current_rate_of_change = 0.0;
        }
    }

    pub fn update_rate_of_change(&mut self, rate_of_change: f64) {
        if self.rate_of_change.len() == self.rate_of_change.capacity() {
            self.rate_of_change.remove(0);
        }
        self.rate_of_change.push(rate_of_change);
    }

    pub fn get_mean_roc(&self) -> f64 {
        self.rate_of_change.iter().sum::<f64>() / self.rate_of_change.len() as f64
    }
}

#[derive(Debug, Clone)]
pub struct ExpectedReturn {
    pub same_expected_return: f64,
    pub cross_expected_return: f64,
    pub cross_return_array: Vec<f64>,
    pub same_return_array: Vec<f64>,
}

impl ExpectedReturn {
    pub fn new(tick_window: usize) -> Self {
        Self {
            same_expected_return: 0.0,
            cross_expected_return: 0.0,
            cross_return_array: Vec::with_capacity(tick_window),
            same_return_array: Vec::with_capacity(tick_window),
        }
    }

    pub fn set_same_expected_return(&mut self, same_expected_return: f64) {
        self.same_expected_return = same_expected_return;
        if self.same_return_array.len() == self.same_return_array.capacity() {
            self.same_return_array.remove(0);
        }
        self.same_return_array.push(same_expected_return);
    }

    pub fn get_same_expected_return(&self) -> f64 {
        self.same_expected_return
    }

    pub fn set_cross_expected_return(&mut self, cross_expected_return: f64) {
        self.cross_expected_return = cross_expected_return;
        if self.cross_return_array.len() == self.cross_return_array.capacity() {
            self.cross_return_array.remove(0);
        }
        self.cross_return_array.push(cross_expected_return);
    }

    pub fn get_cross_expected_return(&self) -> f64 {
        self.cross_expected_return
    }

    pub fn get_mean_cross_return(&self) -> f64 {
        if self.cross_return_array.is_empty() {
            return 0.0;
        }
        self.cross_return_array.iter().sum::<f64>() / self.cross_return_array.len() as f64
    }

    pub fn get_mean_same_return(&self) -> f64 {
        if self.same_return_array.is_empty() {
            return 0.0;
        }
        self.same_return_array.iter().sum::<f64>() / self.same_return_array.len() as f64
    }

    pub fn get_same_vol(&self) -> f64 {
        if self.same_return_array.len() <= 1 {
            return 0.0;
        }
        let mean_return = self.get_mean_same_return();
        let variance = self
            .same_return_array
            .iter()
            .map(|x| (x - mean_return).powi(2))
            .sum::<f64>()
            / (self.same_return_array.len() - 1) as f64;
        variance.sqrt()
    }

    pub fn get_cross_vol(&self) -> f64 {
        if self.cross_return_array.len() <= 1 {
            return 0.0;
        }
        let mean_return = self.get_mean_cross_return();
        let variance = self
            .cross_return_array
            .iter()
            .map(|x| (x - mean_return).powi(2))
            .sum::<f64>()
            / (self.cross_return_array.len() - 1) as f64;
        variance.sqrt()
    }
}

#[derive(Debug, Clone)]
pub struct MPB {
    pub mid_price_basis: f64,
    pub mean_basis: f64,
    pub basis_array: Vec<f64>,
}

impl MPB {
    pub fn new(tick_window: usize) -> Self {
        Self {
            mid_price_basis: 0.0,
            mean_basis: 0.0,
            basis_array: Vec::with_capacity(tick_window),
        }
    }

    pub fn set_mid_price_basis(&mut self, mid_price_basis: f64) {
        self.mid_price_basis = mid_price_basis;
        if self.basis_array.len() == self.basis_array.capacity() {
            self.basis_array.remove(0);
        }
        self.basis_array.push(mid_price_basis);
    }

    pub fn get_mid_price_basis(&self) -> f64 {
        self.mid_price_basis
    }

    pub fn get_mean_basis(&self) -> f64 {
        if self.basis_array.is_empty() {
            return 0.0;
        }
        self.basis_array.iter().sum::<f64>() / self.basis_array.len() as f64
    }

    pub fn get_volatility(&self) -> f64 {
        if self.basis_array.len() <= 1 {
            return 0.0;
        }
        let mean_basis = self.get_mean_basis();
        let variance = self
            .basis_array
            .iter()
            .map(|x| (x - mean_basis).powi(2))
            .sum::<f64>()
            / (self.basis_array.len() - 1) as f64;
        variance.sqrt()
    }
}
