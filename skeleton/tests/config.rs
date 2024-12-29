mod tests {
    use super::*;
    use skeleton::utils::config::read_toml;
    use skeleton::utils::models::Config;
    #[test]
    fn test_read_toml() {
        let config = read_toml::<&str, Config>("./tests/test.toml");
        println!("{:#?}", config);

    }
}
