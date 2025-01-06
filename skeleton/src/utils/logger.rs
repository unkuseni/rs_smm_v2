use super::time::{get_formatted_date, get_formatted_time};
use std::fmt;

#[derive(Debug, Clone)]
pub struct Logger;

impl Logger {
    /// Returns the current date and time in a formatted tuple.
    ///
    /// The returned tuple contains the month, day, hour, minute, second, and AM/PM indicator.
    ///
    /// # Examples
    ///
    /// 
    fn get_formatted_datetime() -> (String, u64, u64, u64, u64, &'static str) {
        let (month, day, _) = get_formatted_date();
        let (hours, mins, secs, is_pm) = get_formatted_time();
        let am_pm = if is_pm { "PM" } else { "AM" };
        (month, day, hours, mins, secs, am_pm)
    }

    /// Logs a message with the given level and message.
    ///
    /// The message is formatted to include the current date, time, and level.
    /// The message is then printed to the console and returned as a `String`.
    ///
    /// # Arguments
    ///
    /// * `level` - The log level of the message
    /// * `msg` - The log message to be printed
    ///
    /// # Examples
    ///
    /// 
    /// 
    pub fn log(level: LogLevel, msg: &str) -> String {
        let (month, day, hours, mins, secs, am_pm) = Self::get_formatted_datetime();
        let formatted_msg = format!(
            "{} {}, {:02}:{:02}:{:02} {} | {:<8} | {}",
            day, month, hours, mins, secs, am_pm, level, msg
        );
        // Print to console
        println!("{}", formatted_msg);

        // Return the formatted message
        formatted_msg
    }

    /// Logs a message with the `Success` log level.
    ///
    /// Equivalent to calling `log(LogLevel::Success, msg)`.
    pub fn success(&self, msg: &str) -> String {
        Self::log(LogLevel::Success, msg)
    }

    /// Logs a message with the `Info` log level.
    ///
    /// Equivalent to calling `log(LogLevel::Info, msg)`.
    pub fn info(&self, msg: &str) -> String {
        Self::log(LogLevel::Info, msg)
    }

    /// Logs a message with the `Debug` log level.
    ///
    /// Equivalent to calling `log(LogLevel::Debug, msg)`.
    pub fn debug(&self, msg: &str) -> String {
        Self::log(LogLevel::Debug, msg)
    }

    /// Logs a message with the `Warning` log level.
    ///
    /// Equivalent to calling `log(LogLevel::Warning, msg)`.
    pub fn warning(&self, msg: &str) -> String {
        Self::log(LogLevel::Warning, msg)
    }

    /// Logs a message with the `Error` log level.
    ///
    /// Equivalent to calling `log(LogLevel::Error, msg)`.
    pub fn error(&self, msg: &str) -> String {
        Self::log(LogLevel::Error, msg)
    }

    /// Logs a message with the `Critical` log level.
    ///
    /// Equivalent to calling `log(LogLevel::Critical, msg)`.
    pub fn critical(&self, msg: &str) -> String {
        Self::log(LogLevel::Critical, msg)
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
    /// Formats the LogLevel as a string.
    ///
    /// This method is used when writing a LogLevel to a stream with the
    /// `Display` trait. The output is the name of the LogLevel in uppercase
    /// with the first letter capitalized.
    ///
    /// # Examples
    ///
    /// 
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        const LEVEL_NAMES: [&str; 6] = ["SUCCESS", "INFO", "DEBUG", "WARNING", "ERROR", "CRITICAL"];
        let idx = *self as usize;
        write!(f, "{}", LEVEL_NAMES[idx])
    }
}
