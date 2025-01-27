use skeleton::utils::{config::read_toml, models::Config};

pub async fn use_toml() -> Config {
    let path = "./config.toml";
    let result = read_toml(path).await.unwrap();
    result
}
