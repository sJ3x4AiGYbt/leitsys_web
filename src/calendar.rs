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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_grid_always_has_42_cells() {
        assert_eq!(month_grid(2026, Month::February).len(), 42);
        assert_eq!(month_grid(2026, Month::September).len(), 42);
        assert_eq!(month_grid(2024, Month::February).len(), 42); // leap year
    }

    #[test]
    fn month_grid_marks_in_month_correctly() {
        let year = 2026;
        let month = Month::September;
        let days_in_month = Date::from_calendar_date(year, month.next(), 1).unwrap() - Duration::days(1);
        let expected_in_month_count = days_in_month.day() as usize;

        let grid = month_grid(year, month);
        let in_month_count = grid.iter().filter(|c| c.in_month).count();
        assert_eq!(in_month_count, expected_in_month_count);

        for cell in &grid {
            let belongs_to_requested_month = cell.date.year() == year && cell.date.month() == month;
            assert_eq!(cell.in_month, belongs_to_requested_month);
        }
    }

    #[test]
    fn previous_month_rolls_over_the_year_boundary() {
        assert_eq!(previous_month(2026, Month::January), (2025, Month::December));
        assert_eq!(previous_month(2026, Month::September), (2026, Month::August));
    }

    #[test]
    fn next_month_rolls_over_the_year_boundary() {
        assert_eq!(next_month(2026, Month::December), (2027, Month::January));
        assert_eq!(next_month(2026, Month::September), (2026, Month::October));
    }

    #[test]
    fn parse_date_reads_the_date_prefix_of_an_rfc3339_timestamp() {
        assert_eq!(
            parse_date("2026-09-17T10:00:00.123456Z"),
            Some(Date::from_calendar_date(2026, Month::September, 17).unwrap())
        );
    }

    #[test]
    fn parse_date_rejects_garbage() {
        assert_eq!(parse_date("not a date"), None);
        assert_eq!(parse_date("2026-13-40T00:00:00Z"), None);
    }
}
