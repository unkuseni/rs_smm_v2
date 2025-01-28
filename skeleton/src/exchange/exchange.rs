use binance::model::AggrTradesEvent;
use bybit::model::WsTrade;
use std::{collections::VecDeque, future::Future};
use tokio::sync::mpsc::UnboundedSender;

use crate::utils::models::{BatchAmend, BatchOrder, BinanceMarket, BybitMarket};

pub trait Exchange {
    type TimeOutput;
    type FeeOutput;
    type LeverageOutput;
    type TraderOutput;
    type StreamData;
    type StreamOutput;
    type PrivateStreamData;
    type PrivateStreamOutput;
    type PlaceOrderOutput;
    type AmendOrderOutput;
    type CancelOrderOutput;
    type CancelAllOutput;
    type BatchOrdersOutput;
    type BatchAmendsOutput;
    type SymbolInformationOutput;

    fn init(api_key: String, api_secret: String) -> impl Future<Output = Self>;
    fn time(&self) -> impl Future<Output = Self::TimeOutput>;
    fn fees(&self, symbol: String) -> impl Future<Output = Self::FeeOutput>;
    fn set_leverage(
        &self,
        symbol: &str,
        leverage: u8,
    ) -> impl Future<Output = Self::LeverageOutput>;
    fn trader(&self, recv_window: u16) -> Self::TraderOutput;
    fn place_order(
        &self,
        symbol: &str,
        price: f64,
        qty: f64,
        is_buy: bool,
    ) -> impl Future<Output = Self::PlaceOrderOutput>;
    fn amend_order(
        &self,
        order_id: &str,
        price: f64,
        qty: f64,
        symbol: &str,
    ) -> impl Future<Output = Self::AmendOrderOutput>;
    fn cancel_order(
        &self,
        order_id: &str,
        symbol: &str,
    ) -> impl Future<Output = Self::CancelOrderOutput>;
    fn cancel_all(&self, symbol: &str) -> impl Future<Output = Self::CancelAllOutput>;
    fn batch_orders(
        &self,
        orders: Vec<BatchOrder>,
    ) -> impl Future<Output = Self::BatchOrdersOutput>;
    fn batch_amends(
        &self,
        orders: Vec<BatchAmend>,
    ) -> impl Future<Output = Self::BatchAmendsOutput>;
    fn get_symbol_info(&self, symbol: &str) -> impl Future<Output = Self::SymbolInformationOutput>;
    fn market_subscribe(
        &self,
        symbols: Vec<String>,
        sender: UnboundedSender<Self::StreamData>,
    ) -> impl Future<Output = Self::StreamOutput>;
    fn private_subscribe(
        &self,
        symbol: String,
        sender: UnboundedSender<Self::PrivateStreamData>,
    ) -> impl Future<Output = Self::PrivateStreamOutput>;
}

#[derive(Debug, Clone)]
pub enum MarketData {
    Bybit(BybitMarket),
    Binance(BinanceMarket),
}

#[derive(Debug, Clone)]
pub enum TradeType {
    Bybit(VecDeque<WsTrade>),
    Binance(VecDeque<AggrTradesEvent>),
}
