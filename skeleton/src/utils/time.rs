use chrono::{DateTime, Datelike, Local, Timelike};
use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};
// Precompute cumulative days for leap/non-leap years for O(1) month/day lookups


/// Generates current timestamp in milliseconds (optimized)
#[inline(always)]
pub fn generate_timestamp() -> Result<u64, SystemTimeError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
}

#[inline(always)]
pub fn get_formatted_time() -> (u32, u32, u32, bool) {
    let now: DateTime<Local> = Local::now();
    let hour_12 = now.hour12().1; // Returns (is_pm, hour)
    (hour_12, now.minute(), now.second(), now.hour12().0)
}

/// Gets formatted date string (e.g., "Jan 31 2023")
#[inline(always)]
pub fn get_formatted_date() -> (String, u8, i32) {
    let now: DateTime<Local> = Local::now();
    (
        now.format("%b").to_string(), // Short month name (e.g., "Jan")
        now.day() as u8,
        now.year(),
    )
}
