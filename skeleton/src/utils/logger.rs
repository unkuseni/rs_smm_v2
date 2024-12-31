use super::time::{get_formatted_date, get_formatted_time};
use std::fmt;

#[derive(Debug, Clone)]
pub struct Logger;

impl Logger {
    pub fn log(level: LogLevel, msg: &str) -> String {
        let (month, day, _) = get_formatted_date();
        let (hours, mins, secs, is_pm) = get_formatted_time();
        let am_pm = if is_pm { "PM" } else { "AM" };

        let formatted_msg = format!(
            "{} {}, {:02}:{:02}:{:02} {} | {:<8} | {}",
            day, month, hours, mins, secs, am_pm, level, msg
        );
        // Print to console
        println!("{}", formatted_msg);

        // Return the formatted message
        formatted_msg
    }

    pub fn success(&self, msg: &str) -> String {
        Self::log(LogLevel::Success, msg)
    }

    pub fn info(&self, msg: &str) -> String {
        Self::log(LogLevel::Info, msg)
    }

    pub fn debug(&self, msg: &str) -> String {
        Self::log(LogLevel::Debug, msg)
    }

    pub fn warning(&self, msg: &str) -> String {
        Self::log(LogLevel::Warning, msg)
    }

    pub fn error(&self, msg: &str) -> String {
        Self::log(LogLevel::Error, msg)
    }

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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        const LEVEL_NAMES: [&str; 6] = ["SUCCESS", "INFO", "DEBUG", "WARNING", "ERROR", "CRITICAL"];
        let idx = *self as usize;
        write!(f, "{}", LEVEL_NAMES[idx])
    }
}
