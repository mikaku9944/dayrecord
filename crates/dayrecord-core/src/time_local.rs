//! Wall-clock helpers: stored timestamps are UTC; habit/rhythm analysis uses local time.

use chrono::{DateTime, Datelike, Local, Timelike, Utc};

pub fn local_day_string(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local).format("%Y-%m-%d").to_string()
}

pub fn local_hour(dt: DateTime<Utc>) -> u8 {
    dt.with_timezone(&Local).hour() as u8
}

pub fn local_weekday(dt: DateTime<Utc>) -> u8 {
    dt.with_timezone(&Local)
        .weekday()
        .num_days_from_monday() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn local_parts_use_machine_timezone() {
        let utc = Utc.with_ymd_and_hms(2026, 6, 10, 1, 0, 0).unwrap();
        let local = utc.with_timezone(&Local);
        assert_eq!(local_hour(utc), local.hour() as u8);
        assert_eq!(local_weekday(utc), local.weekday().num_days_from_monday() as u8);
        assert_eq!(local_day_string(utc), local.format("%Y-%m-%d").to_string());
    }
}
