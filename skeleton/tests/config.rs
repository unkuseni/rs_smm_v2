#[cfg(test)]
mod tests {
    use skeleton::utils::config::{read_toml, watch_config};
    use skeleton::utils::models::Config;
    #[tokio::test]
    async fn test_read_toml() {
        let config = read_toml::<&str, Config>("./tests/test.toml").await;
        println!("{:#?}", config);
    }

    #[tokio::test]
    async fn test_watch_config() {
        use tokio::time::{timeout, Duration};

        let (tx, mut rx) = tokio::sync::mpsc::channel::<Config>(10);
        let test_path = "./tests/test.toml";

        // Write initial config content
        let initial_toml = r#"
        token = "initial_value"
    "#;
        tokio::fs::write(test_path, initial_toml).await.unwrap();

        // Spawn the watcher
        let handle = tokio::spawn(async move {
            watch_config(test_path, tx).await.unwrap();
        });

        // Receive initial config
        let first_config = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("Timeout waiting for initial config")
            .unwrap();
        assert_eq!(first_config.token, "initial_value");

        // Modify the config file
        let updated_toml = r#"
        key = "updated_value"
    "#;
        tokio::fs::write(test_path, updated_toml).await.unwrap();

        // Wait for debounce and reload
        let second_config = timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("Timeout waiting for updated config")
            .unwrap();
        assert_eq!(second_config.token, "updated_value");

        // Cleanup
        handle.abort(); // Stop the watcher task
    }
}
