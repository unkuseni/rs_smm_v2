use std::{
    collections::{BTreeMap, VecDeque},
    error::Error,
    sync::atomic::AtomicBool,
    thread,
    time::Duration,
};

use binance::{
    api::Binance,
    config::Config,
    futures::{
        account::FuturesAccount,
        general::FuturesGeneral,
        market::FuturesMarket,
        websockets::{FuturesMarket as FuturesMarketWs, FuturesWebSockets, FuturesWebsocketEvent},
    },
    model::{Asks, Bids, DepthOrderBookEvent},
};
use ordered_float::OrderedFloat;
use tokio::task;

use crate::utils::{
    bot::LiveBot,
    localorderbook::OrderBook,
    logger::Logger,
    models::{BinanceBook, BinanceClient, BinanceMarket},
};

use super::exchange::Exchange;

impl Exchange for BinanceClient {
    type TimeOutput = Result<u64, Box<dyn Error>>;
    type FeeOutput = Result<f64, Box<dyn Error>>;
    type LeverageOutput = Result<bool, Box<dyn Error>>;
    type TraderOutput = FuturesAccount;
    type StreamOutput = BinanceMarket;
    type PrivateStreamOutput = ();

    fn init(api_key: String, api_secret: String) -> Self {
        Self {
            api_key,
            api_secret,
            logger: Logger,
            bot: LiveBot::new().unwrap(),
        }
    }

    async fn time(&self) -> Self::TimeOutput {
        let general: FuturesGeneral = Binance::new(None, None);
        let time = task::spawn_blocking(move || match general.get_server_time() {
            Ok(res) => Ok(res.server_time),
            Err(e) => Err(e),
        })
        .await
        .map_err(|e| Box::new(e) as Box<dyn Error>)?; // Handle JoinError;
        Ok(time?)
    }

    async fn fees(&self, _symbol: String) -> Self::FeeOutput {
        let account: FuturesAccount =
            Binance::new(Some(self.api_key.clone()), Some(self.api_secret.clone()));
        let fees = task::spawn_blocking(move || match account.account_information() {
            Ok(res) => Ok(res.fee_tier),
            Err(e) => Err(e),
        })
        .await
        .map_err(|e| Box::new(e) as Box<dyn Error>)?; // Handle JoinError;
        Ok(fees?)
    }

    async fn set_leverage(&self, symbol: &str, leverage: u8) -> Self::LeverageOutput {
        let account: FuturesAccount =
            Binance::new(Some(self.api_key.clone()), Some(self.api_secret.clone()));
        let symbol_str = String::from(symbol);
        let leverage = task::spawn_blocking(move || {
            match account.change_initial_leverage(&symbol_str, leverage) {
                Ok(res) => {
                    if res.leverage == leverage {
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
                Err(e) => Err(e),
            }
        })
        .await
        .map_err(|e| Box::new(e) as Box<dyn Error>)?; // Handle JoinError;
        Ok(leverage?)
    }

    fn trader(&self, recv_window: u16) -> Self::TraderOutput {
        let config = { Config::default().set_recv_window(recv_window as u64) };
        let trader: FuturesAccount = Binance::new_with_config(
            Some(self.api_key.clone()),
            Some(self.api_key.clone()),
            &config,
        );
        trader
    }

    async fn market_subscribe(
        &self,
        symbols: Vec<String>,
        sender: tokio::sync::mpsc::UnboundedSender<Self::StreamOutput>,
    ) -> () {
        let delay = 600;
        let keep_streaming = AtomicBool::new(true);
        let request = build_requests(&symbols);
        let mut market_data = BinanceMarket::default();
        for k in symbols.clone() {
            market_data.books.insert(k.clone(), BinanceBook::new());
            market_data
                .trades
                .insert(k.clone(), VecDeque::with_capacity(5000));
            market_data.ticker.insert(k, VecDeque::with_capacity(10));
        }
        let book_snapshot = (market_data.clone(), symbols.clone());
        let snapshot_update =
            task::spawn_blocking(move || book_snapshot.0.get_book_snapshot(&book_snapshot.1))
                .await
                .unwrap();
        market_data = snapshot_update;

        let handler = move |event| {
            match event {
                FuturesWebsocketEvent::DepthOrderBook(DepthOrderBookEvent {
                    symbol,
                    event_time,
                    bids,
                    asks,
                    final_update_id,
                    ..
                }) => {
                    if let Some(book) = market_data.books.get_mut(&symbol) {
                        market_data.timestamp = event_time;
                        if bids.len() == 20 && asks.len() == 20 {
                            book.update_bba(asks, bids, event_time, final_update_id);
                        } else {
                            book.update(asks, bids, event_time, 20);
                        }
                    }
                }
                FuturesWebsocketEvent::AggrTrades(trade_data) => {
                    if let Some(trades) = market_data.trades.get_mut(&trade_data.symbol) {
                        if trades.len() == trades.capacity()
                            || (trades.capacity() - trades.len()) <= 5
                        {
                            for _ in 0..10 {
                                trades.pop_front();
                            }
                        }
                        trades.push_back(trade_data);
                    }
                }
                FuturesWebsocketEvent::BookTicker(book_ticker) => {
                    if let Some(ticker) = market_data.ticker.get_mut(&book_ticker.symbol) {
                        if ticker.len() == ticker.capacity()
                            || (ticker.capacity() - ticker.len()) <= 10
                        {
                            for _ in 0..10 {
                                ticker.pop_front();
                            }
                        }
                        ticker.push_back(book_ticker);
                    }
                }
                _ => {}
            }
            let _ = sender.send(market_data.clone());
            Ok(())
        };
        let mut market: FuturesWebSockets<'_> = FuturesWebSockets::new(handler);
        loop {
            market
                .connect_multiple_streams(&FuturesMarketWs::USDM, &request)
                .unwrap();

            // check error
            if let Err(e) = market.event_loop(&keep_streaming) {
                eprintln!("Error: {}", e);
                thread::sleep(Duration::from_millis(delay));
            }
        }
    }

    async fn private_subscribe(
        &self,
        symbol: String,
        sender: tokio::sync::mpsc::UnboundedSender<Self::PrivateStreamOutput>,
    ) -> () {
        let delay = 600;
    }
}

impl OrderBook for BinanceBook {
    type Ask = Asks;
    type Bid = Bids;

    fn new() -> Self {
        Self {
            last_update: 0,
            sequence: 0,
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),

            best_ask: Self::Ask {
                price: 0.0,
                qty: 0.0,
            },

            best_bid: Self::Bid {
                price: 0.0,
                qty: 0.0,
            },

            mid_price: 0.0,
        }
    }

    fn reset(&mut self, asks: Vec<Self::Ask>, bids: Vec<Self::Bid>, timestamp: u64, sequence: u64) {
        self.last_update = timestamp;
        self.sequence = sequence;

        for ask in asks.iter() {
            let price = OrderedFloat::from(ask.price);
            self.asks
                .entry(price)
                .and_modify(|qty| *qty = ask.qty)
                .or_insert(ask.qty);
        }

        for bid in bids.iter() {
            let price = OrderedFloat::from(bid.price);
            self.bids
                .entry(price)
                .and_modify(|qty| *qty = bid.qty)
                .or_insert(bid.qty);
        }
        self.asks.retain(|_, &mut v| v != 0.0);
        self.bids.retain(|_, &mut v| v != 0.0);
    }

    fn update_bba(
        &mut self,
        asks: Vec<Self::Ask>,
        bids: Vec<Self::Bid>,
        timestamp: u64,
        sequence: u64,
    ) {
        if timestamp <= self.last_update || sequence <= self.sequence {
            return;
        }

        self.last_update = timestamp;
        self.sequence = sequence;

        let best_bid_price = bids
            .iter()
            .max_by(|a, b| {
                a.price
                    .partial_cmp(&b.price)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|bid| bid.price);

        let best_ask_price = asks
            .iter()
            .min_by(|a, b| {
                a.price
                    .partial_cmp(&b.price)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|ask| ask.price);

        // Update the bids in the order book
        for bid in bids.iter() {
            let price = OrderedFloat::from(bid.price);
            // Modify or insert the bid price and quantity into the bids HashMap
            self.bids
                .entry(price)
                .and_modify(|qty| *qty = bid.qty)
                .or_insert(bid.qty);
        }
        // Remove bids with prices higher than the current bid price
        if let Some(best_bid_price) = best_bid_price {
            self.bids
                .retain(|&key, _| key <= OrderedFloat::from(best_bid_price));
        }

        for ask in asks.iter() {
            let price = OrderedFloat::from(ask.price);
            // Modify or insert the ask price and quantity into the asks HashMap
            self.asks
                .entry(price)
                .and_modify(|qty| *qty = ask.qty)
                .or_insert(ask.qty);
        }
        // Remove asks with prices lower than the current ask price
        if let Some(best_ask_price) = best_ask_price {
            self.asks
                .retain(|&key, _| key >= OrderedFloat::from(best_ask_price));
        }

        // Remove any bids with quantity equal to 0
        self.bids.retain(|_, &mut v| v != 0.0);
        // Remove any asks with quantity equal to 0
        self.asks.retain(|_, &mut v| v != 0.0);

        // Set the best bid based on the highest bid price and quantity in the order book
        self.best_bid = self
            .bids
            .iter()
            .next_back()
            .map(|(price, qty)| Self::Bid {
                price: **price,
                qty: *qty,
            })
            .unwrap_or_else(|| Self::Bid {
                price: 0.0,
                qty: 0.0,
            });
        // Set the best ask based on the lowest ask price and quantity in the order book
        self.best_ask = self
            .asks
            .iter()
            .next()
            .map(|(price, qty)| Self::Ask {
                price: **price,
                qty: *qty,
            })
            .unwrap_or_else(|| Self::Ask {
                price: 0.0,
                qty: 0.0,
            });

        // Calculate the mid price
        self.set_mid_price();
    }

    fn update(
        &mut self,
        asks: Vec<Self::Ask>,
        bids: Vec<Self::Bid>,
        timestamp: u64,
        levels: usize,
    ) {
        if timestamp <= self.last_update {
            return;
        }
        self.last_update = timestamp;

        let top_ask_threshold = self
            .asks
            .iter()
            .take(levels)
            .map(|(price, _)| **price)
            .last()
            .unwrap_or(f64::MAX);

        let top_bid_threshold = self
            .bids
            .iter()
            .rev()
            .take(levels)
            .map(|(price, _)| **price)
            .last()
            .unwrap_or(0.0);

        for ask in asks.iter() {
            let price = OrderedFloat::from(ask.price);
            if price >= OrderedFloat::from(top_ask_threshold) {
                self.asks
                    .entry(price)
                    .and_modify(|qty| *qty = ask.qty)
                    .or_insert(ask.qty);
            }
        }

        // Update bids except top levels
        for bid in bids.iter() {
            let price = OrderedFloat::from(bid.price);
            if price <= OrderedFloat::from(top_bid_threshold) {
                self.bids
                    .entry(price)
                    .and_modify(|qty| *qty = bid.qty)
                    .or_insert(bid.qty);
            }
        }

        self.asks.retain(|_, &mut v| v != 0.0);
        self.bids.retain(|_, &mut v| v != 0.0);
    }

    fn set_mid_price(&mut self) {
        self.mid_price = (self.best_ask.price + self.best_bid.price) / 2.0;
    }

    fn get_mid_price(&self) -> f64 {
        self.mid_price
    }

    fn get_depth(&self, depth: usize) -> (Vec<Self::Ask>, Vec<Self::Bid>) {
        let asks: Vec<Self::Ask> = {
            let mut ask_vec = Vec::new();
            for (p, q) in self.asks.iter().take(depth) {
                ask_vec.push(Self::Ask {
                    price: **p,
                    qty: *q,
                })
            }
            ask_vec
        };

        let bids: Vec<Self::Bid> = {
            let mut bid_vec = Vec::new();
            for (p, q) in self.bids.iter().rev().take(depth) {
                bid_vec.push(Self::Bid {
                    price: **p,
                    qty: *q,
                })
            }
            bid_vec
        };
        (asks, bids)
    }

    fn get_best_ask(&self) -> Self::Ask {
        self.best_ask.clone()
    }

    fn get_best_bid(&self) -> Self::Bid {
        self.best_bid.clone()
    }

    fn get_bba(&self) -> (Self::Ask, Self::Bid) {
        (self.best_ask.clone(), self.best_bid.clone())
    }

    fn get_spread(&self) -> f64 {
        self.best_ask.price - self.best_bid.price
    }
}

impl BinanceMarket {
    pub fn get_book_snapshot(mut self, symbols: &[String]) -> Self {
        let market: FuturesMarket = Binance::new(None, None);
        for v in symbols {
            match market.get_depth(v) {
                Ok(res) => {
                    if let Some(book) = self.books.get_mut(v) {
                        book.reset(res.asks, res.bids, res.event_time, res.last_update_id);
                    }
                }
                Err(_) => {}
            }
        }
        self
    }
}

fn build_requests(symbol: &[String]) -> Vec<String> {
    let mut request_args = vec![];

    // Agg Trades request
    let trade_req: Vec<String> = symbol
        .iter()
        .map(|sub| sub.to_lowercase())
        .map(|sub| format!("{}@aggTrade", sub))
        .collect();
    request_args.extend(trade_req);
    let best_book: Vec<String> = symbol
        .iter()
        .map(|sub| sub.to_lowercase())
        .map(|sub| format!("{}@depth20@100ms", sub))
        .collect();
    request_args.extend(best_book);
    let book: Vec<String> = symbol
        .iter()
        .map(|sub| sub.to_lowercase())
        .map(|sub| format!("{}@depth@100ms", sub))
        .collect();
    request_args.extend(book);
    let tickers: Vec<String> = symbol
        .iter()
        .map(|sub| sub.to_lowercase())
        .map(|sub| format!("{}@bookTicker", sub))
        .collect();
    request_args.extend(tickers);
    request_args
}
