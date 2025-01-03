use bybit::{
    account::AccountManager,
    api::Bybit,
    config::Config,
    errors::BybitError,
    general::General,
    model::{
        AmendOrderRequest, Ask, BatchAmendRequest, Bid, CancelOrderRequest, CancelallRequest,
        Category, LeverageRequest, OrderBookUpdate, OrderStatus, Subscription, Tickers,
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
        LiveOrder,
    },
};

use super::exchange::Exchange;

impl Exchange for BybitClient {
    type TimeOutput = Result<u64, BybitError>;
    type FeeOutput = Result<String, BybitError>;
    type LeverageOutput = Result<bool, BybitError>;
    type TraderOutput = Trader;
    type StreamOutput = BybitMarket;
    type PrivateStreamOutput = (String, BybitPrivate);
    type PlaceOrderOutput = Result<LiveOrder, BybitError>;
    type AmendOrderOutput = Result<LiveOrder, BybitError>;
    type CancelOrderOutput = Result<OrderStatus, BybitError>;
    type CancelAllOutput = Result<Vec<OrderStatus>, BybitError>;
    type BatchOrdersOutput = Result<Vec<Vec<LiveOrder>>, BybitError>;
    type BatchAmendsOutput = Result<Vec<LiveOrder>, BybitError>;
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
            Ok(_) => {
                let success_message = format!("Set leverage for {} to {}", symbol, leverage);
                let _ = self.bot.send_message(&success_message).await;
                Ok(true)
            }
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
