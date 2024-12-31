#[cfg(test)]
mod tests {
    use std::time::Duration;

    use skeleton::exchange::exchange::Exchange;
    use skeleton::utils::bot::LiveBot;
    use skeleton::utils::localorderbook::OrderBook;
    use skeleton::utils::models::BybitClient;
    use tokio::sync::mpsc;
    use tokio::time::interval;

    #[tokio::test]
    async fn test_bybit_market() {
        let bot = LiveBot::new().unwrap();
        let api_key: String = String::from("");
        let api_secret: String = String::from("");
        let client = BybitClient::init(api_key, api_secret);
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let stream = tokio::spawn(async move {
            client
                .market_subscribe(vec!["XRPUSDT".to_string()], sender)
                .await;
        });

        let mut message_interval = interval(Duration::from_secs(5));
        let mut latest_data = None;

        while let Some(data) = receiver.recv().await {
            if let Some(event) = data.books.get("XRPUSDT") {
                latest_data = Some(event.clone());
                let depth = event.get_depth(4);
                println!(
                    "Current XRPUSDT price:\nBest Asks: {:#?}\nBest Bids: {:#?}\n",
                    depth.0, depth.1
                );
            }

            tokio::select! {
                _ = message_interval.tick() => {
                    if let Some(event) = &latest_data {
                        let depth =  event.get_depth(4);
                        let message = format!(
                            "Current XRPUSDT price:\nBest Asks: {:#?}\nBest Bids: {:#?}\n",
                            depth.0, depth.1
                        );
                        if let Err(e) = bot.send_message(&message).await {
                            println!("Error sending message: {}", e);
                        }


                    }
                }
                else => continue,
            }
        }
    }
}
