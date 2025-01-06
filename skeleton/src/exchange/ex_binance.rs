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
    errors::Error as BinanceError,
    futures::{
        account::FuturesAccount,
        general::FuturesGeneral,
        market::FuturesMarket,
        model::{CanceledOrder, Filters::PriceFilter},
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
    models::{
        BatchAmend, BatchOrder, BinanceBook, BinanceClient, BinanceMarket, LiveOrder, SymbolInfo,
    },
    number::decay,
};

use super::exchange::Exchange;

impl Exchange for BinanceClient {
    type TimeOutput = Result<u64, Box<dyn Error>>;
    type FeeOutput = Result<f64, Box<dyn Error>>;
    type LeverageOutput = Result<bool, Box<dyn Error>>;
    type TraderOutput = FuturesAccount;

    type StreamData = BinanceMarket;
    type PrivateStreamData = ();
    type StreamOutput = ();
    type PrivateStreamOutput = ();
    type PlaceOrderOutput = Result<LiveOrder, BinanceError>;
    type AmendOrderOutput = ();
    type CancelOrderOutput = Result<CanceledOrder, BinanceError>;
    type CancelAllOutput = Result<(), BinanceError>;
    type BatchOrdersOutput = ();
    type SymbolInformationOutput = Result<SymbolInfo, BinanceError>;
    type BatchAmendsOutput = ();

    /// Initializes a new `BinanceClient` instance.
    ///
    /// # Arguments
    ///
    /// - `api_key`: Binance API key
    /// - `api_secret`: Binance API secret
    ///
    /// # Returns
    ///
    /// A new `BinanceClient` instance
    fn init(api_key: String, api_secret: String) -> Self {
        Self {
            api_key,
            api_secret,
            logger: Logger,
            bot: LiveBot::new().unwrap(),
        }
    }

    /// Gets the current server time.
    ///
    /// # Returns
    ///
    /// The current server time in milliseconds as a `Result`.
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

    /// Gets the fee tier for the given symbol.
    ///
    /// # Arguments
    ///
    /// - `symbol`: The symbol for which to get the fee tier
    ///
    /// # Returns
    ///
    /// A `Result` containing the fee tier as a u32 for the given symbol.
    ///
    /// # Notes
    ///
    /// The `symbol` argument is currently ignored, and the fee tier is always
    /// retrieved for the entire account.
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

    /// Sets the leverage for the given symbol.
    ///
    /// # Arguments
    ///
    /// - `symbol`: The symbol for which to set the leverage
    /// - `leverage`: The leverage to set
    ///
    /// # Returns
    ///
    /// A `Result` containing a boolean indicating whether the leverage was
    /// successfully set.
    ///
    /// # Notes
    ///
    /// The `symbol` argument is currently ignored, and the leverage is always
    /// retrieved for the entire account.
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

    /// Creates a new `FuturesAccount` instance with the given receive window.
    ///
    /// # Arguments
    ///
    /// - `recv_window`: The receive window in milliseconds.
    ///
    /// # Returns
    ///
    /// A new `FuturesAccount` instance.
    fn trader(&self, recv_window: u16) -> Self::TraderOutput {
        let config = { Config::default().set_recv_window(recv_window as u64) };
        let trader: FuturesAccount = Binance::new_with_config(
            Some(self.api_key.clone()),
            Some(self.api_key.clone()),
            &config,
        );
        trader
    }

    /// Places a new order on Binance Futures.
    ///
    /// # Arguments
    ///
    /// * `symbol`: The symbol of the market to place the order in.
    /// * `price`: The price to place the order at.
    /// * `qty`: The quantity of the order.
    /// * `is_buy`: Whether to place a buy or sell order.
    ///
    /// # Returns
    ///
    /// A `LiveOrder` representing the order that was placed.
    async fn place_order(
        &self,
        symbol: &str,
        price: f64,
        qty: f64,
        is_buy: bool,
    ) -> Self::PlaceOrderOutput {
        let trader = self.trader(2500);
        let new_symbol = symbol.to_string();
        let order = task::spawn_blocking(move || {
            if is_buy {
                match trader.limit_buy(
                    new_symbol,
                    qty,
                    price,
                    binance::futures::account::TimeInForce::GTC,
                ) {
                    Ok(res) => Ok(LiveOrder::new(
                        res.order_id.to_string(),
                        res.avg_price,
                        res.orig_qty,
                    )),
                    Err(e) => Err(e),
                }
            } else {
                match trader.limit_sell(
                    new_symbol,
                    qty,
                    price,
                    binance::futures::account::TimeInForce::GTC,
                ) {
                    Ok(res) => Ok(LiveOrder::new(
                        res.order_id.to_string(),
                        res.avg_price,
                        res.orig_qty,
                    )),
                    Err(e) => Err(e),
                }
            }
        })
        .await
        .unwrap();
        order
    }
    async fn amend_order(
        &self,
        order_id: &str,
        price: f64,
        qty: f64,
        symbol: &str,
    ) -> Self::AmendOrderOutput {
        unimplemented!();
    }
    async fn cancel_order(&self, order_id: &str, symbol: &str) -> Self::CancelOrderOutput {
        let trader = self.trader(2500);
        let (new_id, new_symbol) = (order_id.parse::<u64>().unwrap(), symbol.to_string());
        let cancel = task::spawn_blocking(move || match trader.cancel_order(new_symbol, new_id) {
            Ok(res) => Ok(res),
            Err(e) => Err(e),
        })
        .await
        .unwrap();
        cancel
    }
    async fn cancel_all(&self, symbol: &str) -> Self::CancelAllOutput {
        let trader = self.trader(2500);
        let new_symbol = symbol.to_string();
        let cancel =
            task::spawn_blocking(move || match trader.cancel_all_open_orders(new_symbol) {
                Ok(res) => Ok(res),
                Err(e) => Err(e),
            })
            .await
            .unwrap();
        cancel
    }
    async fn batch_orders(&self, orders: Vec<BatchOrder>) -> Self::BatchOrdersOutput {}
    async fn batch_amends(&self, orders: Vec<BatchAmend>) -> Self::BatchAmendsOutput {}

    /// Get the symbol information for a given symbol.
    ///
    /// This function retrieves the symbol information for a given symbol, and returns a `SymbolInfo` object.
    /// The `SymbolInfo` object contains the following fields:
    ///
    /// * `tick_size`: The tick size of the symbol.
    /// * `lot_size`: The minimum lot size of the symbol.
    /// * `min_notional`: The minimum notional value of the symbol.
    /// * `min_qty`: The minimum quantity of the symbol.
    /// * `post_only_max`: The maximum post-only quantity of the symbol.
    ///
    /// If the request fails, the function will panic with the error message.
    async fn get_symbol_info(&self, symbol: &str) -> Self::SymbolInformationOutput {
        let market_data: FuturesGeneral = Binance::new(None, None);
        let new_symbol = symbol.to_string();
        let info = task::spawn_blocking(move || match market_data.get_symbol_info(new_symbol) {
            Ok(res) => {
                let price_filter = match &res.filters[0] {
                    PriceFilter { tick_size, .. } => tick_size.parse().unwrap_or(0.0),
                    _ => 0.0,
                };
                let final_data = SymbolInfo {
                    tick_size: price_filter,
                    lot_size: match &res.filters[1] {
                        binance::model::Filters::LotSize { step_size, .. } => {
                            step_size.parse().unwrap_or(0.0)
                        }
                        _ => 0.0,
                    },
                    min_notional: match &res.filters[5] {
                        binance::model::Filters::MinNotional { notional, .. } => {
                            notional.clone().unwrap().parse().unwrap_or(0.0)
                        }
                        _ => 0.0,
                    },
                    min_qty: 0.0,
                    post_only_max: match &res.filters[1] {
                        binance::model::Filters::LotSize { max_qty, .. } => {
                            max_qty.parse().unwrap_or(0.0)
                        }
                        _ => 0.0,
                    },
                };
                Ok(final_data)
            }
            Err(e) => Err(e),
        })
        .await;
        match info {
            Ok(res) => Ok(res?),
            Err(e) => {
                let error_message = format!("Order failed. Error: {}", e);
                let _ = self
                    .bot
                    .send_message(&self.logger.error(&error_message))
                    .await;
                panic!("Order failed. Error: {}", e)
            }
        }
    }
    /// Subscribes to Binance futures market data for the given symbols and sends
    /// it over the given sender channel.
    ///
    /// The data is sent as a `BinanceMarket` struct, which contains the order
    /// book, last trades, and ticker data.
    ///
    /// This function blocks until the subscription is stopped or an error occurs.
    ///
    /// The `sender` channel must be kept open for the duration of the
    /// subscription, as the data is sent over it.
    ///
    /// The `symbols` parameter is a vector of strings representing the symbols
    /// to subscribe to. The symbols must be in the format of `<symbol>@<exchange>`.
    ///
    /// The `sender` parameter is an unbounded sender channel that will receive
    /// the market data.
    ///
    /// The function returns an empty tuple.
    async fn market_subscribe(
        &self,
        symbols: Vec<String>,
        sender: tokio::sync::mpsc::UnboundedSender<Self::StreamData>,
    ) -> () {
        let delay = 600;
        let keep_streaming = AtomicBool::new(true);
        let request = build_requests(&symbols);
        let mut market_data = BinanceMarket::default();
        for k in symbols.clone() {
            market_data.books.insert(k.clone(), BinanceBook::new());
            market_data
                .trades
                .insert(k.clone(), VecDeque::with_capacity(1000));
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
        let _ = task::spawn_blocking(move || {
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
        })
        .await;
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

    /// Creates a new `BinanceBook` with all fields initialized to zero.
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
            tick_size: 0.0,
            lot_size: 0.0,
            min_notional: 0.0,
            min_qty: 0.0,
            post_only_max: 0.0,
        }
    }

    /// Resets the order book to a new state.
    ///
    /// # Arguments
    ///
    /// * `asks`: The new asks for the order book.
    /// * `bids`: The new bids for the order book.
    /// * `timestamp`: The new timestamp for the order book.
    /// * `sequence`: The new sequence number for the order book.
    ///
    /// # Effects
    ///
    /// * Sets `last_update` to `timestamp`.
    /// * Sets `sequence` to `sequence`.
    /// * Replaces the current asks and bids with `asks` and `bids`.
    /// * Removes any asks or bids with a quantity of 0.
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

    /// Updates the order book with new data.
    ///
    /// # Arguments
    ///
    /// * `asks`: The new asks for the order book.
    /// * `bids`: The new bids for the order book.
    /// * `timestamp`: The new timestamp for the order book.
    /// * `sequence`: The new sequence number for the order book.
    ///
    /// # Effects
    ///
    /// * Sets `last_update` to `timestamp`.
    /// * Sets `sequence` to `sequence`.
    /// * Updates the bids in the order book.
    /// * Removes any bids or asks with a quantity of 0.
    /// * Sets the best bid and best ask based on the highest bid price and lowest ask price in the order book.
    /// * Calculates the mid price.
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

    /// Updates the order book with the given asks and bids at the given timestamp.
    ///
    /// This function will not update the order book if the given timestamp is less than or equal
    /// to the last update timestamp.
    ///
    /// The update is done in the following way:
    ///  - The asks and bids are iterated over and only the ones with a price higher than or equal
    ///    to the top ask threshold and lower than or equal to the top bid threshold are considered.
    ///  - The quantity of the asks and bids is updated in the order book.
    ///  - Any asks or bids with a quantity of 0 are removed from the order book.
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

    /// Sets the mid price of the order book.
    ///
    /// The mid price is calculated as the average of the best ask and best bid prices.
    ///
    /// This function is used to update the mid price when the order book is updated.
    
    fn set_mid_price(&mut self) {
        self.mid_price = (self.best_ask.price + self.best_bid.price) / 2.0;
    }

    /// Returns the mid price of the order book.
    ///
    /// The mid price is calculated as the average of the best ask and best bid prices.
    fn get_mid_price(&self) -> f64 {
        self.mid_price
    }

    /// Returns a tuple of vectors of the top N asks and bids, respectively.
    ///
    /// The asks are returned in ascending order of price, and the bids in descending order.
    ///
    /// # Arguments
    ///
    /// * `depth`: The number of asks and bids to return.
    ///
    /// # Returns
    ///
    /// A tuple of vectors of the top N asks and bids, respectively.
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

    /// Returns a clone of the best ask in the order book.
    ///
    /// The best ask is the highest price ask in the order book.
    fn get_best_ask(&self) -> Self::Ask {
        self.best_ask.clone()
    }

    /// Returns a clone of the best bid in the order book.
    ///
    /// The best bid is the lowest price bid in the order book.
    fn get_best_bid(&self) -> Self::Bid {
        self.best_bid.clone()
    }

    /// Returns a tuple of the best ask and best bid in the order book.
    ///
    /// The best ask is the highest price ask in the order book, and the best bid is the lowest price bid
    /// in the order book.
    fn get_bba(&self) -> (Self::Ask, Self::Bid) {
        (self.best_ask.clone(), self.best_bid.clone())
    }

    /// Returns the spread between the best ask and best bid in the order book.
    ///
    /// The spread is calculated as the difference between the best ask price and the best bid price.
    ///
    /// # Returns
    ///
    /// The spread between the best ask and best bid in the order book.
    fn get_spread(&self) -> f64 {
        self.best_ask.price - self.best_bid.price
    }

    /// Returns the spread between the best ask and best bid in the order book, in ticks.
    ///
    /// The spread is calculated as the difference between the best ask price and the best bid price,
    /// divided by the tick size.
    fn get_spread_in_ticks(&self) -> f64 {
        (self.best_ask.price - self.best_bid.price) / self.tick_size
    }
    /// Returns the lot size of the order book.
    ///
    /// The lot size is the minimum quantity that can be traded in the symbol.
    fn get_lot_size(&self) -> f64 {
        self.lot_size
    }
    /// Returns the minimum notional value for the symbol.
    ///
    /// The minimum notional value is the minimum value that can be traded in the symbol.
    fn get_min_notional(&self) -> f64 {
        self.min_notional
    }
    /// Returns the post-only maximum quantity for the symbol.
    ///
    /// The post-only maximum quantity is the maximum quantity that can be posted to the order book
    /// using a post-only order.
    fn get_post_only_max_qty(&self) -> f64 {
        self.post_only_max
    }
    /// Returns the tick size of the symbol.
    ///
    /// The tick size is the minimum price increment for the symbol.
    fn get_tick_size(&self) -> f64 {
        self.tick_size
    }
    /// Returns the minimum quantity for the symbol.
    ///
    /// The minimum quantity is the minimum amount that can be traded in the symbol.
    fn min_qty(&self) -> f64 {
        self.min_qty
    }
    /// Calculates the weighted mid price of the order book.
    ///
    /// The weighted mid price is a measure of the imbalance between the bid and ask sides of the
    /// order book. It is calculated as the weighted average of the best bid and best ask prices,
    /// where the weights are the quantities at the best bid and best ask. The `depth` parameter can
    /// be used to specify the depth of the order book to use when calculating the weighted mid
    /// price. If `depth` is `None`, the best bid and best ask quantities are used.
    ///
    /// The weighted mid price is calculated as follows:
    ///
    /// * Let `weighted_bid_qty` be the weighted bid quantity, calculated by summing the products
    ///   of the prices and quantities of the bids in the order book, starting from the best bid
    ///   and going down in price.
    /// * Let `weighted_ask_qty` be the weighted ask quantity, calculated by summing the products
    ///   of the prices and quantities of the asks in the order book, starting from the best ask
    ///   and going up in price.
    /// * Let `total_qty` be the sum of `weighted_bid_qty` and `weighted_ask_qty`.
    /// * The weighted mid price is then calculated as `(best_bid_price * (1.0 - imbalance)) +
    ///   (best_ask_price * imbalance)`, where `imbalance` is `weighted_bid_qty / total_qty`.
    /// * If `imbalance` is 0.0, the weighted mid price is set to the mid price of the order book.
    ///
    fn get_wmid(&self, depth: Option<usize>) -> f64 {
        let imbalance = {
            let (weighted_bid_qty, weighted_ask_qty) = if let Some(depth) = depth {
                // Calculate the weighted bid quantity using the specified depth.
                (
                    self.calculate_weighted_bid(depth, Some(0.5)),
                    self.calculate_weighted_ask(depth, Some(0.5)),
                )
            } else {
                (self.best_bid.qty, self.best_ask.qty)
            };

            let total_qty = weighted_bid_qty + weighted_ask_qty;
            weighted_bid_qty / total_qty
        };
        if imbalance != 0.0 {
            (self.best_bid.price * (1.0 - imbalance)) + (self.best_ask.price * imbalance)
        } else {
            // Otherwise, return the mid_price of the LocalBook.
            self.mid_price
        }
    }
    /// Returns the effective spread for a given side of the order book.
    ///
    /// The effective spread is the difference between the best bid/ask price and the mid price of
    /// the order book. If `is_buy` is `true`, the effective spread is calculated as the difference
    /// between the best bid price and the mid price. If `is_buy` is `false`, the effective spread is
    /// calculated as the difference between the mid price and the best ask price.
    fn effective_spread(&self, is_buy: bool) -> f64 {
        if is_buy {
            self.best_bid.price - self.mid_price
        } else {
            self.mid_price - self.best_ask.price
        }
    }
    /// Calculates the microprice of the order book.
    ///
    /// The microprice is a measure of the order book imbalance, calculated as the weighted average
    /// of the best bid and best ask prices. The weights are the quantities at the best bid and best
    /// ask. The `depth` parameter can be used to specify the depth of the order book to use when
    /// calculating the microprice. If `depth` is `None`, the best bid and best ask quantities are
    /// used. If the total quantity is 0, the mid price of the order book is returned.
    ///
    /// The microprice is calculated as follows:
    ///
    /// * Let `bid_qty` be the weighted bid quantity, calculated by summing the products of the
    ///   prices and quantities of the bids in the order book, starting from the best bid and going
    ///   down in price.
    /// * Let `ask_qty` be the weighted ask quantity, calculated by summing the products of the
    ///   prices and quantities of the asks in the order book, starting from the best ask and going
    ///   up in price.
    /// * Let `total_qty` be the sum of `bid_qty` and `ask_qty`.
    /// * The microprice is then calculated as `(best_ask_price * qty_ratio) + (best_bid_price *
    ///   (1.0 - qty_ratio))`, where `qty_ratio` is `bid_qty / total_qty`.
    ///
    /// # Returns
    ///
    /// The microprice of the order book.
    fn get_microprice(&self, depth: Option<usize>) -> f64 {
        let (bid_qty, ask_qty) = match depth {
            Some(depth) => (
                self.calculate_weighted_bid(depth, Some(0.5)),
                self.calculate_weighted_ask(depth, Some(0.5)),
            ),
            None => (self.best_bid.qty, self.best_ask.qty),
        };

        let total_qty = bid_qty + ask_qty;
        if total_qty == 0.0 {
            return self.mid_price;
        }

        let qty_ratio = bid_qty / total_qty;
        (self.best_ask.price * qty_ratio) + (self.best_bid.price * (1.0 - qty_ratio))
    }

    /// Calculates the price impact of the difference between the order book and the old order book.
    ///
    /// The price impact is a measure of the change in the order book imbalance between the old and
    /// new order books. The price impact is calculated as the sum of the bid and ask impacts.
    ///
    /// The bid impact is calculated as the difference in the volume at the best bid between the old
    /// and new order books. If the best bid price has increased or the volume at the best bid has
    /// increased, the bid impact is positive. If the best bid price has decreased or the volume at
    /// the best bid has decreased, the bid impact is negative.
    ///
    /// The ask impact is calculated as the difference in the volume at the best ask between the old
    /// and new order books. If the best ask price has decreased or the volume at the best ask has
    /// increased, the ask impact is positive. If the best ask price has increased or the volume at
    /// the best ask has decreased, the ask impact is negative.
    ///
    /// The `depth` parameter can be used to specify the depth of the order book to use when
    /// calculating the price impact. If `depth` is `None`, the best bid and best ask quantities are
    /// used.
    fn price_impact(&self, old_book: &Self, depth: Option<usize>) -> f64 {
        // Calculate the volume at the bid and ask offsets
        let (mut old_bid_vol, mut curr_bid_vol, old_bid_price, curr_bid_price) = (
            old_book.best_bid.qty,
            self.best_bid.qty,
            old_book.best_bid.price,
            self.best_bid.price,
        );
        let (mut old_ask_vol, mut curr_ask_vol, old_ask_price, curr_ask_price) = (
            old_book.best_ask.qty,
            self.best_ask.qty,
            old_book.best_ask.price,
            self.best_ask.price,
        );

        // Calculate the volume at the depth, if provided
        if let Some(depth) = depth {
            old_bid_vol = 0.0;
            curr_bid_vol = 0.0;
            old_ask_vol = 0.0;
            curr_ask_vol = 0.0;

            // Iterate over the depth asks and bids in the old and new order books
            for (_, (_, qty)) in old_book.asks.iter().take(depth).enumerate() {
                old_ask_vol += qty;
            }
            for (_, (_, qty)) in self.asks.iter().take(depth).enumerate() {
                curr_ask_vol += qty;
            }
            for (_, (_, qty)) in old_book.bids.iter().rev().take(depth).enumerate() {
                old_bid_vol += qty;
            }
            for (_, (_, qty)) in self.bids.iter().rev().take(depth).enumerate() {
                curr_bid_vol += qty;
            }
        }

        // Calculate the volume at the bid and ask offsets
        let bid_impact = if curr_bid_price > old_bid_price || curr_bid_vol > old_bid_vol {
            curr_bid_vol - old_bid_vol
        } else if curr_bid_price < old_bid_price || curr_bid_vol < old_bid_vol {
            curr_bid_vol - old_bid_vol
        } else {
            0.0
        };
        let ask_impact = if curr_ask_price < old_ask_price || curr_ask_vol > old_ask_vol {
            curr_ask_vol - old_ask_vol
        } else if curr_ask_price > old_ask_price || curr_ask_vol < old_ask_vol {
            curr_ask_vol - old_ask_vol
        } else {
            0.0
        };

        // Return the sum of the bid and ask impacts
        bid_impact + ask_impact
    }

    /// Calculates the imbalance ratio of the order book.
    ///
    /// The imbalance ratio is a measure of the imbalance between the bid and ask sides of the
    /// order book. It is calculated as the difference between the weighted bid and ask quantities,
    /// divided by the sum of the weighted bid and ask quantities. The `depth` parameter can be
    /// used to specify the depth of the order book to use when calculating the imbalance ratio.
    /// If `depth` is `None`, the best bid and best ask quantities are used.
    ///
    /// The imbalance ratio is a value between -1.0 and 1.0. A positive imbalance ratio indicates
    /// that the bid side is stronger, while a negative imbalance ratio indicates that the ask side
    /// is stronger. An imbalance ratio of 0.0 indicates that the order book is balanced.
    ///
    /// # Returns
    ///
    /// The imbalance ratio of the order book.
    fn imbalance_ratio(&self, depth: Option<usize>) -> f64 {
        // Initialize the weighted bid and ask quantities to the quantities of the best bid and ask.
        let (weighted_bid_qty, weighted_ask_qty) = if let Some(depth) = depth {
            // Calculate the weighted bid quantity using the specified depth.
            (
                self.calculate_weighted_bid(depth, Some(0.5)),
                self.calculate_weighted_ask(depth, Some(0.5)),
            )
        } else {
            (self.best_bid.qty, self.best_ask.qty)
        };

        // Calculate the difference between the weighted bid and ask quantities.
        let diff = weighted_bid_qty - weighted_ask_qty;
        // Calculate the sum of the weighted bid and ask quantities.
        let sum = weighted_bid_qty + weighted_ask_qty;
        // Calculate the imbalance ratio by dividing the difference by the sum.
        let ratio = diff / sum;

        // Return the imbalance ratio, checking for NaN and out-of-range values.
        match ratio {
            x if x.is_nan() => 0.0, // If NaN, return 0.
            x if x > 0.20 => x,     // If positive and greater than 0.20, return the ratio.
            x if x < -0.20 => x,    // If negative and less than -0.20, return the ratio.
            _ => 0.0,               // Otherwise, return 0.
        }
    }

    /// Calculates the order flow imbalance of the order book.
    ///
    /// The order flow imbalance is a measure of the change in the order book imbalance between
    /// the old and new order books. The order flow imbalance is calculated as the sum of the bid
    /// and ask order flow imbalances.
    ///
    /// The bid order flow imbalance is calculated as the difference in the volume at the best
    /// bid between the old and new order books. If the best bid price has increased or the volume
    /// at the best bid has increased, the bid order flow imbalance is positive. If the best bid
    /// price has decreased or the volume at the best bid has decreased, the bid order flow
    /// imbalance is negative.
    ///
    /// The ask order flow imbalance is calculated as the difference in the volume at the best
    /// ask between the old and new order books. If the best ask price has decreased or the volume
    /// at the best ask has increased, the ask order flow imbalance is positive. If the best ask
    /// price has increased or the volume at the best ask has decreased, the ask order flow
    /// imbalance is negative.
    ///
    /// The `depth` parameter can be used to specify the depth of the order book to use when
    /// calculating the order flow imbalance. If `depth` is `None`, the best bid and best ask
    /// quantities are used.
    ///
    /// # Returns
    ///
    /// The order flow imbalance of the order book.
        fn ofi(&self, old_book: &Self, depth: Option<usize>) -> f64 {
        let bid_ofi = {
            if self.best_bid.price > old_book.best_bid.price {
                if let Some(depth) = depth {
                    let weighted_bid = self.calculate_weighted_bid(depth, Some(0.5));
                    weighted_bid
                } else {
                    self.best_bid.qty
                }
            } else if self.best_bid.price == old_book.best_bid.price {
                if let Some(depth) = depth {
                    let weighted_bid = self.calculate_weighted_bid(depth, Some(0.5));
                    let prev_weighted_bid = old_book.calculate_weighted_bid(depth, Some(0.5));
                    weighted_bid - prev_weighted_bid
                } else {
                    self.best_bid.qty - old_book.best_bid.qty
                }
            } else {
                if let Some(depth) = depth {
                    let weighted_bid = self.calculate_weighted_bid(depth, Some(0.5));
                    -weighted_bid
                } else {
                    -self.best_bid.qty
                }
            }
        };
        let ask_ofi = {
            if self.best_ask.price < old_book.best_ask.price {
                if let Some(depth) = depth {
                    let weighted_ask = self.calculate_weighted_ask(depth, Some(0.5));
                    -weighted_ask
                } else {
                    -self.best_ask.qty
                }
            } else if self.best_ask.price == old_book.best_ask.price {
                if let Some(depth) = depth {
                    let weighted_ask = self.calculate_weighted_ask(depth, Some(0.5));
                    let prev_weighted_ask = old_book.calculate_weighted_ask(depth, Some(0.5));
                    prev_weighted_ask - weighted_ask
                } else {
                    old_book.best_ask.qty - self.best_ask.qty
                }
            } else {
                if let Some(depth) = depth {
                    let weighted_ask = self.calculate_weighted_ask(depth, Some(0.5));
                    weighted_ask
                } else {
                    self.best_ask.qty
                }
            }
        };
        let ofi = ask_ofi + bid_ofi;

        ofi
    }

    /// Calculates the volume imbalance of the order book.
    ///
    /// The volume imbalance is a measure of the difference in the volume at the best bid and
    /// best ask between the old and new order books. If the best bid price has increased or the
    /// volume at the best bid has increased, the bid volume imbalance is positive. If the best
    /// bid price has decreased or the volume at the best bid has decreased, the bid volume
    /// imbalance is negative.
    ///
    /// If the best ask price has decreased or the volume at the best ask has increased, the ask
    /// volume imbalance is positive. If the best ask price has increased or the volume at the
    /// best ask has decreased, the ask volume imbalance is negative.
    ///
    /// The `depth` parameter can be used to specify the depth of the order book to use when
    /// calculating the volume imbalance. If `depth` is `None`, the best bid and best ask
    /// quantities are used.
    ///
    /// # Returns
    ///
    /// The volume imbalance of the order book.
    fn voi(&self, old_book: &Self, depth: Option<usize>) -> f64 {
        // Calculate the volume at the bid side
        let bid_v = match self.best_bid.price {
            x if x < old_book.best_bid.price => 0.0,
            x if x == old_book.best_bid.price => {
                if let Some(depth) = depth {
                    let curr_bid_qty = self.calculate_weighted_bid(depth, Some(0.5));
                    let prev_bid_qty = old_book.calculate_weighted_bid(depth, Some(0.5));
                    curr_bid_qty - prev_bid_qty
                } else {
                    self.best_bid.qty - old_book.best_bid.qty
                }
            }
            x if x > old_book.best_bid.price => {
                if let Some(depth) = depth {
                    let curr_bid = self.calculate_weighted_bid(depth, Some(0.5));
                    curr_bid
                } else {
                    self.best_bid.qty
                }
            }
            _ => 0.0,
        };

        // Calculate the volume at the ask side
        let ask_v = match self.best_ask.price {
            x if x < old_book.best_ask.price => {
                if let Some(depth) = depth {
                    let curr_ask = self.calculate_weighted_ask(depth, Some(0.5));
                    curr_ask
                } else {
                    self.best_ask.qty
                }
            }
            x if x == old_book.best_ask.price => {
                if let Some(depth) = depth {
                    let curr_ask_qty = self.calculate_weighted_ask(depth, Some(0.5));
                    let prev_ask_qty = old_book.calculate_weighted_ask(depth, Some(0.5));
                    curr_ask_qty - prev_ask_qty
                } else {
                    self.best_ask.qty - old_book.best_ask.qty
                }
            }
            x if x > old_book.best_ask.price => 0.0,
            _ => 0.0,
        };

        // Calculate the volume at the offset
        let diff = bid_v - ask_v;
        diff
    }

    /// Calculates the weighted ask quantity of the order book.
    ///
    /// The weighted ask quantity is calculated by summing the products of the prices and
    /// quantities of the asks in the order book, starting from the best ask and going up in
    /// price. The `depth` parameter can be used to specify the depth of the order book to use
    /// when calculating the weighted ask quantity. If `depth` is `None`, the best ask quantity
    /// is used.
    ///
    /// The `decay_rate` parameter can be used to specify the decay rate for the weighted ask
    /// quantity. If `decay_rate` is `None`, no decay is applied. Otherwise, the decay rate is
    /// applied to each ask based on its position in the order book. The decay rate is calculated
    /// as `decay_rate ^ (position / depth)`.
    ///
    /// # Returns
    ///
    /// The weighted ask quantity of the order book.
    fn calculate_weighted_ask(&self, depth: usize, decay_rate: Option<f64>) -> f64 {
        self.asks
            .iter()
            .take(depth)
            .enumerate()
            .map(|(i, (_, qty))| (decay(i as f64, decay_rate) * qty) as f64)
            .sum::<f64>()
    }

    /// Calculates the weighted bid quantity of the order book.
    ///
    /// The weighted bid quantity is calculated by summing the products of the prices and
    /// quantities of the bids in the order book, starting from the best bid and going down in
    /// price. The `depth` parameter can be used to specify the depth of the order book to use
    /// when calculating the weighted bid quantity. If `depth` is `None`, the best bid quantity
    /// is used.
    ///
    /// The `decay_rate` parameter can be used to specify the decay rate for the weighted bid
    /// quantity. If `decay_rate` is `None`, no decay is applied. Otherwise, the decay rate is
    /// applied to each bid based on its position in the order book. The decay rate is calculated
    /// as `decay_rate ^ (position / depth)`.
    ///
    /// # Returns
    ///
    /// The weighted bid quantity of the order book.
    fn calculate_weighted_bid(&self, depth: usize, decay_rate: Option<f64>) -> f64 {
        self.bids
            .iter()
            .rev()
            .take(depth)
            .enumerate()
            .map(|(i, (_, qty))| (decay(i as f64, decay_rate) * qty) as f64)
            .sum::<f64>()
    }
}

impl BinanceMarket {
    /// Gets a snapshot of the order books for the given symbols and updates the order books
    /// in the `BinanceMarket` instance.
    ///
    /// This function is useful for getting an initial snapshot of the order books when the
    /// bot is started.
    ///
    /// # Arguments
    ///
    /// * `symbols` - The symbols to get the order books for.
    ///
    /// # Returns
    ///
    /// The updated `BinanceMarket` instance.
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

    /// Builds a list of Binance streams to subscribe to.
    ///
    /// This function takes a slice of strings representing the symbols to subscribe to and
    /// returns a vector of strings representing the Binance streams to subscribe to. The
    /// streams are of the form `<symbol>@<stream>`, where `<symbol>` is the symbol and
    /// `<stream>` is the stream name. The streams are:
    ///
    ///  - `aggTrade`: The aggregated trade stream.
    ///  - `depth20@100ms`: The order book stream with 20 levels of depth, updated every 100ms.
    ///  - `depth@100ms`: The order book stream with all levels of depth, updated every 100ms.
    ///  - `bookTicker`: The order book ticker stream.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The symbols to subscribe to.
    ///
    /// # Returns
    ///
    /// A vector of strings representing the Binance streams to subscribe to.
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
