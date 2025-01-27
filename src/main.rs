use std::collections::HashMap;

use rs_smm_v2::{params::params::use_toml, strategy::maker::Maker};
use skeleton::{
    exchange::exchange::Exchange,
    ss,
    utils::models::{BybitClient, Config},
};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let Config {
        api_keys,
        balances,
        leverage,
        orders_per_side,
        depths,
        rate_limit,
        tick_window,
        bps,
        ..
    } = use_toml().await;

    let mut state = ss::SharedState::new("bybit".to_string());
    let mut symbols_array = Vec::new();
    let clients = api_keys;
    for (key, secret, symbol) in clients {
        symbols_array.push(symbol.clone());
        state.add_clients(symbol, BybitClient::init(key, secret).await);
    }

    // Create a hashmap for balances of each client/symbols
    let balance = map_balances(balances);

    // Initialize the market maker and set the initial state, balance, leverage, orders per side, final order distance, depths, and rate limit
    let mut market_maker = Maker::new(
        state.clone(),
        symbols_array,
        balance,
        leverage,
        orders_per_side,
        rate_limit,
        tick_window,
        depths,
    );

    // sets the  base spread in bps for profit
    market_maker.set_spread_toml(bps);

    // create an unbounded channel
    let (sender, receiver) = mpsc::unbounded_channel();

    // loads up the shareed state and sends it across the channel
    tokio::spawn(async move {
        state.load_data(sender).await;
    });

    // passes in the data receiver to the market maker and starts the loop
    market_maker.start_loop(receiver).await;
}

fn map_balances(arr: Vec<(String, f64)>) -> HashMap<String, f64> {
    let mut new_map = HashMap::new();
    for (k, v) in arr {
        new_map.insert(k, v);
    }
    new_map
}
