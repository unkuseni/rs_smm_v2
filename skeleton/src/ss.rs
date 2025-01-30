use std::{collections::BTreeMap, sync::Arc, vec};

use tokio::sync::{mpsc, Mutex};

use crate::{
    exchange::exchange::{Exchange, MarketData},
    utils::models::{BinanceClient, BinanceMarket, BybitClient, BybitMarket, BybitPrivate},
};

#[derive(Debug, Clone)]
pub struct SharedState {
    pub exchange: String,
    pub clients: BTreeMap<String, BybitClient>,
    pub privates: BTreeMap<String, BybitPrivate>,
    pub markets: Vec<MarketData>,
    pub symbols: Vec<String>,
}

impl SharedState {
    pub fn new(exchange: String) -> Self {
        Self {
            exchange,
            clients: BTreeMap::new(),
            privates: BTreeMap::new(),
            markets: vec![
                MarketData::Bybit(BybitMarket::default()),
                MarketData::Binance(BinanceMarket::default()),
            ],
            symbols: Vec::new(),
        }
    }

    pub fn add_clients(&mut self, symbol: String, client: BybitClient) {
        self.symbols.push(symbol.clone());
        self.clients.insert(symbol.clone(), client);
        self.privates
            .entry(symbol)
            .or_insert(BybitPrivate::default());
    }

    pub async fn load_data(state: SharedState, state_sender: mpsc::UnboundedSender<SharedState>) {
        match state.exchange.as_str() {
            "bybit" => Self::load_bybit(state, state_sender).await,
            "binance" => Self::load_binance(state, state_sender).await,
            "both" => Self::load_both(state, state_sender).await,
            _ => panic!("Invalid exchange"),
        }
    }

    async fn load_binance(_state: SharedState, _state_sender: mpsc::UnboundedSender<SharedState>) {
        unimplemented!("Binance not implemented");
    }

    async fn load_bybit(state: SharedState, state_sender: mpsc::UnboundedSender<SharedState>) {
        let symbols = state.symbols.clone();

        let (bybit_market_sender, mut bybit_market_receiver) =
            mpsc::unbounded_channel::<BybitMarket>();
        let (bybit_private_sender, mut bybit_private_receiver) =
            mpsc::unbounded_channel::<(String, BybitPrivate)>();

        for (symbol, client) in state.clients.clone() {
            let private_clone = bybit_private_sender.clone();
            tokio::spawn(async move {
                client.private_subscribe(symbol, private_clone).await;
            });
        }
        tokio::spawn(async move {
            let market_stream = BybitClient::init("".to_string(), "".to_string()).await;
            market_stream
                .market_subscribe(symbols, bybit_market_sender)
                .await;
        });
        let state = Arc::new(Mutex::new(state.clone()));

        loop {
            tokio::select! {
            Some(data) = bybit_market_receiver.recv() => {
                let mut state = state.lock().await;
                state.markets[0] = MarketData::Bybit(data);
                state_sender.send(state.clone()).unwrap();
                }
            Some(data) = bybit_private_receiver.recv() => {
                let mut state = state.lock().await;
                state.privates.insert(data.0, data.1);
                state_sender.send(state.clone()).unwrap();
                }
            }
        }
    }

    async fn load_both(state: SharedState, state_sender: mpsc::UnboundedSender<SharedState>) {
        let (bybit_market_sender, mut bybit_market_receiver) =
            mpsc::unbounded_channel::<BybitMarket>();
        let (binance_market_sender, mut binance_market_receiver) =
            mpsc::unbounded_channel::<BinanceMarket>();
        let (bybit_private_sender, mut bybit_private_receiver) =
            mpsc::unbounded_channel::<(String, BybitPrivate)>();

        let binance_symbols = state.symbols.clone();
        let bybit_symbols = state.symbols.clone();

        for (symbol, client) in state.clients.clone() {
            let private_clone = bybit_private_sender.clone();
            tokio::spawn(async move {
                client.private_subscribe(symbol, private_clone).await;
            });
        }

        let state = Arc::new(Mutex::new(state.clone()));

        tokio::spawn(async move {
            let market_stream = BybitClient::init("".to_string(), "".to_string()).await;
            market_stream
                .market_subscribe(bybit_symbols, bybit_market_sender)
                .await;
        });

        tokio::spawn(async move {
            let market_stream = BinanceClient::init("".to_string(), "".to_string()).await;
            market_stream
                .market_subscribe(binance_symbols, binance_market_sender)
                .await;
        });

        loop {
            tokio::select! {
              Some(data) = bybit_market_receiver.recv() => {
                let mut state = state.lock().await;
                state.markets[0] = MarketData::Bybit(data);
                state_sender.send(state.clone()).unwrap();
              }
              Some(data) = binance_market_receiver.recv() => {
                let mut state = state.lock().await;
                state.markets[1] = MarketData::Binance(data);
                state_sender.send(state.clone()).unwrap();
              }
              Some(data) = bybit_private_receiver.recv() => {
                let mut state = state.lock().await;
                state.privates.insert(data.0, data.1);
                state_sender.send(state.clone()).unwrap();
              }
            }
        }
    }
}
