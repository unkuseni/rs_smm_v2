mod tests {
    use super::*;
    use skeleton::utils::config::{read_toml, watch_config};
    use skeleton::utils::models::Config;
    #[test]
    fn test_read_toml() {
        let config = read_toml::<&str, Config>("./tests/test.toml");
        println!("{:#?}", config);
    }

    #[tokio::test]
    async fn test_watch_config() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Config>();

        tokio::spawn(async move {
            watch_config("./tests/test.toml", tx).await.unwrap();
        });

        while let Some(config) = rx.recv().await {
            println!("{:#?}", config);
        }
    }
}
deborah