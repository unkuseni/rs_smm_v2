#[cfg(test)]
mod tests {
    use skeleton::utils::{bot::LiveBot, logger::Logger};

    #[tokio::test]
    async fn test_logger() {
        let bot = LiveBot::new("./tests/test.toml").await.unwrap();
        let log = Logger::new(bot);
        let _ = log.info("This is a test, Please ignore");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        println!("Logger test passed");
    }
}
