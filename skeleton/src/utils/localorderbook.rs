pub trait OrderBook {
    type Ask;
    type Bid;
    fn new() -> Self;
    fn update_bba(
        &mut self,
        asks: Vec<Self::Ask>,
        bids: Vec<Self::Bid>,
        timestamp: u64,
        sequence: u64,
    );
    fn update(&mut self, asks: Vec<Self::Ask>, bids: Vec<Self::Bid>, timestamp: u64, levels: usize);
    fn reset(&mut self, asks: Vec<Self::Ask>, bids: Vec<Self::Bid>, timestamp: u64, sequence: u64);
    fn set_mid_price(&mut self);
    fn get_mid_price(&self) -> f64;
    fn get_depth(&self, depth: usize) -> (Vec<Self::Ask>, Vec<Self::Bid>);
    fn get_best_ask(&self) -> Self::Ask;
    fn get_best_bid(&self) -> Self::Bid;
    fn get_bba(&self) -> (Self::Ask, Self::Bid);
    fn get_spread(&self) -> f64;
    fn get_spread_in_ticks(&self) -> f64;
    fn get_tick_size(&self) -> f64;
    fn get_lot_size(&self) -> f64;
    fn get_min_notional(&self) -> f64;
    fn get_post_only_max_qty(&self) -> f64;
    fn min_qty(&self) -> f64;
    fn get_wmid(&self, depth: Option<usize>) -> f64;
    fn effective_spread(&self, is_buy: bool) -> f64;
    fn get_microprice(&self, depth: Option<usize>) -> f64;
    fn imbalance_ratio(&self, depth: Option<usize>) -> f64;
    fn price_impact(&self, old_book: &Self, depth: Option<usize>) -> f64;
    fn ofi(&self, old_book: &Self, depth: Option<usize>) -> f64;
    fn voi(&self, old_book: &Self, depth: Option<usize>) -> f64;
    fn calculate_weighted_ask(&self, depth: usize, decay_rate: Option<f64>) -> f64;
    fn calculate_weighted_bid(&self, depth: usize, decay_rate: Option<f64>) -> f64;
}
