use std::{
    borrow::Cow,
    collections::{BTreeMap, VecDeque},
    time::Duration,
};
use bybit::{
    account::AccountManager,
    api::Bybit,
    config::Config,
    errors::BybitError,
    general::General,
    model::{
        Ask, Bid, Category, LeverageRequest, OrderBookUpdate, Subscription, Tickers,
        WebsocketEvents,
    },
    position::PositionManager,
    trade::Trader,
    ws::Stream,
};
use ordered_float::OrderedFloat;

use crate::utils::{
    bot::LiveBot,
    localorderbook::OrderBook,
    logger::Logger,
    models::{BybitBook, BybitClient, BybitMarket, BybitPrivate},
};

use super::exchange::Exchange;

impl Exchange for BybitClient {
    type TimeOutput = Result<u64, BybitError>;
    type FeeOutput = Result<String, BybitError>;
    type LeverageOutput = Result<bool, BybitError>;
    type TraderOutput = Trader;
    type StreamOutput = BybitMarket;
    type PrivateStreamOutput = (String, BybitPrivate);
    fn init(api_key: String, api_secret: String) -> Self {
        Self {
            api_key,
            api_secret,
            logger: Logger,
            bot: LiveBot::new().unwrap(),
        }
    }

    async fn time(&self) -> Self::TimeOutput {
        let general: General = Bybit::new(None, None);
        Ok(general.get_server_time().await?.result.time_second * 1000 as u64)
    }

    async fn fees(&self, symbol: String) -> Self::FeeOutput {
        let account: AccountManager =
            Bybit::new(Some(self.api_key.clone()), Some(self.api_secret.clone()));
        match account.get_fee_rate(Category::Spot, Some(symbol)).await {
            Ok(fee) => Ok(fee.result.list[0].maker_fee_rate.clone()),
            Err(e) => Err(e),
        }
    }

    async fn set_leverage(&self, symbol: &str, leverage: u8) -> Self::LeverageOutput {
        let account: PositionManager =
            Bybit::new(Some(self.api_key.clone()), Some(self.api_secret.clone()));
        let request_format = LeverageRequest {
            category: Category::Linear,
            symbol: Cow::Borrowed(symbol),
            leverage: leverage as i8,
        };
        match account.set_leverage(request_format).await {
            Ok(_) => Ok(true),
            Err(e) => Err(e),
        }
    }

    fn trader(&self, recv_window: u16) -> Self::TraderOutput {
        let config = { Config::default().set_recv_window(recv_window) };
        let trader: Trader = Bybit::new_with_config(
            &config,
            Some(self.api_key.clone()),
            Some(self.api_key.clone()),
        );
        trader
    }

    async fn market_subscribe(
        &self,
        symbols: Vec<String>,
        sender: tokio::sync::mpsc::UnboundedSender<Self::StreamOutput>,
    ) {
        let delay = 600;
        let market_stream: Stream = Bybit::new(None, None);
        let category = Category::Linear;
        let args = build_request(&symbols);
        let mut market_data = BybitMarket::default();
        let request = Subscription::new("subscribe", args.iter().map(String::as_str).collect());

        for k in symbols {
            market_data.books.insert(k.clone(), BybitBook::new());
            market_data
                .trades
                .insert(k.clone(), VecDeque::with_capacity(5000));
            market_data.ticker.insert(k, VecDeque::with_capacity(10));
        }
        let handler = move |event| {
            match event {
                WebsocketEvents::OrderBookEvent(OrderBookUpdate {
                    topic,
                    data,
                    event_type,
                    timestamp,
                    cts,
                }) => {
                    let symbol = topic.split(".").nth(2).unwrap();
                    let event_type_str = event_type.as_str();
                    if let Some(book) = market_data.books.get_mut(symbol) {
                        market_data.timestamp = timestamp;
                        match event_type_str {
                            "snapshot" => book.reset(data.asks, data.bids, timestamp, cts),
                            "delta" => {
                                if topic == format!("orderbook.1.{}", symbol) {
                                    book.update_bba(data.asks, data.bids, timestamp, cts);
                                } else if topic == format!("orderbook.50.{}", symbol) {
                                    book.update(data.asks, data.bids, timestamp, 1);
                                } else {
                                    book.update(data.asks, data.bids, timestamp, 50);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                WebsocketEvents::TickerEvent(tick) => {
                    let symbol = tick.topic.split('.').nth(1).unwrap();
                    if let Some(ticker) = market_data.ticker.get_mut(symbol) {
                        if ticker.len() == ticker.capacity()
                            || (ticker.capacity() - ticker.len()) <= 1
                        {
                            for _ in 0..2 {
                                ticker.pop_front();
                            }
                        }
                        match tick.data {
                            Tickers::Linear(data) => ticker.push_back(data),
                            _ => unreachable!(),
                        }
                    }
                }
                WebsocketEvents::TradeEvent(data) => {
                    let symbol = data.topic.split(".").nth(2).unwrap();
                    if let Some(trades) = market_data.trades.get_mut(symbol) {
                        if trades.len() == trades.capacity()
                            || (trades.capacity() - trades.len()) <= data.data.len()
                        {
                            for _ in 0..data.data.len() {
                                trades.pop_front();
                            }
                        }
                        trades.extend(data.data);
                    }
                }

                _ => {}
            }
            let _ = sender.send(market_data.clone());
            Ok(())
        };

        loop {
            match market_stream
                .ws_subscribe(request.clone(), category, handler.clone())
                .await
            {
                Ok(_) => {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
                Err(e) => {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    println!("Error: {}", e);
                }
            }
        }
    }

    async fn private_subscribe(
        &self,
        symbol: String,
        sender: tokio::sync::mpsc::UnboundedSender<Self::PrivateStreamOutput>,
    ) -> () {
        let delay = 600;
        let user_stream: Stream = Bybit::new(
            Some(self.api_key.clone()),    // API key
            Some(self.api_secret.clone()), // Secret Key
        );
        let request_args = {
            let mut args = vec![];
            args.push("position.linear".to_string());
            args.push("execution.fast".to_string());
            args.push("order.linear".to_string());
            args.push("wallet".to_string());
            args
        };
        let mut private_data = BybitPrivate::default();
        let request = Subscription::new(
            "subscribe",
            request_args.iter().map(String::as_str).collect(),
        );
        let handler = move |event| {
            match event {
                WebsocketEvents::Wallet(data) => {
                    private_data.time = data.creation_time;
                    if private_data.wallet.len() == private_data.wallet.capacity()
                        || (private_data.wallet.capacity() - private_data.wallet.len())
                            <= data.data.len()
                    {
                        for _ in 0..data.data.len() {
                            private_data.wallet.pop_front();
                        }
                    }
                    private_data.wallet.extend(data.data);
                }
                WebsocketEvents::PositionEvent(data) => {
                    private_data.time = data.creation_time;
                    if private_data.positions.len() == private_data.positions.capacity()
                        || (private_data.positions.capacity() - private_data.positions.len())
                            <= data.data.len()
                    {
                        for _ in 0..data.data.len() {
                            private_data.positions.pop_front();
                        }
                    }
                    private_data.positions.extend(data.data);
                }
                WebsocketEvents::FastExecEvent(data) => {
                    private_data.time = data.creation_time;
                    if private_data.executions.len() == private_data.executions.capacity()
                        || (private_data.executions.capacity() - private_data.executions.len())
                            <= data.data.len()
                    {
                        for _ in 0..data.data.len() {
                            private_data.executions.pop_front();
                        }
                    }
                    private_data.executions.extend(data.data);
                }
                WebsocketEvents::OrderEvent(data) => {
                    private_data.time = data.creation_time;
                    if private_data.orders.len() == private_data.orders.capacity()
                        || (private_data.orders.capacity() - private_data.orders.len())
                            <= data.data.len()
                    {
                        for _ in 0..data.data.len() {
                            private_data.orders.pop_front();
                        }
                    }
                    private_data.orders.extend(data.data);
                }
                _ => {
                    eprintln!("Unhandled event: {:#?}", event);
                }
            }
            let _ = sender.send((symbol.clone(), private_data.clone()));
            Ok(())
        };
        loop {
            match user_stream
                .ws_priv_subscribe(request.clone(), handler.clone())
                .await
            {
                Ok(_) => {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
                Err(e) => {
                    eprintln!("Subscription error: {}", e);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }
}

impl OrderBook for BybitBook {
    type Ask = Ask;
    type Bid = Bid;

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
        // Set the best ask based on the lowest ask price and quantity in the order boo
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

fn build_request(symbol: &[String]) -> Vec<String> {
    let mut request_args = vec![];
    let book_req: Vec<String> = symbol
        .iter()
        .flat_map(|s| vec![(1, s), (50, s), (200, s)])
        .map(|(num, sym)| format!("orderbook.{}.{}", num, sym.to_uppercase()))
        .collect();
    request_args.extend(book_req);
    let tickers_req: Vec<String> = symbol
        .iter()
        .map(|sub| format!("tickers.{}", sub.to_uppercase()))
        .collect();
    request_args.extend(tickers_req);
    let trade_req: Vec<String> = symbol
        .iter()
        .map(|sub| format!("publicTrade.{}", sub.to_uppercase()))
        .collect();
    request_args.extend(trade_req);
    request_args
}
