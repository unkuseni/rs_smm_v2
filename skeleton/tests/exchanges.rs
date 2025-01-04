#[cfg(test)]
mod tests {
    use skeleton::exchange::exchange::Exchange;
    use skeleton::utils::bot::LiveBot;
    use skeleton::utils::localorderbook::OrderBook;
    use skeleton::utils::models::BybitClient;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_bybit_market() {
        let bot = LiveBot::new().unwrap();
        let api_key: String = String::from("");
        let api_secret: String = String::from("");
        let client = BybitClient::init(api_key, api_secret);
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let stream = tokio::spawn(async move {
            client
                .market_subscribe(vec!["SOLUSDT".to_string()], sender)
                .await;
        });

        while let Some(data) = receiver.recv().await {
            if let Some(event) = data.books.get("SOLUSDT") {
                let (mut asks, bids) = event.get_depth(4);
                asks.reverse();
                println!(
                    "Current SOLUSDT price:\nBest Asks: {:#?}\nMid: {:#?}\nBest Bids: {:#?}\n",
                    asks, event.mid_price, bids
                );
            }
        }
    }

    #[tokio::test]
    async fn test_bybit_private() {
        let bot = LiveBot::new().unwrap();
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

    #[tokio::test]
    async fn test_binance_market() {
        let bot = LiveBot::new().unwrap();
        let api_key: String = String::from("");
        let api_secret: String = String::from("");
        let client = BybitClient::init(api_key, api_secret);
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let stream = tokio::spawn(async move {
            client
                .market_subscribe(vec!["SOLUSDT".to_string()], sender)
                .await;
        });

        while let Some(data) = receiver.recv().await {
            if let Some(event) = data.books.get("SOLUSDT") {
                let (mut asks, bids) = event.get_depth(4);
                asks.reverse();
                println!(
                    "Current SOLUSDT price:\nBest Asks: {:#?}\nMid: {:#?}\nBest Bids: {:#?}\n",
                    asks, event.mid_price, bids
                );
            }
        }
    }
}
