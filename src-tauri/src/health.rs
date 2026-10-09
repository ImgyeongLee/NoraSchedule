//! Diet and exercise log: meals (eaten or still planned) and workouts, one row each, by date.

use chrono::NaiveDate;
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::db::{D_FMT, Db, DbResult, now_ts, parse_d};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Meal {
    pub id: i64,
    pub date: NaiveDate,
    /// "breakfast", "lunch", "dinner" or "snack".
    pub slot: String,
    pub name: String,
    pub kcal: i64,
    /// false = planned (going to eat it), true = already eaten.
    pub eaten: bool,
}

impl Meal {
    pub const SLOTS: [&'static str; 4] = ["breakfast", "lunch", "dinner", "snack"];
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workout {
    pub id: i64,
    pub date: NaiveDate,
    /// Preset exercise id, e.g. "walking", "running" (see src/lib/health.svelte.ts).
    pub kind: String,
    pub minutes: i64,
    /// Calories burned: the estimate when it was saved, or the user's own number.
    pub kcal: i64,
    pub note: String,
}

/// Upper limits that catch typos (an extra zero) without getting in anyone's way.
pub const MAX_KCAL: i64 = 20_000;
pub const MAX_MINUTES: i64 = 24 * 60;

const MEAL_COLS: &str = "id, date, slot, name, kcal, eaten";
const WORKOUT_COLS: &str = "id, date, kind, minutes, kcal, note";

fn meal_from_row(r: &Row) -> DbResult<Meal> {
    Ok(Meal { id: r.get(0)?, date: parse_d(r.get(1)?), slot: r.get(2)?, name: r.get(3)?, kcal: r.get(4)?, eaten: r.get(5)? })
}

fn workout_from_row(r: &Row) -> DbResult<Workout> {
    Ok(Workout {
        id: r.get(0)?,
        date: parse_d(r.get(1)?),
        kind: r.get(2)?,
        minutes: r.get(3)?,
        kcal: r.get(4)?,
        note: r.get(5)?,
    })
}

impl Db {
    /// Meals dated within `from..=to`, oldest first and in the order they were added.
    pub fn meals_between(&self, from: NaiveDate, to: NaiveDate) -> DbResult<Vec<Meal>> {
        let mut st =
            self.conn.prepare(&format!("SELECT {MEAL_COLS} FROM meals WHERE date >= ?1 AND date <= ?2 ORDER BY date, id"))?;
        st.query_map([from.format(D_FMT).to_string(), to.format(D_FMT).to_string()], meal_from_row)?.collect()
    }

    pub fn save_meal(&self, m: &Meal) -> DbResult<i64> {
        let date = m.date.format(D_FMT).to_string();
        if m.id == 0 {
            self.conn.execute(
                "INSERT INTO meals(date, slot, name, kcal, eaten, created_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![date, m.slot, m.name, m.kcal, m.eaten, now_ts()],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute(
                "UPDATE meals SET date=?1, slot=?2, name=?3, kcal=?4, eaten=?5 WHERE id=?6",
                params![date, m.slot, m.name, m.kcal, m.eaten, m.id],
            )?;
            Ok(m.id)
        }
    }

    /// Deletes a meal and returns it, so the deletion can be undone.
    pub fn delete_meal(&self, id: i64) -> DbResult<Option<Meal>> {
        let stored =
            self.conn.query_row(&format!("SELECT {MEAL_COLS} FROM meals WHERE id=?1"), [id], meal_from_row).optional()?;
        self.conn.execute("DELETE FROM meals WHERE id=?1", [id])?;
        Ok(stored)
    }

    /// Workouts dated within `from..=to`, oldest first and in the order they were added.
    pub fn workouts_between(&self, from: NaiveDate, to: NaiveDate) -> DbResult<Vec<Workout>> {
        let mut st = self
            .conn
            .prepare(&format!("SELECT {WORKOUT_COLS} FROM workouts WHERE date >= ?1 AND date <= ?2 ORDER BY date, id"))?;
        st.query_map([from.format(D_FMT).to_string(), to.format(D_FMT).to_string()], workout_from_row)?.collect()
    }

    pub fn save_workout(&self, w: &Workout) -> DbResult<i64> {
        let date = w.date.format(D_FMT).to_string();
        if w.id == 0 {
            self.conn.execute(
                "INSERT INTO workouts(date, kind, minutes, kcal, note, created_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![date, w.kind, w.minutes, w.kcal, w.note, now_ts()],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute(
                "UPDATE workouts SET date=?1, kind=?2, minutes=?3, kcal=?4, note=?5 WHERE id=?6",
                params![date, w.kind, w.minutes, w.kcal, w.note, w.id],
            )?;
            Ok(w.id)
        }
    }

    /// Deletes a workout and returns it, so the deletion can be undone.
    pub fn delete_workout(&self, id: i64) -> DbResult<Option<Workout>> {
        let stored = self
            .conn
            .query_row(&format!("SELECT {WORKOUT_COLS} FROM workouts WHERE id=?1"), [id], workout_from_row)
            .optional()?;
        self.conn.execute("DELETE FROM workouts WHERE id=?1", [id])?;
        Ok(stored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::{d, temp_db};

    #[test]
    fn meals_are_queried_by_date_and_can_be_marked_eaten() {
        let (db, path) = temp_db();
        let meal = |day, name: &str, eaten| Meal {
            id: 0,
            date: d(day),
            slot: "lunch".into(),
            name: name.into(),
            kcal: 500,
            eaten,
        };
        db.save_meal(&meal("2026-10-08", "yesterday", true)).unwrap();
        let salad = db.save_meal(&meal("2026-10-09", "salad", false)).unwrap();
        db.save_meal(&meal("2026-10-09", "soup", true)).unwrap();
        db.save_meal(&meal("2026-10-10", "tomorrow", false)).unwrap();

        let today = db.meals_between(d("2026-10-09"), d("2026-10-09")).unwrap();
        assert_eq!(today.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["salad", "soup"]);
        assert!(!today[0].eaten);

        db.save_meal(&Meal { id: salad, eaten: true, ..today[0].clone() }).unwrap();
        assert!(db.meals_between(d("2026-10-09"), d("2026-10-09")).unwrap()[0].eaten);

        let removed = db.delete_meal(salad).unwrap().unwrap();
        assert_eq!(removed.name, "salad");
        assert_eq!(db.meals_between(d("2026-10-08"), d("2026-10-10")).unwrap().len(), 3);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn workouts_are_queried_by_date_range() {
        let (db, path) = temp_db();
        let add = |day, kind: &str| {
            db.save_workout(&Workout { id: 0, date: d(day), kind: kind.into(), minutes: 30, kcal: 200, note: String::new() })
                .unwrap()
        };
        add("2026-10-01", "walking");
        let run = add("2026-10-05", "running");
        add("2026-10-12", "yoga");
        let week = db.workouts_between(d("2026-10-01"), d("2026-10-07")).unwrap();
        assert_eq!(week.iter().map(|w| w.kind.as_str()).collect::<Vec<_>>(), ["walking", "running"]);

        assert_eq!(db.delete_workout(run).unwrap().unwrap().kind, "running");
        assert!(db.delete_workout(run).unwrap().is_none());
        let _ = std::fs::remove_file(path);
    }
}
