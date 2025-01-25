// bot.rs
use teloxide::{prelude::Requester, types::ChatId, Bot, RequestError};
use tokio::sync::OnceCell;

use super::models::Config;

static BOT: OnceCell<Bot> = OnceCell::const_new();

#[derive(Debug, Clone)]
pub struct LiveBot {
    chat_id: i64,
    token: String,
}

impl LiveBot {
    /// Async initialization with config loading
    pub async fn new(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config: Config = super::config::read_toml(config_path).await?;

        // Initialize bot instance once
        BOT.get_or_init(|| async { Bot::new(&config.token) });

        Ok(Self {
            chat_id: config.chat_id,
            token: config.token,
        })
    }

    /// Send message with automatic rate limiting
    pub async fn send_message(&self, msg: &str) -> Result<(), RequestError> {
        BOT.get()
            .expect("Bot should be initialized")
            .send_message(ChatId(self.chat_id), msg)
            .await?;

        Ok(())
    }

    /// Get current chat ID for monitoring
    pub fn chat_id(&self) -> i64 {
        self.chat_id
    }
}