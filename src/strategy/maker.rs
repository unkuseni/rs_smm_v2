use skeleton::{
    exchange::exchange::{MarketData, TradeType},
    ss::SharedState,
    utils::{localorderbook::OrderBook, models::BybitBook},
};
use std::{collections::HashMap, time::Duration};
use tokio::{sync::mpsc, time::interval};

use crate::features::{self, engine::Engine};

pub struct Maker<T: OrderBook> {
    pub features: HashMap<String, Engine>,
    pub previous_book: HashMap<String, T>,
    pub previous_trades: HashMap<String, TradeType>,
    pub current_trades: HashMap<String, TradeType>,
    pub previous_avg_trade_price: HashMap<String, f64>,
    pub depths: Vec<usize>,
    pub tick_window: usize,
}

impl<T: OrderBook> Maker<T> {
    pub fn new(symbols: Vec<String>, tick_window: usize, depths: Vec<usize>) -> Self {
        Self {
            features: Self::build_features(symbols, tick_window),
            previous_book: HashMap::new(),
            previous_trades: HashMap::new(),
            current_trades: HashMap::new(),
            previous_avg_trade_price: HashMap::new(),
            depths,
            tick_window,
        }
    }

    pub async fn start_loop(&mut self, mut receiver: mpsc::UnboundedReceiver<SharedState>) {
        let mut update_grid_send = 0;
        let mut wait = interval(Duration::from_millis(100));

        while let Some(new_state) = receiver.recv().await {
            match new_state.exchange.as_str() {
                "bybit" | "binance" => {
                    let market_data = new_state.markets[0].clone();
                    let depths = self.depths.clone();
                    self.update_features(market_data, depths);

                    if update_grid_send > self.tick_window {
                        // update grid orders
                    } else {
                        wait.tick().await;
                        update_grid_send += 1;
                    }
                }
                &_ => {}
            }
        }
    }

    fn build_features(symbols: Vec<String>, tick_window: usize) -> HashMap<String, Engine> {
        let mut features = HashMap::new();
        for symbol in symbols {
            features.insert(symbol.clone(), Engine::new(tick_window));
        }
        features
    }

    fn update_features(&mut self, market_data: MarketData, depths: Vec<usize>) {}
}
