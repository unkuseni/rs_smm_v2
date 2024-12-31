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
}
