use std::future::Future;
use tokio::sync::mpsc::UnboundedSender;

pub trait MarketData {}

pub trait PrivateData {}

pub trait Exchange {
    type TimeOutput;
    type FeeOutput;
    type LeverageOutput;
    type TraderOutput;
    type StreamOutput;
    fn init(api_key: String, api_secret: String) -> Self;
    fn time(&self) -> impl Future<Output = Self::TimeOutput>;
    fn fees(&self, symbol: String) -> impl Future<Output = Self::FeeOutput>;
    fn set_leverage(
        &self,
        symbol: &str,
        leverage: u8,
    ) -> impl Future<Output = Self::LeverageOutput>;
    fn trader(&self, recv_window: u16) -> Self::TraderOutput;
    fn market_subscribe(
        &self,
        symbols: Vec<String>,
        sender: UnboundedSender<Self::StreamOutput>,
    ) -> impl Future<Output = ()>;
}
