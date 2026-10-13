//! UTC timestamps with second precision.

use std::fmt;
use std::str::FromStr;

use crate::error::{ModelError, Result};

const SECONDS_PER_DAY: i64 = 86_400;

/// A UTC instant at second precision, with a canonical text form `YYYY-MM-DDThh:mm:ssZ`.
///
/// Years 0000 to 9999 are representable. The clock is injected by callers (see `Clock` in
/// `functor-core`), so this type never reads the system time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Timestamp(i64);

// Hinnant's civil-calendar algorithms.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if is_leap(year) => 29,
        _ => 28,
    }
}

impl Timestamp {
    /// The earliest representable instant, `0000-01-01T00:00:00Z`.
    pub const MIN: Self = Self(-62_167_219_200);
    /// The latest representable instant, `9999-12-31T23:59:59Z`.
    pub const MAX: Self = Self(253_402_300_799);

    /// Builds a timestamp from seconds since the Unix epoch.
    ///
    /// # Errors
    /// Fails when the instant is outside years 0000 to 9999.
    pub fn from_unix_seconds(seconds: i64) -> Result<Self> {
        if (Self::MIN.0..=Self::MAX.0).contains(&seconds) {
            Ok(Self(seconds))
        } else {
            Err(ModelError::new(
                "timestamp",
                "must be within years 0000 to 9999",
            ))
        }
    }

    /// Seconds since the Unix epoch.
    #[must_use]
    pub fn unix_seconds(self) -> i64 {
        self.0
    }

    /// Parses `YYYY-MM-DDThh:mm:ssZ` exactly.
    ///
    /// # Errors
    /// Fails when the text has another shape or names an impossible date or time.
    pub fn parse(input: &str) -> Result<Self> {
        const SHAPE: &str = "must have the form YYYY-MM-DDThh:mm:ssZ";
        let bytes = input.as_bytes();
        let separators = [
            (4, b'-'),
            (7, b'-'),
            (10, b'T'),
            (13, b':'),
            (16, b':'),
            (19, b'Z'),
        ];
        if bytes.len() != 20 || separators.iter().any(|&(i, c)| bytes[i] != c) {
            return Err(ModelError::new("timestamp", SHAPE));
        }
        let number = |range: std::ops::Range<usize>| -> Result<i64> {
            let part = &bytes[range];
            if part.iter().all(u8::is_ascii_digit) {
                Ok(part.iter().fold(0, |acc, d| acc * 10 + i64::from(d - b'0')))
            } else {
                Err(ModelError::new("timestamp", SHAPE))
            }
        };
        let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
        let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
        if !(1..=12).contains(&month)
            || day < 1
            || day > days_in_month(year, month)
            || hour > 23
            || minute > 59
            || second > 59
        {
            return Err(ModelError::new(
                "timestamp",
                "names an impossible date or time",
            ));
        }
        let days = days_from_civil(year, month, day);
        Ok(Self(
            days * SECONDS_PER_DAY + hour * 3600 + minute * 60 + second,
        ))
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let days = self.0.div_euclid(SECONDS_PER_DAY);
        let rem = self.0.rem_euclid(SECONDS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        write!(
            f,
            "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
            rem / 3600,
            rem % 3600 / 60,
            rem % 60
        )
    }
}

impl FromStr for Timestamp {
    type Err = ModelError;

    fn from_str(input: &str) -> Result<Self> {
        Self::parse(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_instants() {
        assert_eq!(
            Timestamp::parse("1970-01-01T00:00:00Z")
                .unwrap()
                .unix_seconds(),
            0
        );
        assert_eq!(
            Timestamp::parse("2026-10-12T18:00:00Z")
                .unwrap()
                .unix_seconds(),
            1_791_828_000
        );
        assert_eq!(
            Timestamp::parse("0000-01-01T00:00:00Z").unwrap(),
            Timestamp::MIN
        );
        assert_eq!(
            Timestamp::parse("9999-12-31T23:59:59Z").unwrap(),
            Timestamp::MAX
        );
    }

    #[test]
    fn display_round_trips_at_extremes_and_leap_days() {
        for text in [
            "0000-01-01T00:00:00Z",
            "1969-12-31T23:59:59Z",
            "2000-02-29T12:30:45Z",
            "2024-02-29T00:00:00Z",
            "9999-12-31T23:59:59Z",
        ] {
            assert_eq!(Timestamp::parse(text).unwrap().to_string(), text);
        }
    }

    #[test]
    fn rejects_bad_shapes_and_impossible_dates() {
        for bad in [
            "",
            "2026-10-12",
            "2026-10-12 18:00:00Z",
            "2026-10-12T18:00:00",
            "2026-10-12T18:00:00+00:00",
            "2026-13-01T00:00:00Z",
            "2026-00-01T00:00:00Z",
            "2026-02-29T00:00:00Z",
            "1900-02-29T00:00:00Z",
            "2026-04-31T00:00:00Z",
            "2026-10-12T24:00:00Z",
            "2026-10-12T00:60:00Z",
            "2026-10-12T00:00:60Z",
            "2026-1a-12T00:00:00Z",
        ] {
            assert!(Timestamp::parse(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn range_is_enforced() {
        assert!(Timestamp::from_unix_seconds(Timestamp::MAX.unix_seconds() + 1).is_err());
        assert!(Timestamp::from_unix_seconds(Timestamp::MIN.unix_seconds() - 1).is_err());
        assert!(Timestamp::from_unix_seconds(0).is_ok());
    }
}
