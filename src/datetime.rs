/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs LLC <hello@stalw.art>
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use crate::fields::{date, first_field};
use std::{cmp::Ordering, fmt};

/// Three-letter day names, Sunday first, indexed by
/// [`DateTime::day_of_week`].
pub static DOW: &[&str] = &["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
/// Three-letter month names, January first.
pub static MONTH: &[&str] = &[
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// An RFC 5322 date and time with its zone offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DateTime {
    /// Year, four digits.
    pub year: u16,
    /// Month, 1 to 12.
    pub month: u8,
    /// Day of the month, 1 to 31.
    pub day: u8,
    /// Hour, 0 to 23.
    pub hour: u8,
    /// Minute, 0 to 59.
    pub minute: u8,
    /// Second, 0 to 59.
    pub second: u8,
    /// Whether the zone offset is negative (west of UTC).
    pub tz_before_gmt: bool,
    /// Hours of the zone offset.
    pub tz_hour: u8,
    /// Minutes of the zone offset.
    pub tz_minute: u8,
}

const RFC3339_WIDTHS: [u32; 8] = [4, 2, 2, 2, 2, 2, 2, 2];

impl DateTime {
    /// Parses an RFC 5322 date with the header date parser.
    pub fn parse_rfc822(value: &str) -> Option<Self> {
        let bytes = value.as_bytes();
        date::parse_bytes(bytes.get(first_field(bytes, 0..bytes.len()))?)
    }

    /// Parses an RFC 3339 date.
    pub fn parse_rfc3339(value: &str) -> Option<Self> {
        let mut pos = 0usize;
        let mut parts = [0u32; 8];
        let mut widths = RFC3339_WIDTHS;
        let mut skip_digits = false;
        let mut is_plus = true;

        for &ch in value.as_bytes() {
            match ch {
                b'0'..=b'9' if !skip_digits => {
                    let width = widths.get_mut(pos)?;
                    if *width == 0 {
                        return None;
                    }
                    *width -= 1;
                    *parts.get_mut(pos)? += (ch - b'0') as u32 * 10u32.pow(*width);
                }
                b'-' if pos <= 1 => pos += 1,
                b'-' if pos == 5 => {
                    pos += 1;
                    is_plus = false;
                    skip_digits = false;
                }
                b'T' if pos == 2 => pos += 1,
                b':' if matches!(pos, 3 | 4 | 6) => pos += 1,
                b'+' if pos == 5 => {
                    pos += 1;
                    skip_digits = false;
                }
                b'.' if pos == 5 => skip_digits = true,
                b'-' | b'T' | b':' | b'+' | b'.' => return None,
                _ => (),
            }
        }

        let [year, month, day, hour, minute, second, tz_hour, tz_minute] = parts;
        (pos >= 5).then_some(DateTime {
            year: year as u16,
            month: month as u8,
            day: day as u8,
            hour: hour as u8,
            minute: minute as u8,
            second: second as u8,
            tz_hour: tz_hour as u8,
            tz_minute: tz_minute as u8,
            tz_before_gmt: !is_plus,
        })
    }

    fn zone_sign(&self) -> &'static str {
        if self.tz_before_gmt && (self.tz_hour > 0 || self.tz_minute > 0) {
            "-"
        } else {
            "+"
        }
    }

    /// Formats the date as RFC 5322.
    pub fn to_rfc822(&self) -> String {
        format!(
            "{}, {} {} {:04} {:02}:{:02}:{:02} {}{:02}{:02}",
            DOW.get(self.day_of_week() as usize)
                .copied()
                .unwrap_or_default(),
            self.day,
            MONTH
                .get(self.month.saturating_sub(1) as usize)
                .copied()
                .unwrap_or_default(),
            self.year,
            self.hour,
            self.minute,
            self.second,
            self.zone_sign(),
            self.tz_hour,
            self.tz_minute
        )
    }

    /// Formats the date as RFC 3339.
    pub fn to_rfc3339(&self) -> String {
        if self.tz_hour != 0 || self.tz_minute != 0 {
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}{:02}:{:02}",
                self.year,
                self.month,
                self.day,
                self.hour,
                self.minute,
                self.second,
                self.zone_sign(),
                self.tz_hour,
                self.tz_minute
            )
        } else {
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                self.year, self.month, self.day, self.hour, self.minute, self.second,
            )
        }
    }

    /// Whether every field is in range (years 1900 to 3000).
    pub fn is_valid(&self) -> bool {
        (0..=23).contains(&self.tz_hour)
            && (1900..=3000).contains(&self.year)
            && (0..=59).contains(&self.tz_minute)
            && (1..=12).contains(&self.month)
            && (1..=31).contains(&self.day)
            && (0..=23).contains(&self.hour)
            && (0..=59).contains(&self.minute)
            && (0..=59).contains(&self.second)
    }

    /// Seconds since the Unix epoch, applying the zone offset.
    pub fn to_timestamp(&self) -> i64 {
        self.to_timestamp_local()
            + ((self.tz_hour as i64 * 3600 + self.tz_minute as i64 * 60)
                * if self.tz_before_gmt { 1 } else { -1 })
    }

    /// Seconds since the Unix epoch, ignoring the zone offset.
    pub fn to_timestamp_local(&self) -> i64 {
        let month = self.month as u32;
        let year_base = 4800;
        let m_adj = month.wrapping_sub(3);
        let carry = i64::from(m_adj > month);
        let adjust = if carry > 0 { 12 } else { 0 };
        let y_adj = self.year as i64 + year_base - carry;
        let month_days = ((m_adj.wrapping_add(adjust)) * 62719 + 769) / 2048;
        let leap_days = y_adj / 4 - y_adj / 100 + y_adj / 400;
        (y_adj * 365 + leap_days + month_days as i64 + (self.day as i64 - 1) - 2472632) * 86400
            + self.hour as i64 * 3600
            + self.minute as i64 * 60
            + self.second as i64
    }

    /// Creates a UTC date from seconds since the Unix epoch.
    pub fn from_timestamp(timestamp: i64) -> Self {
        let (z, seconds) = (
            (timestamp.div_euclid(86400)) + 719468,
            timestamp.rem_euclid(86400),
        );
        let era: i64 = (if z >= 0 { z } else { z - 146096 }) / 146097;
        let doe: u64 = (z - era * 146097) as u64;
        let yoe: u64 = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y: i64 = (yoe as i64) + era * 400;
        let doy: u64 = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d: u64 = doy - (153 * mp + 2) / 5 + 1;
        let m: u64 = if mp < 10 { mp + 3 } else { mp - 9 };
        let (h, mn, s) = (seconds / 3600, (seconds / 60) % 60, seconds % 60);

        DateTime {
            year: (y + i64::from(m <= 2)) as u16,
            month: m as u8,
            day: d as u8,
            hour: h as u8,
            minute: mn as u8,
            second: s as u8,
            tz_before_gmt: false,
            tz_hour: 0,
            tz_minute: 0,
        }
    }

    /// Day of the week, 0 (Sunday) to 6 (Saturday).
    pub fn day_of_week(&self) -> u8 {
        (((self.to_timestamp_local() as f64 / 86400.0).floor() as i64 + 4).rem_euclid(7)) as u8
    }

    /// Julian day number.
    pub fn julian_day(&self) -> i64 {
        let day = self.day as i64;
        let (month, year) = if self.month > 2 {
            ((self.month - 3) as i64, self.year as i64)
        } else {
            ((self.month + 9) as i64, (self.year as i64 - 1))
        };

        let c = year / 100;
        c * 146097 / 4 + (year - c * 100) * 1461 / 4 + (month * 153 + 2) / 5 + day + 1721119
    }

    /// The same instant in a zone `tz` seconds away from UTC. Offsets too
    /// large for the fields saturate instead of overflowing.
    pub fn to_timezone(&self, tz: i64) -> DateTime {
        let mut dt = DateTime::from_timestamp(self.to_timestamp().saturating_add(tz));
        dt.tz_before_gmt = tz < 0;
        let offset = tz.unsigned_abs();
        dt.tz_hour = u8::try_from(offset / 3600).unwrap_or(u8::MAX);
        dt.tz_minute = ((offset % 3600) / 60) as u8;
        dt
    }
}

impl PartialOrd for DateTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DateTime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_timestamp().cmp(&other.to_timestamp())
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(&self.to_rfc3339())
    }
}

impl From<DateTime> for i64 {
    fn from(value: DateTime) -> Self {
        value.to_timestamp()
    }
}

#[cfg(test)]
mod tests {
    use super::DateTime;

    #[test]
    fn rfc3339_round_trip() {
        for input in [
            "2021-11-20T14:22:01-08:00",
            "1969-02-13T23:32:00-03:30",
            "2004-06-28T23:43:45Z",
        ] {
            let date = DateTime::parse_rfc3339(input).expect("valid");
            assert_eq!(date.to_rfc3339(), input);
        }
        assert_eq!(
            DateTime::parse_rfc3339("2004-06-28T23:43:45.000Z").map(|d| d.to_rfc3339()),
            Some("2004-06-28T23:43:45Z".to_string())
        );
        assert!(DateTime::parse_rfc3339("2004-06-28+23:43:45").is_none());
    }

    #[test]
    fn timestamps() {
        let date = DateTime::parse_rfc3339("2021-11-20T14:22:01-08:00").expect("valid");
        assert_eq!(date.to_timestamp(), 1637446921);
        assert_eq!(
            DateTime::from_timestamp(1637446921).to_rfc3339(),
            "2021-11-20T22:22:01Z"
        );
        assert_eq!(date.to_rfc822(), "Sat, 20 Nov 2021 14:22:01 -0800");
        assert_eq!(date.day_of_week(), 6);
        assert!(date.is_valid());
        assert_eq!(i64::from(date), 1637446921);
        assert_eq!(
            date.to_timezone(3600).to_rfc3339(),
            "2021-11-20T23:22:01+01:00"
        );
        assert_eq!(date.julian_day(), 2459539);
    }

    #[test]
    fn extreme_timezones_saturate() {
        let date = DateTime::parse_rfc3339("2021-11-20T14:22:01-08:00").expect("valid");
        for tz in [i64::MIN, i64::MIN + 1, i64::MAX] {
            let moved = date.to_timezone(tz);
            assert_eq!(moved.tz_before_gmt, tz < 0);
            assert_eq!(moved.tz_hour, u8::MAX);
            assert!(moved.tz_minute < 60);
        }
        let moved = date.to_timezone(-(5 * 3600 + 30 * 60));
        assert_eq!(moved.to_rfc3339(), "2021-11-20T16:52:01-05:30");
    }
}
