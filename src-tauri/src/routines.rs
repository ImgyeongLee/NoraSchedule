//! Routines: todos that come back by themselves (e.g. "a glass of water in the morning").
//! Each day a routine is due, it is added to the todo list once, due that day. Unticked copies
//! from earlier days are cleared away; ticked ones stay as a record.

use chrono::{Datelike, NaiveDate};
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
}

impl Routine {
    fn yes() -> bool {
        true
    }

    pub fn is_due(&self, day: NaiveDate) -> bool {
        let weekday = day.weekday().num_days_from_sunday() as u8;
        self.active && (self.weekdays.is_empty() || self.weekdays.contains(&weekday))
    }
}

fn routine_from_row(r: &Row) -> DbResult<Routine> {
    let weekdays: String = r.get(3)?;
    Ok(Routine {
        id: r.get(0)?,
        title: r.get(1)?,
        group_id: r.get(2)?,
        weekdays: serde_json::from_str(&weekdays).unwrap_or_default(),
        active: r.get(4)?,
        created_at: r.get(5)?,
    })
}

impl Db {
    pub fn routines(&self) -> DbResult<Vec<Routine>> {
        let mut st = self.conn.prepare("SELECT id, title, group_id, weekdays, active, created_at FROM routines ORDER BY id")?;
        st.query_map([], routine_from_row)?.collect()
    }

    pub fn save_routine(&self, r: &Routine) -> DbResult<i64> {
        let mut weekdays: Vec<u8> = r.weekdays.iter().copied().filter(|d| *d < 7).collect();
        weekdays.sort_unstable();
        weekdays.dedup();
        // All seven days is the same as every day.
        if weekdays.len() == 7 {
            weekdays.clear();
        }
        let weekdays = serde_json::to_string(&weekdays).unwrap_or_else(|_| "[]".into());
        if r.id == 0 {
            self.conn.execute(
                "INSERT INTO routines(title, group_id, weekdays, active, created_at) VALUES(?1, ?2, ?3, ?4, ?5)",
                params![r.title, r.group_id, weekdays, r.active, now_ts()],
            )?;
            return Ok(self.conn.last_insert_rowid());
        }
        self.conn.execute(
            "UPDATE routines SET title=?1, group_id=?2, weekdays=?3, active=?4 WHERE id=?5",
            params![r.title, r.group_id, weekdays, r.active, r.id],
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
        Routine { id: 0, title: title.into(), group_id: None, weekdays, active: true, created_at: 0 }
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
