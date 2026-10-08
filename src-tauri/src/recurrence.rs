//! Repeating events. A series is stored once (its first occurrence plus a rule) and
//! expanded into individual occurrences when a date range is queried.

use chrono::{Datelike, Duration, Months, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repeat {
    pub freq: Freq,
    /// Every `interval` days/weeks/months/years.
    #[serde(default = "one")]
    pub interval: u32,
    /// Weekly only: 0 = Sunday … 6 = Saturday. Empty means the first occurrence's weekday.
    #[serde(default)]
    pub weekdays: Vec<u8>,
    /// Last date an occurrence may start on (inclusive).
    #[serde(default)]
    pub until: Option<NaiveDate>,
    /// Total number of occurrences in the series.
    #[serde(default)]
    pub count: Option<u32>,
}

fn one() -> u32 {
    1
}

/// Upper bound on generated candidates, to stay fast for very long series.
const MAX_STEPS: usize = 50_000;

/// Start dates of the series' occurrences that fall within `from..=to`.
/// `first` is the start date of the first occurrence.
pub fn occurrences(first: NaiveDate, rule: &Repeat, from: NaiveDate, to: NaiveDate) -> Vec<NaiveDate> {
    let interval = rule.interval.max(1);
    let last = match rule.until {
        Some(until) => until.min(to),
        None => to,
    };
    let mut out = Vec::new();
    let mut produced: u32 = 0;
    let mut emit = |d: NaiveDate, out: &mut Vec<NaiveDate>| -> bool {
        // Returns false once the series is over.
        if d > last || rule.count.is_some_and(|c| produced >= c) {
            return false;
        }
        produced += 1;
        if d >= from {
            out.push(d);
        }
        true
    };

    match rule.freq {
        Freq::Daily => {
            for k in 0..MAX_STEPS {
                let d = first + Duration::days(k as i64 * interval as i64);
                if !emit(d, &mut out) {
                    break;
                }
            }
        }
        Freq::Weekly => {
            let mut days: Vec<u8> = rule.weekdays.iter().copied().filter(|d| *d < 7).collect();
            if days.is_empty() {
                days.push(first.weekday().num_days_from_sunday() as u8);
            }
            days.sort_unstable();
            days.dedup();
            let week0 = first - Duration::days(first.weekday().num_days_from_sunday() as i64);
            'weeks: for w in 0..MAX_STEPS {
                let base = week0 + Duration::weeks(w as i64 * interval as i64);
                for &wd in &days {
                    let d = base + Duration::days(wd as i64);
                    if d < first {
                        continue;
                    }
                    if !emit(d, &mut out) {
                        break 'weeks;
                    }
                }
            }
        }
        Freq::Monthly | Freq::Yearly => {
            let step = if rule.freq == Freq::Monthly { interval } else { interval * 12 };
            let month0 = first.with_day(1).unwrap();
            for k in 0..MAX_STEPS {
                let Some(month) = month0.checked_add_months(Months::new(step * k as u32)) else { break };
                if month > last {
                    break;
                }
                // Months without this day (e.g. the 31st, Feb 29) are skipped.
                if let Some(d) = month.with_day(first.day())
                    && !emit(d, &mut out)
                {
                    break;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn rule(freq: Freq) -> Repeat {
        Repeat { freq, interval: 1, weekdays: vec![], until: None, count: None }
    }

    fn fmt(v: Vec<NaiveDate>) -> Vec<String> {
        v.into_iter().map(|x| x.format("%m-%d").to_string()).collect()
    }

    #[test]
    fn daily_with_interval_and_window() {
        let r = Repeat { interval: 2, ..rule(Freq::Daily) };
        assert_eq!(fmt(occurrences(d("2026-10-01"), &r, d("2026-10-04"), d("2026-10-09"))), ["10-05", "10-07", "10-09"]);
    }

    #[test]
    fn until_and_count_end_the_series() {
        let until = Repeat { until: Some(d("2026-10-03")), ..rule(Freq::Daily) };
        assert_eq!(occurrences(d("2026-10-01"), &until, d("2026-09-01"), d("2026-12-31")).len(), 3);
        // The count includes occurrences before the queried window.
        let count = Repeat { count: Some(4), ..rule(Freq::Daily) };
        assert_eq!(fmt(occurrences(d("2026-10-01"), &count, d("2026-10-03"), d("2026-12-31"))), ["10-03", "10-04"]);
    }

    #[test]
    fn weekly_on_chosen_weekdays() {
        // 2026-10-07 is a Wednesday. Every other week on Mon and Wed.
        let r = Repeat { interval: 2, weekdays: vec![1, 3], ..rule(Freq::Weekly) };
        assert_eq!(
            fmt(occurrences(d("2026-10-07"), &r, d("2026-10-01"), d("2026-10-31"))),
            ["10-07", "10-19", "10-21"],
            "the Monday before the first occurrence is not included"
        );
        // Without weekdays it repeats on the start's weekday.
        assert_eq!(occurrences(d("2026-10-07"), &rule(Freq::Weekly), d("2026-10-01"), d("2026-10-31")).len(), 4);
    }

    #[test]
    fn monthly_skips_months_without_the_day_and_yearly_handles_leap_days() {
        assert_eq!(
            fmt(occurrences(d("2026-01-31"), &rule(Freq::Monthly), d("2026-01-01"), d("2026-05-31"))),
            ["01-31", "03-31", "05-31"]
        );
        let leap = occurrences(d("2024-02-29"), &rule(Freq::Yearly), d("2024-01-01"), d("2032-12-31"));
        assert_eq!(leap.iter().map(|x| x.year()).collect::<Vec<_>>(), [2024, 2028, 2032]);
    }

    #[test]
    fn a_rule_round_trips_through_json() {
        let r: Repeat = serde_json::from_str(r#"{"freq":"weekly","weekdays":[1,5],"until":"2026-12-31"}"#).unwrap();
        assert_eq!(r.interval, 1);
        assert_eq!(r.count, None);
        assert_eq!(serde_json::from_str::<Repeat>(&serde_json::to_string(&r).unwrap()).unwrap(), r);
    }
}
