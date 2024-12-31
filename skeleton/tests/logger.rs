#[cfg(test)]
mod tests {
    use skeleton::utils::{bot::LiveBot, logger::Logger};

    #[tokio::test]
    async fn test_logger() {
        let bot = LiveBot::new().unwrap();
        let log = Logger;
        let _ = bot
            .send_message(&log.info("This is a test, Please ignore"))
            .await;
    }
}
