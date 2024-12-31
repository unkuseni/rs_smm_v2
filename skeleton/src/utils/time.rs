use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

#[inline(always)]
pub fn generate_timestamp() -> Result<u64, SystemTimeError> {
    let time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
    Ok(time)
}

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

#[inline(always)]
fn is_leap_year(year: u64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

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
