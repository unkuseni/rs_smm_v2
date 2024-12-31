use teloxide::{prelude::Requester, types::ChatId, Bot, RequestError};
use tokio::sync::OnceCell;

use super::{config::read_toml, models::Config};

static BOT: OnceCell<Bot> = OnceCell::const_new();

pub struct LiveBot {
    pub token: String,
    pub chat_id: i64,
}
impl LiveBot {
    /// Initializes a new `LiveBot` instance.
    ///
    /// If the `BOT` static instance is `None`, initializes it with the given `token`.
    /// Otherwise, the existing bot instance is used.
    ///
    /// # Errors
    ///
    /// Returns an error if the bot initialization fails.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = read_toml::<&str, Config>("./config.toml")?;
        if BOT.get().is_none() {
            let bot = Bot::new(&config.token);
            _ = BOT.set(bot);
        }
        Ok(Self {
            token: config.token,
            chat_id: config.chat_id,
        })
    }

    pub async fn send_message(&self, msg: &str) -> Result<bool, RequestError> {
        if let Some(bot) = BOT.get() {
            bot.send_message(ChatId(self.chat_id), msg).await?;
        }
        Ok(true)
    }
}
