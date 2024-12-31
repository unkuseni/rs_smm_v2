use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub token: String,
    pub chat_id: i64,
}

impl PartialEq for Config {
    fn eq(&self, other: &Self) -> bool {
        self.token == other.token && self.chat_id == other.chat_id
    }

    fn ne(&self, other: &Self) -> bool {
        self.token != other.token && self.chat_id != other.chat_id
    }
}
