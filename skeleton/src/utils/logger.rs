// logger.rs
use super::{
    bot::LiveBot,
    time::{get_formatted_date, get_formatted_time},
};
use std::fmt;

#[derive(Debug, Clone)]
pub struct Logger {
    bot: LiveBot,
}

impl Logger {
    /// Create a new Logger instance with a LiveBot.
    pub fn new(bot: LiveBot) -> Self {
        Logger { bot }
    }

    /// Returns the current date and time in a formatted tuple.
    fn get_formatted_datetime() -> (String, u8, u32, u32, u32, String) {
        let (month, day, _) = get_formatted_date();
        let (hours, mins, secs, is_pm) = get_formatted_time();
        let am_pm = if is_pm { "PM" } else { "AM" };
        (month.to_string(), day, hours, mins, secs, am_pm.to_string())
    }

    /// Logs a message with the given level and sends it to Telegram.
    pub fn log(&self, level: LogLevel, msg: &str) -> String {
        let (month, day, hours, mins, secs, am_pm) = Self::get_formatted_datetime();
        let formatted_msg = format!(
            "{} {}, {:02}:{:02}:{:02} {} | {:<8} | {}",
            day, month, hours, mins, secs, am_pm, level, msg
        );

        // Clone necessary data for the async block
        let bot_clone = self.bot.clone();
        let msg_clone = formatted_msg.clone();

        // Spawn the async task without awaiting it
        tokio::spawn(async move {
            if let Err(err) = bot_clone.send_message(&msg_clone).await {
                eprintln!("Failed to send message: {:?}", err);
            }
        });

        println!("{}", formatted_msg);
        formatted_msg
    }

    /// Logs a message with the `Success` log level.
    pub fn success(&self, msg: &str) -> String {
        self.log(LogLevel::Success, msg)
    }

    /// Logs a message with the `Info` log level.
    pub fn info(&self, msg: &str) -> String {
        self.log(LogLevel::Info, msg)
    }

    /// Logs a message with the `Debug` log level.
    pub fn debug(&self, msg: &str) -> String {
        self.log(LogLevel::Debug, msg)
    }

    /// Logs a message with the `Warning` log level.
    pub fn warning(&self, msg: &str) -> String {
        self.log(LogLevel::Warning, msg)
    }

    /// Logs a message with the `Error` log level.
    pub fn error(&self, msg: &str) -> String {
        self.log(LogLevel::Error, msg)
    }

    /// Logs a message with the `Critical` log level.
    pub fn critical(&self, msg: &str) -> String {
        self.log(LogLevel::Critical, msg)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum LogLevel {
    Success,
    Info,
    Debug,
    Warning,
    Error,
    Critical,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        const LEVEL_NAMES: [&str; 6] = ["SUCCESS", "INFO", "DEBUG", "WARNING", "ERROR", "CRITICAL"];
        let idx = *self as usize;
        write!(f, "{}", LEVEL_NAMES[idx])
    }
}
