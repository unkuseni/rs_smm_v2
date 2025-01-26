use teloxide::{prelude::Requester, types::ChatId, Bot, RequestError};
use tokio::sync::OnceCell;

use super::models::Config;

static BOT: OnceCell<Bot> = OnceCell::const_new();
#[derive(Debug, Clone)]
pub struct LiveBot {
    bot: OnceCell<Bot>,
    chat_id: i64,
}

impl LiveBot {
    pub async fn new(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config: Config = super::config::read_toml(config_path).await?;
        if BOT.get().is_none() {
            let bot = Bot::new(&config.token);
            _ = BOT.set(bot);
        }
        Ok(Self {
            bot: BOT.clone(),
            chat_id: config.chat_id,
        })
    }
    pub async fn send_message(&self, msg: &str) -> Result<bool, RequestError> {
        if let Some(init_bot) = self.bot.get() {
            init_bot.send_message(ChatId(self.chat_id), msg).await?;
        }
        Ok(true)
    }

    pub fn chat_id(&self) -> i64 {
        self.chat_id
    }
}
