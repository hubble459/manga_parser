use chrono::{
    DateTime, Datelike, Duration, Months, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc,
};
use regex::{Captures, Regex};

lazy_static::lazy_static! {
    static ref CLEAN_DATE: Regex = Regex::new(r"[^\w\d:.+\-]+").unwrap();
    static ref CLEAN_DATE_2: Regex = Regex::new(r"-{2,}").unwrap();
    static ref ORDINAL_NUMBER: Regex = Regex::new(r"(\d)(nd|st|rd|th)").unwrap();
    static ref DIGITS_ONLY: Regex = Regex::new(r"^\d+$").unwrap();
    static ref HAS_DIGITS: Regex = Regex::new(r"\d+").unwrap();
    static ref NONE_LETTER: Regex = Regex::new(r"\W").unwrap();
    static ref RELATIVE_DATE: Regex = Regex::new(r"(\d+)\s*(\w\w?)").unwrap();
}

/// Selects "1 year ago" -> "1y"

const STRING_FOR_CURRENT_DATE: [&str; 6] = ["now", "latest", "hot", "today", "current", "while"];

const DEFAULT_DATE_FORMATS: [&str; 20] = [
    // 2022-01-30T09:10:11.123Z
    "%Y-%m-%dT%H:%M:%S%.fZ",
    // 2022-01-30T09:10:11.123+0800
    "%Y-%m-%dT%H:%M:%S%.f%z",
    // 2022-01-30T09:10:11+0800
    "%Y-%m-%dT%H:%M:%S%z",
    // 2022-01-30T09:10:11Z
    "%Y-%m-%dT%H:%M:%SZ",
    // 2022-01-30T09:10:11
    "%Y-%m-%dT%H:%M:%S",
    // Juli 30 22 - 09:10
    "%B-%d-%y-%H:%M",
    // Juli 30 2022 09:10
    "%B-%d-%Y-%H:%M",
    // Oct 30 22 09:10:11
    "%b-%d-%y-%H:%M:%S",
    // Juli-30,22 09:10:11
    "%B-%d-%y-%H:%M:%S",
    // MISSING YEAR
    // Oct 30 09:10
    "%b-%d-%H:%M",
    // 30 Juli 09:10
    "%d-%B-%H:%M",
    // 30 Oct 09:10
    "%d-%b-%H:%M",
    // Oct 30 09:10
    "%b-%d-%H:%M",
    // Juli 30 2022
    "%B-%d-%Y",
    // Oct 30 2022
    "%b-%d-%Y",
    // Oct 30 22
    "%b-%d-%y",
    // 30 Juli 2022
    "%d-%B-%Y",
    // 2022.12.30
    "%Y.%m.%d",
    // 01 30 2022
    "%m-%d-%Y",
    // 30 01 2022
    "%d-%m-%Y",
];

pub fn try_parse_date(date: &str, date_formats: &[String]) -> Option<DateTime<Utc>> {
    if date.is_empty() {
        return None;
    }
    let date = date.trim();

    // Check if epoch millis [digits only]
    if DIGITS_ONLY.is_match(date) {
        let millis: i64 = date.parse().unwrap_or(-1);
        if millis == -1 {
            return None;
        }
        return NaiveDateTime::from_timestamp_millis(millis)
            .map(|datetime| DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc));
    }

    let now = Utc::now();

    // Check if text only
    if !HAS_DIGITS.is_match(date) {
        let date = NONE_LETTER.replace_all(date, "").to_ascii_lowercase();

        for current_string in STRING_FOR_CURRENT_DATE {
            if date.contains(current_string) {
                return Some(now);
            }
        }
        if date.contains("yesterday") {
            return now.checked_sub_signed(Duration::days(1));
        }
        if date.contains("week") {
            return now.checked_sub_signed(Duration::weeks(1));
        }
        if date.contains("month") {
            return now.checked_sub_months(Months::new(1));
        }
        if date.contains("year") {
            return now.checked_sub_signed(Duration::days(365));
        }
        return None;
    }

    // Check if date format (multiple digits)
    if HAS_DIGITS.find_iter(date).count() > 1 {
        let date = CLEAN_DATE.replace_all(date, "-").into_owned();
        let date = CLEAN_DATE_2.replace_all(&date, "-");
        let date = &ORDINAL_NUMBER
            .replace_all(&date, |cap: &Captures| cap[1].to_owned())
            .into_owned();
        for format in date_formats.iter().map(String::as_str).chain(DEFAULT_DATE_FORMATS) {
            let datetime = NaiveDateTime::parse_from_str(date, format);
            if let Ok(date) = datetime {
                return Some(Utc.from_utc_datetime(&date));
            } else if let Err(e) = datetime {
                if e.kind() == chrono::format::ParseErrorKind::NotEnough {
                    // Retry with year
                    let datetime = NaiveDateTime::parse_from_str(
                        &format!("{date}-{}", now.year()),
                        &format!("{format}-%Y"),
                    );
                    if let Ok(date) = datetime {
                        return Some(Utc.from_utc_datetime(&date));
                    } else if let Err(e) = datetime {
                        // If missing time
                        if e.kind() == chrono::format::ParseErrorKind::NotEnough {
                            // Parse only date
                            let date = NaiveDate::parse_from_str(date, format);
                            if let Ok(date) = date {
                                // Return date time with default time
                                return Some(
                                    Utc.from_utc_datetime(&date.and_time(NaiveTime::default())),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // Check if relative
    // e.g. "1 year ago"
    let binding = date.to_ascii_lowercase();
    let captures = RELATIVE_DATE.captures(&binding);
    if let Some(captures) = captures {
        // Assume that it always is [number][type] ago
        // like 1 year ago
        let amount: i64 = captures.get(1).unwrap().as_str().parse().unwrap_or(1);
        let rel_type = captures.get(2).unwrap().as_str();

        // Minutes
        if rel_type == "mi" {
            return Some(now - Duration::minutes(amount));
        }

        let rel_type = rel_type.chars().next().unwrap();

        return match rel_type {
            's' => Some(now - Duration::seconds(amount)),
            'h' => Some(now - Duration::hours(amount)),
            'd' => Some(now - Duration::days(amount)),
            'w' => Some(now - Duration::weeks(amount)),
            'm' => Some(now - Months::new(amount as u32)),
            'y' => Some(now - Duration::days(365 * amount)),
            _ => None,
        };
    }

    // No date detected
    None
}
