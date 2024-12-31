use teloxide::{prelude::Requester, types::ChatId, Bot, RequestError};
use tokio::sync::OnceCell;

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
    pub fn new(token: String, chat_id: i64) -> Result<Self, RequestError> {
        if BOT.get().is_none() {
            let bot = Bot::new(&token);
            _ = BOT.set(bot);
        }
        Ok(Self { token, chat_id })
    }

    pub async fn send_message(&self, msg: &str) -> Result<bool, RequestError> {
        if let Some(bot) = BOT.get() {
            bot.send_message(ChatId(self.chat_id), msg).await?;
        }
        Ok(true)
    }
}
