use std::{
    borrow::Cow,
    collections::{BTreeMap, VecDeque},
};

use binance::model::{AggrTradesEvent, Asks, Bids, BookTickerEvent};
use bybit::model::{
    AmendOrderRequest, Ask, BatchAmendRequest, BatchPlaceRequest, Bid, Category, FastExecData,
    LinearTickerData, OrderData, OrderRequest, PositionData, Side, WalletData, WsTrade,
};
use ordered_float::OrderedFloat;
use serde::Deserialize;

use super::{bot::LiveBot, logger::Logger};

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub token: String,
    pub chat_id: i64,
}

impl PartialEq for Config {
    fn eq(&self, other: &Self) -> bool {
        self.token == other.token && self.chat_id == other.chat_id
    }

    fn ne(&self, other: &Self) -> bool {
        self.token != other.token && self.chat_id != other.chat_id
    }
}
#[derive(Clone, Debug)]
pub struct BybitClient {
    pub api_key: String,
    pub api_secret: String,
    pub logger: Logger,
    pub bot: LiveBot,
}
#[derive(Clone, Debug)]
pub struct BinanceClient {
    pub api_key: String,
    pub api_secret: String,
    pub logger: Logger,
    pub bot: LiveBot,
}

#[derive(Clone, Debug)]
pub struct BybitMarket {
    pub timestamp: u64,
    pub books: BTreeMap<String, BybitBook>,
    pub trades: BTreeMap<String, VecDeque<WsTrade>>,
    pub ticker: BTreeMap<String, VecDeque<LinearTickerData>>,
}

impl Default for BybitMarket {
    fn default() -> Self {
        Self {
            timestamp: 0,
            books: BTreeMap::new(),
            trades: BTreeMap::new(),
            ticker: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinanceMarket {
    pub timestamp: u64,
    pub books: BTreeMap<String, BinanceBook>,
    pub trades: BTreeMap<String, VecDeque<AggrTradesEvent>>,
    pub ticker: BTreeMap<String, VecDeque<BookTickerEvent>>,
}
impl Default for BinanceMarket {
    fn default() -> Self {
        Self {
            timestamp: 0,
            books: BTreeMap::new(),
            trades: BTreeMap::new(),
            ticker: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BybitBook {
    pub last_update: u64,
    pub sequence: u64,
    pub asks: BTreeMap<OrderedFloat<f64>, f64>,
    pub bids: BTreeMap<OrderedFloat<f64>, f64>,
    pub best_ask: Ask,
    pub best_bid: Bid,
    pub mid_price: f64,
    pub tick_size: f64,
    pub lot_size: f64,
    pub min_notional: f64,
    pub min_qty: f64,
    pub post_only_max: f64,
}

/// symbol, price, qty, side
#[derive(Debug, Clone)]
pub struct BatchOrder(pub String, pub f64, pub f64, pub bool);

impl BatchOrder {
    pub fn new(symbol: String, price: f64, qty: f64, is_buy: bool) -> Self {
        Self(symbol, price, qty, is_buy)
    }
}

pub trait IntoReq<'a> {
    type BatchOrdersOutput;

    fn into_req(self) -> Self::BatchOrdersOutput;
}

impl<'a> IntoReq<'a> for Vec<BatchOrder> {
    type BatchOrdersOutput = BatchPlaceRequest<'a>;
    fn into_req(self) -> BatchPlaceRequest<'a> {
        BatchPlaceRequest {
            category: Category::Linear,
            requests: self
                .into_iter()
                .map(|order| OrderRequest {
                    symbol: order.0.into(),
                    price: Some(order.1),
                    qty: order.2,
                    side: if order.3 { Side::Buy } else { Side::Sell },
                    order_type: bybit::model::OrderType::Limit,
                    time_in_force: Some(Cow::Borrowed("PostOnly")),
                    ..Default::default()
                })
                .collect(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct BatchAmend(pub String, pub f64, pub f64, pub String);

impl BatchAmend {
    pub fn new(symbol: String, price: f64, qty: f64, order_id: String) -> Self {
        Self(symbol, price, qty, order_id)
    }
}

impl<'a> IntoReq<'a> for Vec<BatchAmend> {
    type BatchOrdersOutput = BatchAmendRequest<'a>;
    fn into_req(self) -> Self::BatchOrdersOutput {
        BatchAmendRequest {
            category: Category::Linear,
            requests: self
                .into_iter()
                .map(|amend| AmendOrderRequest {
                    symbol: amend.0.into(),
                    category: Category::Linear,
                    order_id: Some(amend.3.into()),
                    price: Some(amend.1),
                    qty: amend.2,
                    ..Default::default()
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LiveOrder {
    pub order_id: String,
    pub price: f64,
    pub qty: f64,
}
impl Default for LiveOrder {
    fn default() -> Self {
        Self {
            order_id: String::new(),
            price: 0.0,
            qty: 0.0,
        }
    }
}

impl LiveOrder {
    pub fn new(order_id: String, price: f64, qty: f64) -> Self {
        Self {
            order_id,
            price,
            qty,
        }
    }
}

impl PartialOrd for LiveOrder {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.price.partial_cmp(&other.price)
    }
}

impl PartialEq for LiveOrder {
    fn eq(&self, other: &Self) -> bool {
        self.order_id == other.order_id
    }
    fn ne(&self, other: &Self) -> bool {
        self.order_id != other.order_id
    }
}
#[derive(Debug, Clone)]
pub struct BinanceBook {
    pub last_update: u64,
    pub sequence: u64,
    pub asks: BTreeMap<OrderedFloat<f64>, f64>,
    pub bids: BTreeMap<OrderedFloat<f64>, f64>,
    pub best_ask: Asks,
    pub best_bid: Bids,
    pub mid_price: f64,
    pub tick_size: f64,
    pub lot_size: f64,
    pub min_notional: f64,
    pub min_qty: f64,
    pub post_only_max: f64,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
  pub   tick_size: f64,
  pub   lot_size: f64,
  pub   min_notional: f64,
  pub   min_qty: f64,
  pub   post_only_max: f64,
}

#[derive(Clone, Debug)]
pub struct BybitPrivate {
    pub time: u64,
    pub wallet: VecDeque<WalletData>,
    pub orders: VecDeque<OrderData>,
    pub positions: VecDeque<PositionData>,
    pub executions: VecDeque<FastExecData>,
}
impl Default for BybitPrivate {
    fn default() -> Self {
        Self {
            time: 0,
            wallet: VecDeque::with_capacity(20),
            orders: VecDeque::with_capacity(500),
            positions: VecDeque::with_capacity(500),
            executions: VecDeque::with_capacity(500),
        }
    }
}
