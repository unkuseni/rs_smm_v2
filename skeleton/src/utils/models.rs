use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub logger_token: String,
    pub chat_id: String,
}

impl PartialEq for Config {
    fn eq(&self, other: &Self) -> bool {
        self.logger_token == other.logger_token && self.chat_id == other.chat_id
    }

    fn ne(&self, other: &Self) -> bool {
        self.logger_token != other.logger_token && self.chat_id != other.chat_id
    }
}
