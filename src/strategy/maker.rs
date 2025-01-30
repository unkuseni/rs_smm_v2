use skeleton::{
    exchange::exchange::{Exchange, MarketData, TradeType},
    ss::SharedState,
    utils::models::{BybitBook, BybitClient, BybitMarket, BybitPrivate},
};
use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};
use tokio::sync::mpsc;

use crate::{features::engine::Engine, trader::quote_gen::QuoteGenerator};

pub struct Maker {
    pub features: BTreeMap<String, Engine>,
    pub previous_book: BTreeMap<String, BybitBook>,
    pub previous_trades: BTreeMap<String, TradeType>,
    pub current_trades: BTreeMap<String, TradeType>,
    pub previous_avg_trade_price: BTreeMap<String, f64>,
    pub generators: BTreeMap<String, QuoteGenerator>,
    pub depths: Vec<usize>,
    pub tick_window: usize,
}

impl Maker {
    pub async fn new(
        ss: SharedState,
        asset: HashMap<String, f64>,
        leverage: f64,
        orders_per_side: usize,
        rate_limit: usize,
        tick_window: usize,
        depths: Vec<usize>,
    ) -> Self {
        Self {
            features: Self::build_features(ss.symbols, tick_window),
            previous_book: BTreeMap::new(),
            previous_trades: BTreeMap::new(),
            current_trades: BTreeMap::new(),
            previous_avg_trade_price: BTreeMap::new(),
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
        let mut send = 0;
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        let mut latest_market_data = None;
        let depths = self.depths.clone();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Some(market_data) = latest_market_data.take() {
                        self.update_features(market_data, &depths);
                        send += 1;
                    }
                },
                Some(ss) = receiver.recv() => {
                    match &ss.markets[0] {
                        MarketData::Bybit(data) => latest_market_data = Some(data.clone()),
                        _ => continue,
                    }
                    if send > self.tick_window {
                        if let Some(market_data) = latest_market_data.take() {
                        self.potentially_update(ss.privates, market_data).await;
                        }
                    }
                }
            }
        }
    }

    fn build_features(symbols: Vec<String>, tick_window: usize) -> BTreeMap<String, Engine> {
        symbols
            .into_iter()
            .map(|symbol| (symbol, Engine::new(tick_window)))
            .collect()
    }

    async fn build_generators(
        clients: BTreeMap<String, BybitClient>,
        mut asset: HashMap<String, f64>,
        leverage: f64,
        orders_per_side: usize,
        tick_window: usize,
        rate_limit: usize,
    ) -> BTreeMap<String, QuoteGenerator> {
        let mut generators = BTreeMap::new();
        let mut tasks = Vec::new();

        for (symbol, client) in clients {
            let Some(asset_value) = asset.remove(&symbol) else {
                eprintln!("Missing asset for {}", symbol);
                continue;
            };

            let symbol_clone = symbol.clone();
            tasks.push(async move {
                let _ = client.set_leverage(&symbol_clone, leverage as u8).await;

                (
                    symbol,
                    QuoteGenerator::new(
                        client,
                        asset_value,
                        leverage,
                        orders_per_side,
                        tick_window,
                        rate_limit,
                    )
                    .await,
                )
            });
        }

        for task in tasks {
            let (symbol, generator) = task.await;
            generators.insert(symbol, generator);
        }

        generators
    }

    fn update_features(&mut self, market_data: BybitMarket, depths: &[usize]) {
        self.current_trades = market_data.trades;

        for (symbol, current_book) in market_data.books {
            let features = match self.features.get_mut(&symbol) {
                Some(f) => f,
                None => continue,
            };

            let (Some(prev_book), Some(prev_trades), Some(curr_trades), Some(prev_avg)) = (
                self.previous_book.get(&symbol),
                self.previous_trades.get(&symbol),
                self.current_trades.get(&symbol),
                self.previous_avg_trade_price.get(&symbol),
            ) else {
                continue;
            };

            features.update_engine(
                &current_book,
                prev_book,
                curr_trades,
                prev_trades,
                *prev_avg,
                depths,
            );

            self.previous_book.insert(symbol.clone(), current_book);

            let avg_price = features.get_avg_trade_price();
            self.previous_avg_trade_price.insert(symbol, avg_price);
        }

        self.previous_trades = std::mem::take(&mut self.current_trades);
    }

    async fn potentially_update(
        &mut self,
        private: BTreeMap<String, BybitPrivate>,
        data: BybitMarket,
    ) {
        for (symbol, book) in data.books {
            if let (Some(engine), Some(generator), Some(private)) = (
                self.features.get(&symbol),
                self.generators.get_mut(&symbol),
                private.get(&symbol),
            ) {
                let skew = engine.get_skew();
                let volatility = engine.get_volatility();

                generator
                    .update_grid(private.clone(), skew, book, symbol, volatility)
                    .await;
            }
        }
    }

    pub fn set_spread_toml(&mut self, bps: Vec<f64>) {
        self.generators
            .values_mut()
            .zip(bps.into_iter())
            .for_each(|(gen, spread)| gen.set_min_spread(spread));
    }
}
