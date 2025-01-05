#[cfg(test)]
mod tests {
    use skeleton::exchange::exchange::Exchange;
    use skeleton::utils::localorderbook::OrderBook;
    use skeleton::utils::models::{BinanceClient, BinanceMarket, BybitClient};
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_bybit_market() {
        let api_key: String = String::from("");
        let api_secret: String = String::from("");
        let client = BybitClient::init(api_key, api_secret);
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let stream = tokio::spawn(async move {
            client
                .market_subscribe(vec!["ETHUSDT".to_string()], sender)
                .await;
        });
        while let Some(data) = receiver.recv().await {
            if let Some(event) = data.books.get("ETHUSDT") {
                let (mut asks, bids) = event.get_depth(4);
                asks.reverse();
                if let Some(new_trades) = data.trades.get("ETHUSDT") {
                    println!(
                        "Current ETHUSDT price:\nBest Asks: {:#?}\nWMID: {:#?}  Trade: {:#?}  Trend: {:#?}\nBest Bids: {:#?}\n",
                        asks,
                        event.get_wmid(Some(4)),
                        new_trades.len(),
                        if (event.get_microprice(Some(4)) - event.best_bid.price) > (event.best_ask.price - event.get_microprice(Some(4))) {"up"} else {"down"},
                        bids
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_binance_market() {
        let api_key: String = String::from("");
        let api_secret: String = String::from("");
        let client = BinanceClient::init(api_key, api_secret);
        let (sender, mut receiver) = mpsc::unbounded_channel::<BinanceMarket>();
        let sender_clone = sender.clone();
        let stream = tokio::spawn(async move {
            client
                .market_subscribe(vec!["SOLUSDT".to_string()], sender_clone)
                .await;
        });

        while let Some(data) = receiver.recv().await {
            if let Some(event) = data.trades.get("SOLUSDT") {
                let mut delta = 0.0;
                for trade in event {
                    if trade.is_buyer_maker == true {
                        delta -= trade.qty.parse::<f64>().unwrap();
                    } else {
                        delta += trade.qty.parse::<f64>().unwrap();
                    }
                }

                println!("Current SOLUSDT price:\n{:#?}\n", delta);
            }
        }
    }

    #[tokio::test]
    async fn test_bybit_private() {
        let api_key: String = String::from("");
        let api_secret: String = String::from("");
        let client = BybitClient::init(api_key, api_secret);
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let stream = tokio::spawn(async move {
            client
                .private_subscribe("SOLUSDT".to_string(), sender)
                .await;
        });

        while let Some(data) = receiver.recv().await {
            if data.0 == "SOLUSDT" {
                println!("Current SOLUSDT price:\n{:#?}\n", data.1);
            }
        }
    }
}
