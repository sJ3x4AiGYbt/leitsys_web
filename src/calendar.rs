use time::{Date, Duration, Month, OffsetDateTime};

pub fn today() -> Date {
    OffsetDateTime::now_utc().date()
}

/// Parses the `YYYY-MM-DD...` prefix of an RFC 3339 timestamp (as sent by
/// the backend) into a calendar date, ignoring the time-of-day component.
pub fn parse_date(rfc3339: &str) -> Option<Date> {
    let (year, rest) = rfc3339.split_once('-')?;
    let (month, rest) = rest.split_once('-')?;
    let day = rest.get(..2)?;

    Date::from_calendar_date(
        year.parse().ok()?,
        Month::try_from(month.parse::<u8>().ok()?).ok()?,
        day.parse().ok()?,
    )
    .ok()
}

#[derive(Clone, Copy, PartialEq)]
pub struct DayCell {
    pub date: Date,
    pub in_month: bool,
}

/// A 6-week (42 day) grid covering `year`/`month`, padded with the trailing
/// days of the previous/next month so every row is a full Monday-to-Sunday
/// week.
pub fn month_grid(year: i32, month: Month) -> Vec<DayCell> {
    let first = Date::from_calendar_date(year, month, 1).expect("day 1 is always valid");
    let start = first - Duration::days(first.weekday().number_days_from_monday().into());

    (0..42)
        .map(|i| {
            let date = start + Duration::days(i);
            DayCell { date, in_month: date.year() == year && date.month() == month }
        })
        .collect()
}

pub fn previous_month(year: i32, month: Month) -> (i32, Month) {
    if month == Month::January { (year - 1, Month::December) } else { (year, month.previous()) }
}

pub fn next_month(year: i32, month: Month) -> (i32, Month) {
    if month == Month::December { (year + 1, Month::January) } else { (year, month.next()) }
}
