use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

// Precompute cumulative days for leap/non-leap years for O(1) month/day lookups
const CUMULATIVE_DAYS: [[u16; 12]; 2] = [
    // Non-leap year cumulative days
    [31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365],
    // Leap year cumulative days
    [31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335, 366],
];

/// Generates current timestamp in milliseconds (optimized)
#[inline(always)]
pub fn generate_timestamp() -> Result<u64, SystemTimeError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
}

/// Gets 12-hour formatted time (optimized integer math)
#[inline(always)]
pub fn get_formatted_time() -> (u8, u8, u8, bool) {
    let ts = generate_timestamp().unwrap() / 1000; // Seconds since epoch
    let secs = (ts % 60) as u8;
    let mins = ((ts / 60) % 60) as u8;
    let hours = ((ts / 3600) % 24) as u8;
    let is_pm = hours >= 12;

    let hours_12 = match hours % 12 {
        0 => 12,
        h => h,
    };

    (hours_12, mins, secs, is_pm)
}

/// Gets formatted date using O(1) year calculation (optimized)
#[inline(always)]
pub fn get_formatted_date() -> (&'static str, u8, u64) {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let days = generate_timestamp().unwrap() / 86400_000; // Days since epoch
    let (year, remaining_days) = compute_year(days);
    let (month, day) = compute_month_day(remaining_days as u16, is_leap_year(year as u16));

    (MONTHS[month as usize - 1], day as u8, year)
}

/// O(1) year computation using mathematical formula
fn compute_year(mut days: u64) -> (u64, u64) {
    // Offset to handle leap years mathematically
    let (mut year, cycles_400) = (1970, days / 146_097);
    days -= cycles_400 * 146_097; // Days in 400-year cycle
    year += cycles_400 * 400;

    // Compute remaining years using 4-year cycles
    let cycles_4 = days.min(1459) / 1461; // Days in 4-year cycle
    days -= cycles_4 * 1461;
    year += cycles_4 * 4;

    // Handle remaining years (max 3 iterations)
    while days >= 365 + is_leap_year(year as u16) as u64 {
        days -= 365 + is_leap_year(year as u16) as u64;
        year += 1;
    }

    (year, days)
}

/// O(1) month/day calculation using binary search
fn compute_month_day(days: u16, is_leap: bool) -> (u8, u16) {
    let table = CUMULATIVE_DAYS[is_leap as usize];
    let (mut low, mut high) = (0, 11);
    let mut month = 0;

    // Binary search for month
    while low <= high {
        let mid = (low + high) / 2;
        if days <= table[mid] {
            month = mid;
            high = mid - 1;
        } else {
            low = mid + 1;
        }
    }

    let day = days - if month > 0 { table[month - 1] } else { 0 };
    ((month + 1) as u8, day as u16 + 1)
}

/// Optimized leap year check
#[inline(always)]
fn is_leap_year(year: u16) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}
