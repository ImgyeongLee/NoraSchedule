//! Routines: todos that come back by themselves (e.g. "a glass of water in the morning").
//! Each day a routine is due, it is added to the todo list once, due that day. Unticked copies
//! from earlier days are cleared away; ticked ones stay as a record.
//!
//! A routine repeats every week or every other week on chosen weekdays, or every month on
//! chosen days of the month.

use chrono::{Datelike, Duration, Local, NaiveDate};
use rusqlite::{params, Row};
use serde::{Deserialize, Serialize};

use crate::db::{D_FMT, Db, DbResult, now_ts};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Routine {
    pub id: i64,
    pub title: String,
    #[serde(default)]
    pub group_id: Option<i64>,
    /// Days it repeats on, 0 = Sunday … 6 = Saturday; empty means every day.
    #[serde(default)]
    pub weekdays: Vec<u8>,
    /// Paused routines are kept but not added.
    #[serde(default = "Routine::yes")]
    pub active: bool,
    #[serde(default)]
    pub created_at: i64,
    /// "weekly", "biweekly" (every other week, counted from the week of `start`) or "monthly".
    #[serde(default = "Routine::weekly")]
    pub freq: String,
    /// Days of the month for monthly routines, 1–31. A day the month doesn't have (the 31st in
    /// April) falls on the month's last day, so "31" means "the last day of every month".
    #[serde(default)]
    pub month_days: Vec<u8>,
    /// Any day in the first week of a biweekly routine.
    #[serde(default)]
    pub start: Option<NaiveDate>,
}

pub const FREQS: [&str; 3] = ["weekly", "biweekly", "monthly"];

/// Monday of the week `day` is in.
fn week_of(day: NaiveDate) -> NaiveDate {
    day - Duration::days(day.weekday().num_days_from_monday() as i64)
}

fn last_day_of_month(day: NaiveDate) -> u32 {
    let (y, m) = if day.month() == 12 { (day.year() + 1, 1) } else { (day.year(), day.month() + 1) };
    NaiveDate::from_ymd_opt(y, m, 1).map_or(31, |first| (first - Duration::days(1)).day())
}

impl Routine {
    fn yes() -> bool {
        true
    }

    fn weekly() -> String {
        "weekly".into()
    }

    pub fn is_due(&self, day: NaiveDate) -> bool {
        if !self.active {
            return false;
        }
        let weekday = day.weekday().num_days_from_sunday() as u8;
        let on_weekday = self.weekdays.is_empty() || self.weekdays.contains(&weekday);
        match self.freq.as_str() {
            "monthly" => {
                let (d, last) = (day.day(), last_day_of_month(day));
                self.month_days.iter().any(|&m| m as u32 == d || (m as u32 > last && d == last))
            }
            "biweekly" => {
                let start = self.start.unwrap_or(day);
                let weeks = (week_of(day) - week_of(start)).num_days() / 7;
                on_weekday && weeks.rem_euclid(2) == 0
            }
            _ => on_weekday,
        }
    }
}

fn routine_from_row(r: &Row) -> DbResult<Routine> {
    let weekdays: String = r.get(3)?;
    let month_days: String = r.get(7)?;
    let start: Option<String> = r.get(8)?;
    Ok(Routine {
        id: r.get(0)?,
        title: r.get(1)?,
        group_id: r.get(2)?,
        weekdays: serde_json::from_str(&weekdays).unwrap_or_default(),
        active: r.get(4)?,
        created_at: r.get(5)?,
        freq: r.get(6)?,
        month_days: serde_json::from_str(&month_days).unwrap_or_default(),
        start: start.and_then(|s| NaiveDate::parse_from_str(&s, D_FMT).ok()),
    })
}

/// Cleans up a routine's schedule: known frequency, sorted days, and the dates it needs.
fn normalized(r: &Routine) -> Routine {
    let today = Local::now().date_naive();
    let freq = if FREQS.contains(&r.freq.as_str()) { r.freq.clone() } else { "weekly".into() };
    let mut weekdays: Vec<u8> = r.weekdays.iter().copied().filter(|d| *d < 7).collect();
    weekdays.sort_unstable();
    weekdays.dedup();
    // All seven days is the same as every day.
    if weekdays.len() == 7 {
        weekdays.clear();
    }
    let mut month_days: Vec<u8> = r.month_days.iter().copied().filter(|d| (1..=31).contains(d)).collect();
    month_days.sort_unstable();
    month_days.dedup();
    if freq == "monthly" && month_days.is_empty() {
        month_days.push(r.start.unwrap_or(today).day() as u8);
    }
    let start = if freq == "biweekly" { Some(r.start.unwrap_or(today)) } else { r.start };
    Routine { freq, weekdays, month_days, start, ..r.clone() }
}

impl Db {
    pub fn routines(&self) -> DbResult<Vec<Routine>> {
        let mut st = self.conn.prepare(
            "SELECT id, title, group_id, weekdays, active, created_at, freq, month_days, start FROM routines ORDER BY id",
        )?;
        st.query_map([], routine_from_row)?.collect()
    }

    pub fn save_routine(&self, r: &Routine) -> DbResult<i64> {
        let r = normalized(r);
        let weekdays = serde_json::to_string(&r.weekdays).unwrap_or_else(|_| "[]".into());
        let month_days = serde_json::to_string(&r.month_days).unwrap_or_else(|_| "[]".into());
        let start = r.start.map(|d| d.format(D_FMT).to_string());
        if r.id == 0 {
            self.conn.execute(
                "INSERT INTO routines(title, group_id, weekdays, active, created_at, freq, month_days, start)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![r.title, r.group_id, weekdays, r.active, now_ts(), r.freq, month_days, start],
            )?;
            return Ok(self.conn.last_insert_rowid());
        }
        self.conn.execute(
            "UPDATE routines SET title=?1, group_id=?2, weekdays=?3, active=?4, freq=?5, month_days=?6, start=?7 WHERE id=?8",
            params![r.title, r.group_id, weekdays, r.active, r.freq, month_days, start, r.id],
        )?;
        // Copies not ticked yet follow the new name and group.
        self.conn.execute(
            "UPDATE todos SET title=?1, group_id=?2 WHERE routine_id=?3 AND done=0",
            params![r.title, r.group_id, r.id],
        )?;
        Ok(r.id)
    }

    /// Deletes a routine and its unticked copies; ticked ones stay as plain todos.
    pub fn delete_routine(&self, id: i64) -> DbResult<()> {
        self.conn.execute("DELETE FROM todos WHERE routine_id=?1 AND done=0", [id])?;
        self.conn.execute("DELETE FROM routines WHERE id=?1", [id])?;
        Ok(())
    }

    /// Adds today's copy of every routine due `day` (once per day, so a deleted copy stays deleted)
    /// and clears unticked copies left from earlier days. Returns how many were added.
    pub fn materialize_routines(&self, day: NaiveDate) -> DbResult<usize> {
        let date = day.format(D_FMT).to_string();
        self.conn.execute("DELETE FROM todos WHERE routine_id IS NOT NULL AND done=0 AND due < ?1", [&date])?;
        let mut added = 0;
        for r in self.routines()? {
            if !r.is_due(day) {
                continue;
            }
            let made: Option<String> = self.conn.query_row("SELECT last_made FROM routines WHERE id=?1", [r.id], |row| row.get(0))?;
            if made.as_deref().is_some_and(|m| m >= date.as_str()) {
                continue;
            }
            self.conn.execute(
                "INSERT INTO todos(group_id, parent_id, title, due, created_at, routine_id) VALUES(?1, NULL, ?2, ?3, ?4, ?5)",
                params![r.group_id, r.title, date, now_ts(), r.id],
            )?;
            self.conn.execute("UPDATE routines SET last_made=?1 WHERE id=?2", params![date, r.id])?;
            added += 1;
        }
        Ok(added)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::{d, temp_db};

    fn routine(title: &str, weekdays: Vec<u8>) -> Routine {
        Routine {
            id: 0,
            title: title.into(),
            group_id: None,
            weekdays,
            active: true,
            created_at: 0,
            freq: "weekly".into(),
            month_days: vec![],
            start: None,
        }
    }

    #[test]
    fn biweekly_routines_skip_every_other_week() {
        // 2026-11-16 is a Monday. On Mondays and Thursdays, every other week from that week.
        let r = Routine { freq: "biweekly".into(), start: Some(d("2026-11-18")), ..routine("Laundry", vec![1, 4]) };
        let due: Vec<_> = (0..28).map(|i| d("2026-11-16") + Duration::days(i)).filter(|day| r.is_due(*day)).collect();
        assert_eq!(due, [d("2026-11-16"), d("2026-11-19"), d("2026-11-30"), d("2026-12-03")]);
        // Weeks before the start week count too.
        assert!(r.is_due(d("2026-11-02")) && !r.is_due(d("2026-11-09")));
    }

    #[test]
    fn monthly_routines_fall_on_the_last_day_when_the_month_is_short() {
        let r = Routine { freq: "monthly".into(), month_days: vec![15, 31], ..routine("Rent", vec![]) };
        assert!(r.is_due(d("2026-11-15")) && r.is_due(d("2026-11-30")) && !r.is_due(d("2026-11-29")));
        assert!(r.is_due(d("2026-12-31")) && !r.is_due(d("2026-12-30")));
        assert!(r.is_due(d("2027-02-28")) && r.is_due(d("2028-02-29")) && !r.is_due(d("2028-02-28")));
    }

    #[test]
    fn saving_fills_in_what_a_schedule_needs() {
        let (db, path) = temp_db();
        db.save_routine(&Routine { freq: "monthly".into(), start: Some(d("2026-11-09")), ..routine("A", vec![]) }).unwrap();
        db.save_routine(&Routine { freq: "biweekly".into(), ..routine("B", vec![2]) }).unwrap();
        db.save_routine(&Routine { freq: "yearly".into(), ..routine("C", vec![0, 1, 2, 3, 4, 5, 6]) }).unwrap();
        let saved = db.routines().unwrap();
        assert_eq!(saved[0].month_days, [9]);
        assert!(saved[1].start.is_some());
        assert_eq!((saved[2].freq.as_str(), saved[2].weekdays.len()), ("weekly", 0));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn routines_are_added_once_a_day_on_their_days() {
        let (db, path) = temp_db();
        let water = db.save_routine(&routine("Water", vec![])).unwrap();
        // 2026-10-29 is a Thursday (4); 2026-10-31 a Saturday (6).
        db.save_routine(&routine("Gym", vec![1, 3, 5])).unwrap();

        assert_eq!(db.materialize_routines(d("2026-10-29")).unwrap(), 1);
        assert_eq!(db.materialize_routines(d("2026-10-29")).unwrap(), 0, "only once a day");
        let todos = db.todos().unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!((todos[0].title.as_str(), todos[0].routine_id), ("Water", Some(water)));

        // Deleting today's copy doesn't bring it back today.
        db.delete_todo(todos[0].id).unwrap();
        assert_eq!(db.materialize_routines(d("2026-10-29")).unwrap(), 0);

        // Friday: both are due.
        assert_eq!(db.materialize_routines(d("2026-10-30")).unwrap(), 2);
        let friday: Vec<_> = db.todos().unwrap();
        db.set_todo_done(friday.iter().find(|t| t.title == "Water").unwrap().id, true).unwrap();

        // Saturday: the unticked Gym copy from Friday is cleared, the ticked Water copy stays.
        assert_eq!(db.materialize_routines(d("2026-10-31")).unwrap(), 1);
        let titles: Vec<_> = db.todos().unwrap().into_iter().map(|t| (t.title, t.due.unwrap().to_string(), t.done)).collect();
        assert_eq!(
            titles,
            [("Water".into(), "2026-10-30".into(), true), ("Water".into(), "2026-10-31".into(), false)]
        );

        // Deleting the routine removes the unticked copy and keeps the ticked one.
        db.delete_routine(water).unwrap();
        let left = db.todos().unwrap();
        assert_eq!(left.len(), 1);
        assert!(left[0].done && left[0].routine_id.is_none());
        let _ = std::fs::remove_file(path);
    }
}
