//! The rest of the health page: medicines (and whether each dose was taken), sleep, and how
//! you felt each day. Diet and exercise live in `health.rs`.

use chrono::NaiveDate;
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::db::{D_FMT, Db, DbResult, normalize_time, now_ts, parse_d};

/// A medicine (or supplement) and when to take it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Medication {
    pub id: i64,
    pub name: String,
    /// Free text: "1 tablet", "500 mg".
    #[serde(default)]
    pub dose: String,
    /// Times to take it each day, `HH:MM`, sorted. Empty means once a day at no set time.
    #[serde(default)]
    pub times: Vec<String>,
    /// Taken only when needed (a painkiller): no schedule, each time it is taken is logged.
    #[serde(default)]
    pub as_needed: bool,
    #[serde(default)]
    pub color: i64,
    #[serde(default)]
    pub note: String,
    /// Only scheduled from `start` to `end` (both optional), e.g. a 7-day course.
    #[serde(default)]
    pub start: Option<NaiveDate>,
    #[serde(default)]
    pub end: Option<NaiveDate>,
    /// Stopped medicines are kept with their records but no longer scheduled.
    #[serde(default = "Medication::yes")]
    pub active: bool,
    #[serde(default)]
    pub created_at: i64,
}

impl Medication {
    fn yes() -> bool {
        true
    }
}

/// One dose taken.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MedLog {
    pub id: i64,
    pub med_id: i64,
    pub date: NaiveDate,
    /// The scheduled time it was for (`HH:MM`, or empty for a medicine with no set time). For
    /// as-needed medicines, when it was taken.
    pub slot: String,
    pub taken_at: i64,
}

/// One night's sleep, filed under the day you woke up.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SleepLog {
    pub id: i64,
    pub date: NaiveDate,
    /// `HH:MM`; going to bed after midnight is fine (bed 01:30, wake 08:00).
    pub bed: String,
    pub wake: String,
    /// 1–5, or 0 when not rated.
    #[serde(default)]
    pub quality: i64,
    /// Times woken up during the night.
    #[serde(default)]
    pub wakeups: i64,
    /// Naps during the day, in minutes.
    #[serde(default)]
    pub nap_minutes: i64,
    #[serde(default)]
    pub note: String,
}

/// How a day felt: scores, tags ("tired", "appetite down" or anything typed) and a note.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Condition {
    pub id: i64,
    pub date: NaiveDate,
    /// 1–5 each, or 0 when not picked.
    #[serde(default)]
    pub mood: i64,
    #[serde(default)]
    pub energy: i64,
    #[serde(default)]
    pub stress: i64,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub note: String,
}

pub const MAX_NAP_MINUTES: i64 = 24 * 60;
pub const MAX_WAKEUPS: i64 = 99;

const MED_COLS: &str = "id, name, dose, times, as_needed, color, note, start, end_date, active, created_at";
const SLEEP_COLS: &str = "id, date, bed, wake, quality, wakeups, nap_minutes, note";
const CONDITION_COLS: &str = "id, date, mood, energy, stress, tags, note";

fn opt_date(s: Option<String>) -> Option<NaiveDate> {
    s.and_then(|s| NaiveDate::parse_from_str(&s, D_FMT).ok())
}
fn fmt_date(d: Option<NaiveDate>) -> Option<String> {
    d.map(|d| d.format(D_FMT).to_string())
}

fn med_from_row(r: &Row) -> DbResult<Medication> {
    let times: String = r.get(3)?;
    Ok(Medication {
        id: r.get(0)?,
        name: r.get(1)?,
        dose: r.get(2)?,
        times: serde_json::from_str(&times).unwrap_or_default(),
        as_needed: r.get(4)?,
        color: r.get(5)?,
        note: r.get(6)?,
        start: opt_date(r.get(7)?),
        end: opt_date(r.get(8)?),
        active: r.get(9)?,
        created_at: r.get(10)?,
    })
}

fn log_from_row(r: &Row) -> DbResult<MedLog> {
    Ok(MedLog { id: r.get(0)?, med_id: r.get(1)?, date: parse_d(r.get(2)?), slot: r.get(3)?, taken_at: r.get(4)? })
}

fn sleep_from_row(r: &Row) -> DbResult<SleepLog> {
    Ok(SleepLog {
        id: r.get(0)?,
        date: parse_d(r.get(1)?),
        bed: r.get(2)?,
        wake: r.get(3)?,
        quality: r.get(4)?,
        wakeups: r.get(5)?,
        nap_minutes: r.get(6)?,
        note: r.get(7)?,
    })
}

fn condition_from_row(r: &Row) -> DbResult<Condition> {
    let tags: String = r.get(5)?;
    Ok(Condition {
        id: r.get(0)?,
        date: parse_d(r.get(1)?),
        mood: r.get(2)?,
        energy: r.get(3)?,
        stress: r.get(4)?,
        tags: serde_json::from_str(&tags).unwrap_or_default(),
        note: r.get(6)?,
    })
}

/// Valid `HH:MM` times, sorted, without repeats.
pub fn clean_times(times: &[String]) -> Vec<String> {
    let mut out: Vec<String> = times.iter().filter_map(|t| normalize_time(t)).collect();
    out.sort();
    out.dedup();
    out
}

/// Tags trimmed, without empty ones or repeats, in the order given.
pub fn clean_tags(tags: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tag in tags {
        let tag: String = tag.trim().chars().take(40).collect();
        if !tag.is_empty() && !out.contains(&tag) {
            out.push(tag);
        }
    }
    out
}

impl Db {
    // ---- medicines

    pub fn medications(&self) -> DbResult<Vec<Medication>> {
        let mut st = self.conn.prepare(&format!("SELECT {MED_COLS} FROM medications ORDER BY active DESC, id"))?;
        st.query_map([], med_from_row)?.collect()
    }

    pub fn save_medication(&self, m: &Medication) -> DbResult<i64> {
        let times = serde_json::to_string(&clean_times(&m.times)).unwrap_or_else(|_| "[]".into());
        if m.id == 0 {
            self.conn.execute(
                "INSERT INTO medications(name, dose, times, as_needed, color, note, start, end_date, active, created_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![m.name, m.dose, times, m.as_needed, m.color, m.note, fmt_date(m.start), fmt_date(m.end), m.active, now_ts()],
            )?;
            return Ok(self.conn.last_insert_rowid());
        }
        self.conn.execute(
            "UPDATE medications SET name=?1, dose=?2, times=?3, as_needed=?4, color=?5, note=?6, start=?7, end_date=?8, active=?9
             WHERE id=?10",
            params![m.name, m.dose, times, m.as_needed, m.color, m.note, fmt_date(m.start), fmt_date(m.end), m.active, m.id],
        )?;
        Ok(m.id)
    }

    /// Deletes a medicine together with its records.
    pub fn delete_medication(&self, id: i64) -> DbResult<()> {
        self.conn.execute("DELETE FROM med_logs WHERE med_id=?1", [id])?;
        self.conn.execute("DELETE FROM medications WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn med_logs_between(&self, from: NaiveDate, to: NaiveDate) -> DbResult<Vec<MedLog>> {
        let mut st = self.conn.prepare(
            "SELECT id, med_id, date, slot, taken_at FROM med_logs WHERE date >= ?1 AND date <= ?2 ORDER BY date, slot, id",
        )?;
        st.query_map([from.format(D_FMT).to_string(), to.format(D_FMT).to_string()], log_from_row)?.collect()
    }

    /// Ticks (or unticks) one dose. Ticking twice keeps one record.
    pub fn set_med_taken(&self, med_id: i64, date: NaiveDate, slot: &str, taken: bool) -> DbResult<()> {
        let date = date.format(D_FMT).to_string();
        if taken {
            self.conn.execute(
                "INSERT OR IGNORE INTO med_logs(med_id, date, slot, taken_at) VALUES(?1, ?2, ?3, ?4)",
                params![med_id, date, slot, now_ts()],
            )?;
        } else {
            self.conn.execute("DELETE FROM med_logs WHERE med_id=?1 AND date=?2 AND slot=?3", params![med_id, date, slot])?;
        }
        Ok(())
    }

    // ---- sleep

    pub fn sleep_between(&self, from: NaiveDate, to: NaiveDate) -> DbResult<Vec<SleepLog>> {
        let mut st =
            self.conn.prepare(&format!("SELECT {SLEEP_COLS} FROM sleep_logs WHERE date >= ?1 AND date <= ?2 ORDER BY date"))?;
        st.query_map([from.format(D_FMT).to_string(), to.format(D_FMT).to_string()], sleep_from_row)?.collect()
    }

    /// Saves the night for its date, replacing whatever was recorded for that date.
    pub fn save_sleep(&self, s: &SleepLog) -> DbResult<i64> {
        let date = s.date.format(D_FMT).to_string();
        if s.id != 0 {
            // Moved to another date: that date's old record gives way.
            self.conn.execute("DELETE FROM sleep_logs WHERE date=?1 AND id<>?2", params![date, s.id])?;
            self.conn.execute(
                "UPDATE sleep_logs SET date=?1, bed=?2, wake=?3, quality=?4, wakeups=?5, nap_minutes=?6, note=?7 WHERE id=?8",
                params![date, s.bed, s.wake, s.quality, s.wakeups, s.nap_minutes, s.note, s.id],
            )?;
            return Ok(s.id);
        }
        self.conn.execute(
            "INSERT INTO sleep_logs(date, bed, wake, quality, wakeups, nap_minutes, note, created_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(date) DO UPDATE SET bed=excluded.bed, wake=excluded.wake, quality=excluded.quality,
                wakeups=excluded.wakeups, nap_minutes=excluded.nap_minutes, note=excluded.note",
            params![date, s.bed, s.wake, s.quality, s.wakeups, s.nap_minutes, s.note, now_ts()],
        )?;
        self.conn.query_row("SELECT id FROM sleep_logs WHERE date=?1", [date], |r| r.get(0))
    }

    /// Deletes a night and returns it, so the deletion can be undone.
    pub fn delete_sleep(&self, id: i64) -> DbResult<Option<SleepLog>> {
        let stored = self
            .conn
            .query_row(&format!("SELECT {SLEEP_COLS} FROM sleep_logs WHERE id=?1"), [id], sleep_from_row)
            .optional()?;
        self.conn.execute("DELETE FROM sleep_logs WHERE id=?1", [id])?;
        Ok(stored)
    }

    // ---- how the day felt

    pub fn conditions_between(&self, from: NaiveDate, to: NaiveDate) -> DbResult<Vec<Condition>> {
        let mut st = self
            .conn
            .prepare(&format!("SELECT {CONDITION_COLS} FROM conditions WHERE date >= ?1 AND date <= ?2 ORDER BY date"))?;
        st.query_map([from.format(D_FMT).to_string(), to.format(D_FMT).to_string()], condition_from_row)?.collect()
    }

    /// Saves the day's record (one per date). An empty record is removed instead.
    pub fn save_condition(&self, c: &Condition) -> DbResult<i64> {
        let date = c.date.format(D_FMT).to_string();
        let tags = clean_tags(&c.tags);
        if c.mood == 0 && c.energy == 0 && c.stress == 0 && tags.is_empty() && c.note.trim().is_empty() {
            self.conn.execute("DELETE FROM conditions WHERE date=?1", [date])?;
            return Ok(0);
        }
        let tags = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".into());
        self.conn.execute(
            "INSERT INTO conditions(date, mood, energy, stress, tags, note, created_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(date) DO UPDATE SET mood=excluded.mood, energy=excluded.energy, stress=excluded.stress,
                tags=excluded.tags, note=excluded.note",
            params![date, c.mood, c.energy, c.stress, tags, c.note, now_ts()],
        )?;
        self.conn.query_row("SELECT id FROM conditions WHERE date=?1", [date], |r| r.get(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::{d, temp_db};

    fn med(name: &str, times: &[&str]) -> Medication {
        Medication {
            id: 0,
            name: name.into(),
            dose: "1".into(),
            times: times.iter().map(|t| t.to_string()).collect(),
            as_needed: false,
            color: 0,
            note: String::new(),
            start: None,
            end: None,
            active: true,
            created_at: 0,
        }
    }

    #[test]
    fn doses_are_ticked_once_and_go_with_their_medicine() {
        let (db, path) = temp_db();
        let id = db.save_medication(&med("Vitamin D", &["21:00", "8:00", "bad", "08:00"])).unwrap();
        assert_eq!(db.medications().unwrap()[0].times, ["08:00", "21:00"]);

        let day = d("2026-11-16");
        db.set_med_taken(id, day, "08:00", true).unwrap();
        db.set_med_taken(id, day, "08:00", true).unwrap();
        db.set_med_taken(id, day, "21:00", true).unwrap();
        db.set_med_taken(id, day, "21:00", false).unwrap();
        let logs = db.med_logs_between(day, day).unwrap();
        assert_eq!(logs.iter().map(|l| l.slot.as_str()).collect::<Vec<_>>(), ["08:00"]);

        db.delete_medication(id).unwrap();
        assert!(db.med_logs_between(day, day).unwrap().is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn one_night_and_one_condition_per_day() {
        let (db, path) = temp_db();
        let night = SleepLog {
            id: 0,
            date: d("2026-11-16"),
            bed: "23:30".into(),
            wake: "07:00".into(),
            quality: 4,
            wakeups: 1,
            nap_minutes: 0,
            note: String::new(),
        };
        let id = db.save_sleep(&night).unwrap();
        assert_eq!(db.save_sleep(&SleepLog { wake: "07:30".into(), ..night.clone() }).unwrap(), id);
        let saved = db.sleep_between(d("2026-11-16"), d("2026-11-16")).unwrap();
        assert_eq!((saved.len(), saved[0].wake.as_str()), (1, "07:30"));

        let mut day = Condition {
            id: 0,
            date: d("2026-11-16"),
            mood: 2,
            energy: 1,
            stress: 0,
            tags: vec![" tired ".into(), "tired".into(), "".into(), "appetite_down".into()],
            note: String::new(),
        };
        db.save_condition(&day).unwrap();
        assert_eq!(db.conditions_between(day.date, day.date).unwrap()[0].tags, ["tired", "appetite_down"]);
        // Clearing everything removes the day's record.
        day = Condition { mood: 0, energy: 0, tags: vec![], ..day };
        assert_eq!(db.save_condition(&day).unwrap(), 0);
        assert!(db.conditions_between(day.date, day.date).unwrap().is_empty());
        let _ = std::fs::remove_file(path);
    }
}
