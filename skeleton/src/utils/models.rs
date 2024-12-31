use std::collections::BTreeMap;

use bybit::model::{Ask, Bid};
use ordered_float::OrderedFloat;
use serde::Deserialize;

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

pub struct BybitClient {
    pub api_key: String,
    pub api_secret: String,
}


#[derive(Clone, Debug)]
pub struct BybitMarket {
    pub timestamp: u64,
    pub books: BTreeMap<String, BybitBook>,
}

impl Default for BybitMarket {
    fn default() -> Self {
        Self {
            timestamp: 0,
            books: BTreeMap::new(),
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
}
