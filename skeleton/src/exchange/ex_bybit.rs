use bybit::{
    account::AccountManager,
    api::Bybit,
    config::Config,
    errors::BybitError,
    general::General,
    market::MarketData,
    model::{
        AmendOrderRequest, Ask, Bid, CancelOrderRequest, CancelallRequest, Category,
        InstrumentRequest, LeverageRequest, OrderBookUpdate, OrderStatus, Subscription, Tickers,
        WebsocketEvents,
    },
    position::PositionManager,
    trade::Trader,
    ws::Stream,
};
use ordered_float::OrderedFloat;
use std::{
    borrow::Cow,
    collections::{BTreeMap, VecDeque},
    time::Duration,
};

use crate::utils::{
    bot::LiveBot,
    localorderbook::OrderBook,
    logger::Logger,
    models::{
        BatchAmend, BatchOrder, BybitBook, BybitClient, BybitMarket, BybitPrivate, IntoReq,
        LiveOrder, SymbolInfo,
    },
    number::decay,
};

use super::exchange::Exchange;

impl Exchange for BybitClient {
    type TimeOutput = Result<u64, BybitError>;
    type FeeOutput = Result<String, BybitError>;
    type LeverageOutput = Result<bool, BybitError>;
    type TraderOutput = Trader;
    type StreamData = BybitMarket;
    type PrivateStreamData = (String, BybitPrivate);
    type StreamOutput = ();
    type PrivateStreamOutput = ();
    type PlaceOrderOutput = Result<LiveOrder, BybitError>;
    type AmendOrderOutput = Result<LiveOrder, BybitError>;
    type CancelOrderOutput = Result<OrderStatus, BybitError>;
    type CancelAllOutput = Result<Vec<OrderStatus>, BybitError>;
    type BatchOrdersOutput = Result<Vec<Vec<LiveOrder>>, BybitError>;
    type BatchAmendsOutput = Result<Vec<LiveOrder>, BybitError>;
    type SymbolInformationOutput = Result<SymbolInfo, BybitError>;
    /// Initializes a new `BybitClient` instance.
    ///
    /// # Arguments
    ///
    /// - `api_key`: Bybit API key
    /// - `api_secret`: Bybit API secret
    ///
    /// # Returns
    ///
    /// A new `BybitClient` instance
    fn init(api_key: String, api_secret: String) -> Self {
        Self {
            api_key,
            api_secret,
            logger: Logger,
            bot: LiveBot::new().unwrap(),
        }
    }

    /// Gets the current server time in milliseconds.
    ///
    /// # Returns
    ///
    /// A `Result` containing the current server time in milliseconds as a `u64` if successful, else an error.
    async fn time(&self) -> Self::TimeOutput {
        let general: General = Bybit::new(None, None);
        Ok(general.get_server_time().await?.result.time_second * 1000 as u64)
    }

    /// Gets the fee tier for the given symbol.
    ///
    /// # Arguments
    ///
    /// - `symbol`: The symbol for which to get the fee tier
    ///
    /// # Returns
    ///
    /// A `Result` containing the fee tier as a `String` for the given symbol.
    ///
    /// # Notes
    ///
    /// The `symbol` argument is currently ignored, and the fee tier is always
    /// retrieved for the entire account.
    async fn fees(&self, symbol: String) -> Self::FeeOutput {
        let account: AccountManager =
            Bybit::new(Some(self.api_key.clone()), Some(self.api_secret.clone()));
        match account.get_fee_rate(Category::Spot, Some(symbol)).await {
            Ok(fee) => Ok(fee.result.list[0].maker_fee_rate.clone()),
            Err(e) => Err(e),
        }
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
    async fn set_leverage(&self, symbol: &str, leverage: u8) -> Self::LeverageOutput {
        let account: PositionManager =
            Bybit::new(Some(self.api_key.clone()), Some(self.api_secret.clone()));
        let request_format = LeverageRequest {
            category: Category::Linear,
            symbol: Cow::Borrowed(symbol),
            leverage: leverage as i8,
        };
        match account.set_leverage(request_format).await {
            Ok(_) => {
                let success_message = format!("Set leverage for {} to {}", symbol, leverage);
                let _ = self.bot.send_message(&success_message).await;
                Ok(true)
            }
            Err(e) => Err(e),
        }
    }

    /// Creates a new `Trader` instance with the given receive window.
    ///
    /// # Arguments
    ///
    /// - `recv_window`: The receive window in milliseconds.
    ///
    /// # Returns
    ///
    /// A new `Trader` instance.
    fn trader(&self, recv_window: u16) -> Self::TraderOutput {
        let config = { Config::default().set_recv_window(recv_window) };
        let trader: Trader = Bybit::new_with_config(
            &config,
            Some(self.api_key.clone()),
            Some(self.api_key.clone()),
        );
        trader
    }

    /// Places a new order on Bybit.
    ///
    /// # Arguments
    ///
    /// - `symbol`: The symbol of the market to place the order in.
    /// - `price`: The price to place the order at.
    /// - `qty`: The quantity of the order.
    /// - `is_buy`: Whether to place a buy or sell order.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `LiveOrder` representing the order that was placed.
    ///
    /// # Notes
    ///
    /// The `price` is the limit price, and the `qty` is the quantity of the order.
    /// The `is_buy` argument is used to determine whether to place a buy or sell
    /// order. If `is_buy` is `true`, a buy order is placed. If `is_buy` is `false`,
    /// a sell order is placed.
    async fn place_order(
        &self,
        symbol: &str,
        price: f64,
        qty: f64,
        is_buy: bool,
    ) -> Self::PlaceOrderOutput {
        let trader = self.trader(2500);
        match trader
            .place_futures_limit_order(
                bybit::model::Category::Linear,
                symbol,
                if is_buy {
                    bybit::model::Side::Buy
                } else {
                    bybit::model::Side::Sell
                },
                qty,
                price,
                if is_buy { 1 } else { 2 },
            )
            .await
        {
            Ok(res) => {
                let order_message = format!(
                    "Order placed. Symbol: {}, Price: {}, Quantity: {}, Side: {} Order ID: {}",
                    symbol,
                    price,
                    qty,
                    if is_buy { "Buy" } else { "Sell" },
                    res.result.order_id
                );
                let _ = self
                    .bot
                    .send_message(&self.logger.success(&order_message))
                    .await;
                Ok(LiveOrder::new(res.result.order_id, price, qty))
            }
            Err(e) => Err(e),
        }
    }

    /// Amends an existing order on Bybit.
    ///
    /// # Arguments
    ///
    /// - `order_id`: The ID of the order to amend.
    /// - `price`: The new price to place the order at.
    /// - `qty`: The new quantity of the order.
    /// - `symbol`: The symbol of the market to amend the order in.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `LiveOrder` representing the amended order.
    ///
    /// # Notes
    ///
    /// The `order_id` is the ID of the order to amend. The `price` and `qty` are
    /// the new price and quantity to place the order at. The `symbol` is the symbol
    /// of the market to amend the order in.
    async fn amend_order(
        &self,
        order_id: &str,
        price: f64,
        qty: f64,
        symbol: &str,
    ) -> Self::AmendOrderOutput {
        let trader = self.trader(2500);
        let request = AmendOrderRequest {
            category: Category::Linear,
            order_id: Some(Cow::Borrowed(order_id)),
            symbol: Cow::Borrowed(symbol),
            qty,
            price: Some(price),
            ..Default::default()
        };
        match trader.amend_order(request).await {
            Ok(res) => {
                let order_message = format!(
                    "Order amended. Symbol: {}, Order ID: {}, Price: {}, Quantity: {}",
                    symbol, order_id, price, qty
                );
                let _ = self
                    .bot
                    .send_message(&self.logger.success(&order_message))
                    .await;
                Ok(LiveOrder::new(res.result.order_id, price, qty))
            }
            Err(e) => Err(e),
        }
    }

    /// Cancels an existing order on Bybit.
    ///
    /// # Arguments
    ///
    /// - `order_id`: The ID of the order to cancel.
    /// - `symbol`: The symbol of the market to cancel the order in.
    ///
    /// # Returns
    ///
    /// A `Result` containing the result of the cancellation.
    ///
    /// # Notes
    ///
    /// The `order_id` is the ID of the order to cancel. The `symbol` is the symbol
    /// of the market to cancel the order in.
    async fn cancel_order(&self, order_id: &str, symbol: &str) -> Self::CancelOrderOutput {
        let trader = self.trader(2500);
        let request = CancelOrderRequest {
            category: Category::Linear,
            symbol: Cow::Borrowed(symbol),
            order_id: Some(Cow::Borrowed(order_id)),
            order_filter: None,
            order_link_id: None,
        };
        match trader.cancel_order(request).await {
            Ok(res) => {
                let order_message = format!(
                    "Order cancelled. Symbol: {}, Order ID: {}",
                    symbol, order_id
                );
                let _ = self
                    .bot
                    .send_message(&self.logger.success(&order_message))
                    .await;
                Ok(res.result)
            }
            Err(e) => Err(e),
        }
    }

    /// Cancels all open orders on Bybit.
    ///
    /// # Arguments
    ///
    /// - `symbol`: The symbol of the market to cancel all orders in.
    ///
    /// # Returns
    ///
    /// A `Result` containing a list of the orders that were cancelled.
    ///
    /// # Notes
    ///
    /// The `symbol` is the symbol of the market to cancel all orders in.
    async fn cancel_all(&self, symbol: &str) -> Self::CancelAllOutput {
        let trader = self.trader(2500);
        let request = CancelallRequest {
            category: Category::Linear,
            symbol,
            ..Default::default()
        };
        match trader.cancel_all_orders(request).await {
            Ok(res) => {
                let order_message = format!("All orders cancelled. Symbol: {}", symbol);
                let _ = self
                    .bot
                    .send_message(&self.logger.success(&order_message))
                    .await;
                Ok(res.result.list)
            }
            Err(e) => Err(e),
        }
    }

    /// Amends multiple orders on Bybit.
    ///
    /// # Arguments
    ///
    /// - `orders`: A list of the orders to amend.
    ///
    /// # Returns
    ///
    /// A `Result` containing a list of the orders that were amended.
    ///
    /// # Notes
    ///
    /// The `orders` argument is a list of the orders to amend. Each order is
    /// represented by a `BatchAmend` struct. The `BatchAmend` struct contains
    /// the symbol of the market to amend the order in, the ID of the order to
    /// amend, the new price to place the order at, and the new quantity of the
    /// order.
    async fn batch_amends(&self, orders: Vec<BatchAmend>) -> Self::BatchAmendsOutput {
        let trader = self.trader(2500);
        let mut amends = Vec::with_capacity(10);
        let request = orders.clone().into_req();
        match trader.batch_amend_order(request).await {
            Ok(res) => {
                for ((live_order, ext_info), orders) in res
                    .result
                    .list
                    .iter()
                    .zip(res.ret_ext_info.list.iter())
                    .zip(orders)
                {
                    if ext_info.code == 0 && ext_info.msg == "OK" {
                        let order_message = format!(
                            "Order amended. Symbol: {}, Order ID: {}, Price: {}, Quantity: {}",
                            live_order.symbol, live_order.order_id, orders.1, orders.2
                        );
                        amends.push(LiveOrder::new(
                            live_order.order_id.clone(),
                            orders.1,
                            orders.2,
                        ));
                        let _ = self
                            .bot
                            .send_message(&self.logger.success(&order_message))
                            .await;
                    }
                }
                Ok(amends)
            }
            Err(e) => {
                let error_message = format!("Order failed. Error: {}", e);
                let _ = self
                    .bot
                    .send_message(&self.logger.error(&error_message))
                    .await;
                Err(e)
            }
        }
    }

    /// Places multiple orders on Bybit.
    ///
    /// # Arguments
    ///
    /// - `orders`: A list of the orders to place.
    ///
    /// # Returns
    ///
    /// A `Result` containing a list of the orders that were placed.
    ///
    /// # Notes
    ///
    /// The `orders` argument is a list of the orders to place. Each order is
    /// represented by a `BatchOrder` struct. The `BatchOrder` struct contains
    /// the symbol of the market to place the order in, the price to place the
    /// order at, the quantity of the order, and a boolean indicating whether
    /// the order is a buy or sell.
    ///
    async fn batch_orders(&self, orders: Vec<BatchOrder>) -> Self::BatchOrdersOutput {
        let trader = self.trader(2500);
        let request = orders.clone().into_req();
        let mut live_sells = Vec::with_capacity(5);
        let mut live_buys = Vec::with_capacity(5);
        match trader.batch_place_order(request).await {
            Ok(res) => {
                for ((live_order, ext_info), order_req) in res
                    .result
                    .list
                    .iter()
                    .zip(res.ret_ext_info.list.iter())
                    .zip(orders)
                {
                    if ext_info.code == 0 && ext_info.msg == "OK" {
                        let order_message = format!(
                        "Order placed. Symbol: {}, Price: {}, Quantity: {}, Side: {}, Order ID: {}",
                        live_order.symbol,
                        order_req.1,
                        order_req.2,
                        if order_req.3 { "Buy" } else { "Sell" },
                        live_order.order_id
                    );
                        if order_req.3 {
                            live_buys.push(LiveOrder::new(
                                live_order.order_id.clone(),
                                order_req.1,
                                order_req.2,
                            ));
                        } else {
                            live_sells.push(LiveOrder::new(
                                live_order.order_id.clone(),
                                order_req.1,
                                order_req.2,
                            ));
                        }
                        let _ = self
                            .bot
                            .send_message(&self.logger.success(&order_message))
                            .await;
                    } else {
                        // Handle failed orders
                        let error_message = format!(
                            "Order failed. Symbol: {}, Error: {} (Code: {})",
                            live_order.symbol, ext_info.msg, ext_info.code
                        );
                        let _ = self
                            .bot
                            .send_message(&self.logger.error(&error_message))
                            .await;
                    }
                }
                Ok(vec![live_buys, live_sells])
            }
            Err(e) => {
                let error_message = format!("Order failed. Error: {}", e);
                let _ = self
                    .bot
                    .send_message(&self.logger.error(&error_message))
                    .await;
                Err(e)
            }
        }
    }

    /// Retrieves symbol information from Bybit.
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
        let market_data: MarketData = Bybit::new(None, None);
        let request = InstrumentRequest::new(Category::Linear, Some(symbol), None, None, None);
        match market_data.get_futures_instrument_info(request).await {
            Ok(res) => {
                let data = SymbolInfo {
                    tick_size: res.result.list[0].price_filter.tick_size,
                    lot_size: if let Some(v) = &res.result.list[0].lot_size_filter.qty_step {
                        v.parse::<f64>().unwrap_or(0.0)
                    } else {
                        0.0
                    },
                    min_notional: if let Some(v) =
                        &res.result.list[0].lot_size_filter.min_notional_value
                    {
                        v.parse::<f64>().unwrap_or(0.0)
                    } else {
                        0.0
                    },
                    post_only_max: res.result.list[0].lot_size_filter.max_order_qty,
                    min_qty: res.result.list[0].lot_size_filter.min_order_qty,
                };
                Ok(data)
            }
            Err(e) => {
                let error_message = format!("Order failed. Error: {}", e);
                let _ = self
                    .bot
                    .send_message(&self.logger.error(&error_message))
                    .await;
                Err(e)
            }
        }
    }
    /// Subscribes to Bybit futures market data for the given symbols and sends
    /// it over the given sender channel.
    ///
    /// The data is sent as a `BybitMarket` struct, which contains the order
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
    ) {
        let delay = 600;
        let market_stream: Stream = Bybit::new(None, None);
        let category = Category::Linear;
        let args = build_request(&symbols);
        let mut market_data = BybitMarket::default();
        let request = Subscription::new("subscribe", args.iter().map(String::as_str).collect());

        for k in symbols.clone() {
            market_data.books.insert(k.clone(), BybitBook::new());
            market_data
                .trades
                .insert(k.clone(), VecDeque::with_capacity(1000));
            market_data.ticker.insert(k, VecDeque::with_capacity(10));
        }
        for k in symbols {
            if let Some(book) = market_data.books.get_mut(&k) {
                match self.get_symbol_info(&k).await {
                    Ok(res) => {
                        book.tick_size = res.tick_size;
                        book.lot_size = res.lot_size;
                        book.min_notional = res.min_notional;
                        book.post_only_max = res.post_only_max;
                        book.min_qty = res.min_qty;
                    }
                    Err(e) => {
                        let error_message = format!("Order failed. Error: {}", e);
                        let _ = self
                            .bot
                            .send_message(&self.logger.error(&error_message))
                            .await;
                    }
                }
            }
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
                    let symbol = data.topic.split(".").nth(1).unwrap();
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
                    println!("Subscribed to market stream");
                }
                Err(e) => {
                    let error_message = format!("Error: {}", e);
                    let _ = self.bot.send_message(&error_message).await;
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }

    /// Subscribes to Bybit user stream data for the given symbol and sends it over the given sender channel.
    ///
    /// The data is sent as a `BybitPrivate` struct, which contains the user's wallet, positions, order book, and order data.
    ///
    /// The `sender` channel must be kept open for the duration of the subscription, as the data is sent over it.
    ///
    /// The function returns an empty tuple.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The symbol to subscribe to.
    /// * `sender` - An unbounded sender channel that will receive the user stream data.
    ///
    /// # Returns
    ///
    /// An empty tuple.
    async fn private_subscribe(
        &self,
        symbol: String,
        sender: tokio::sync::mpsc::UnboundedSender<Self::PrivateStreamData>,
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
                    let error_message = format!("Error: {}", e);
                    let _ = self.bot.send_message(&error_message).await;
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }
}

impl OrderBook for BybitBook {
    type Ask = Ask;
    type Bid = Bid;

    /// Creates a new `BybitBook` with all fields initialized to zero.
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
    fn set_mid_price(&mut self) {
        self.mid_price = (self.best_ask.price + self.best_bid.price) * 0.5;
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
    /// The best ask is the highest price ask in the order book, and the best bid is the lowest price
    /// bid in the order book.
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
    /// Returns the minimum notional value of the symbol.
    ///
    /// The minimum notional value is the minimum value that can be traded in the symbol.
    fn get_min_notional(&self) -> f64 {
        self.min_notional
    }
    /// Returns the post-only maximum quantity of the symbol.
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
    /// Returns the minimum quantity of the symbol.
    ///
    /// The minimum quantity is the minimum amount that can be traded in the symbol.
    fn min_qty(&self) -> f64 {
        self.min_qty
    }
    /// Calculates the weighted mid price of the order book.
    ///
    /// The weighted mid price is calculated as the weighted average of the best bid and best ask
    /// prices, where the weights are the quantities at the best bid and best ask. The `depth`
    /// parameter can be used to specify the depth of the order book to use when calculating the
    /// weighted mid price. If `depth` is `None`, the best bid and best ask quantities are used.
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
    /// The microprice is a measure of the order book imbalance, calculated as the weighted
    /// average of the best bid and best ask prices. The weights are the quantities at the best
    /// bid and best ask. The `depth` parameter can be used to specify the depth of the order
    /// book to use when calculating the microprice. If `depth` is `None`, the best bid and best
    /// ask quantities are used. If the total quantity is 0, the mid price of the order book is
    /// returned.
    ///
    /// The microprice is calculated as follows:
    ///
    /// * Let `bid_qty` be the weighted bid quantity, calculated by summing the products of the
    ///   prices and quantities of the bids in the order book, starting from the best bid and
    ///   going down in price.
    /// * Let `ask_qty` be the weighted ask quantity, calculated by summing the products of the
    ///   prices and quantities of the asks in the order book, starting from the best ask and
    ///   going up in price.
    /// * Let `total_qty` be the sum of `bid_qty` and `ask_qty`.
    /// * The microprice is then calculated as `(best_ask_price * qty_ratio) + (best_bid_price *
    ///   (1.0 - qty_ratio))`, where `qty_ratio` is `bid_qty / total_qty`.
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
    /// The order flow imbalance is a measure of the change in the order book imbalance
    /// between the old and new order books. The order flow imbalance is calculated as
    /// the sum of the bid and ask order flow imbalances.
    ///
    /// The bid order flow imbalance is calculated as the difference in the volume at the
    /// best bid between the old and new order books. If the best bid price has increased or
    /// the volume at the best bid has increased, the bid order flow imbalance is positive.
    /// If the best bid price has decreased or the volume at the best bid has decreased, the
    /// bid order flow imbalance is negative.
    ///
    /// The ask order flow imbalance is calculated as the difference in the volume at the
    /// best ask between the old and new order books. If the best ask price has decreased or
    /// the volume at the best ask has increased, the ask order flow imbalance is positive.
    /// If the best ask price has increased or the volume at the best ask has decreased, the
    /// ask order flow imbalance is negative.
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

    /// Builds a list of Bybit subscriptions for the given symbols.
    ///
    /// The subscriptions that are built are:
    ///
    /// - Orderbook with 1, 50, and 200 levels for each symbol
    /// - The ticker for each symbol
    /// - The public trades for each symbol
    ///
    /// # Arguments
    ///
    /// * `symbol` - A vector of strings representing the symbols to subscribe to.
    ///
    /// # Returns
    ///
    /// A vector of strings representing the subscriptions to make.
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
