use skeleton::{
    exchange::exchange::{Exchange, MarketData, TradeType},
    ss::SharedState,
    utils::models::{BybitBook, BybitClient, BybitPrivate},
};
use std::{collections::HashMap, time::Duration};
use tokio::{sync::mpsc, time::interval};

use crate::{features::engine::Engine, trader::quote_gen::QuoteGenerator};

pub struct Maker {
    pub features: HashMap<String, Engine>,
    pub previous_book: HashMap<String, BybitBook>,
    pub previous_trades: HashMap<String, TradeType>,
    pub current_trades: HashMap<String, TradeType>,
    pub previous_avg_trade_price: HashMap<String, f64>,
    pub generators: HashMap<String, QuoteGenerator>,
    pub depths: Vec<usize>,
    pub tick_window: usize,
}

impl Maker {
    pub async fn new(
        ss: SharedState,
        symbols: Vec<String>,
        asset: HashMap<String, f64>,
        leverage: f64,
        orders_per_side: usize,
        rate_limit: usize,
        tick_window: usize,
        depths: Vec<usize>,
    ) -> Self {
        Self {
            features: Self::build_features(symbols, tick_window),
            previous_book: HashMap::new(),
            previous_trades: HashMap::new(),
            current_trades: HashMap::new(),
            previous_avg_trade_price: HashMap::new(),
            generators: Self::build_generators(
                ss.clients,
                asset,
                leverage,
                orders_per_side,
                tick_window,
                rate_limit,
            )
            .await,
            depths,
            tick_window,
        }
    }

    pub async fn start_loop(&mut self, mut receiver: mpsc::UnboundedReceiver<SharedState>) {
        let mut update_grid_send = 0;
        let mut wait = interval(Duration::from_secs(3));

        while let Some(new_state) = receiver.recv().await {
            match new_state.exchange.as_str() {
                "bybit" | "binance" => {
                    let market_data = new_state.markets[0].clone();
                    let depths = self.depths.clone();

                    self.update_features(market_data.clone(), depths);

                    if update_grid_send > self.tick_window {
                        // update grid orders
                        self.potentially_update(new_state.privates, market_data)
                            .await;
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

    fn update_features(&mut self, market_data: MarketData, depths: Vec<usize>) {
        match market_data {
            MarketData::Bybit(info) => {
                for (symbol, current_book) in info.books {
                    if let (
                        Some(previous_book),
                        Some(previous_trades),
                        Some(current_trades),
                        Some(previous_avg_trade_price),
                        Some(features),
                    ) = (
                        self.previous_book.get(&symbol),
                        self.previous_trades.get(&symbol),
                        self.current_trades.get(&symbol),
                        self.previous_avg_trade_price.get(&symbol),
                        self.features.get_mut(&symbol),
                    ) {
                        features.update_engine(
                            current_book,
                            previous_book,
                            current_trades,
                            previous_trades,
                            *previous_avg_trade_price,
                            depths.clone(),
                        );
                    }
                }
            }
            MarketData::Binance(_) => {}
        }
    }

    async fn build_generators(
        clients: HashMap<String, BybitClient>,
        asset: HashMap<String, f64>,
        leverage: f64,
        orders_per_side: usize,
        tick_window: usize,
        rate_limit: usize,
    ) -> HashMap<String, QuoteGenerator> {
        let mut generators = HashMap::new();
        for (symbol, client) in clients {
            let _ = client.set_leverage(symbol.as_str(), leverage as u8).await;
            generators.insert(
                symbol.clone(),
                QuoteGenerator::new(
                    client,
                    asset.get(&symbol).unwrap().clone(),
                    leverage,
                    orders_per_side,
                    tick_window,
                    rate_limit,
                )
                .await,
            );
        }
        generators
    }

    async fn potentially_update(
        &mut self,
        private: HashMap<String, BybitPrivate>,
        data: MarketData,
    ) {
        match data {
            MarketData::Bybit(info) => {
                for (symbol, book) in info.books {
                    if let (Some(engine), Some(generator)) =
                        (self.features.get(&symbol), self.generators.get_mut(&symbol))
                    {
                        let skew = engine.skew;
                        let volatility = engine.volatility.current_vol;
                        if let Some(private) = private.get(&symbol) {
                            generator
                                .update_grid(private.clone(), skew, book, symbol, volatility)
                                .await;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn set_spread_toml(&mut self, bps: Vec<f64>) {
        let mut index = 0;
        for (_, v) in self.generators.iter_mut() {
            // Set the spread for the current generator
            v.set_min_spread(bps[index]);
            // Move to the next spread value
            index += 1;
        }
    }
}
