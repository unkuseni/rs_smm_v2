use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

    /// Generates the current timestamp in milliseconds since the Unix epoch.
    ///
    /// # Errors
    ///
    /// Returns a `SystemTimeError` if there is an error generating the timestamp.
#[inline(always)]
pub fn generate_timestamp() -> Result<u64, SystemTimeError> {
    let time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
    Ok(time)
}

    /// Returns the current time in 12-hour format as a tuple of `(hours, minutes, seconds, is_pm)`.
    ///
    /// # Returns
    ///
    /// A tuple of four values:
    ///
    /// * `hours`: The hour in 12-hour format (1-12).
    /// * `minutes`: The minutes in the current hour (0-59).
    /// * `seconds`: The seconds in the current minute (0-59).
    /// * `is_pm`: A boolean indicating whether the current time is in the PM (true) or AM (false).
#[inline(always)]
pub fn get_formatted_time() -> (u64, u64, u64, bool) {
    let timestamp = generate_timestamp().unwrap() / 1000; // Convert to seconds
    let total_seconds = timestamp % 86400; // Seconds in current day (86400 = 24*60*60)

    // Extract hours, minutes, seconds
    let hours = total_seconds / 3600;
    let mins = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;

    // Convert to 12-hour format
    let is_pm = hours >= 12;
    let hours_12 = if hours == 0 {
        12
    } else if hours > 12 {
        hours - 12
    } else {
        hours
    };

    (hours_12, mins, secs, is_pm)
}

    /// Returns the current date in the format `(month_str, day, year)` as a tuple of three values:
    ///
    /// * `month_str`: A string representing the month of the year (e.g. "Jan", "Feb", etc.).
    /// * `day`: The day of the month (1-31).
    /// * `year`: The year as a 4-digit number (1970-9999).
#[inline(always)]
pub fn get_formatted_date() -> (String, u64, u64) {
    let timestamp = generate_timestamp().unwrap() / 1000; // Convert to seconds
    let days_since_epoch = timestamp / 86400;
    let month_str = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    // More accurate date calculation
    let mut year = 1970;
    let mut remaining_days = days_since_epoch;

    // Account for leap years
    while remaining_days >= 365 {
        if is_leap_year(year) && remaining_days >= 366 {
            remaining_days -= 366;
            year += 1;
        } else if !is_leap_year(year) && remaining_days >= 365 {
            remaining_days -= 365;
            year += 1;
        } else {
            break;
        }
    }

    // Calculate month and day
    let (month, day) = days_to_month_day(remaining_days as u32, is_leap_year(year));

    (month_str[month as usize - 1].into(), day, year as u64)
}

    /// Checks if a given year is a leap year according to the Gregorian calendar rules.
    ///
    /// A year is a leap year if it is divisible by 4, except for end-of-century years which
    /// must be divisible by 400. This means that the year 2000 was a leap year, although the
    /// years 1700, 1800, and 1900 were not.
#[inline(always)]
fn is_leap_year(year: u64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

    /// Converts a given number of days into a month and day of the month.
    ///
    /// The given `days` value is the number of days since the Unix epoch (January 1, 1970).
    /// The `is_leap` parameter indicates whether the year is a leap year or not.
    ///
    /// The returned value is a tuple containing the month and day of the month, where the month
    /// is an integer from 1 to 12 and the day is an integer from 1 to 31.
    ///
    /// This function is designed to be used with the result of `get_formatted_date`, which returns
    /// the number of days since the Unix epoch and whether the year is a leap year or not.
    ///
    /// The function handles leap years according to the Gregorian calendar rules.
#[inline(always)]
fn days_to_month_day(days: u32, is_leap: bool) -> (u64, u64) {
    const DAYS_IN_MONTH: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut remaining_days = days;
    let mut month = 1;

    for (i, &days_in_month) in DAYS_IN_MONTH.iter().enumerate() {
        let actual_days = if i == 1 && is_leap {
            days_in_month + 1
        } else {
            days_in_month
        };

        if remaining_days < actual_days {
            return (month, remaining_days as u64 + 1);
        }

        remaining_days -= actual_days;
        month += 1;
    }

    (12, 31) // Fallback for any edge cases
}
