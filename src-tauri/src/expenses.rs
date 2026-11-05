//! Money book: one row per expense or income, queried by date range.

use chrono::NaiveDate;
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::db::{D_FMT, Db, DbResult, now_ts, parse_d};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Expense {
    pub id: i64,
    /// In the currency chosen in the expense settings (major units, e.g. 12.50 or 4500).
    pub amount: f64,
    /// Preset category id, e.g. "food", "transport" (see src/lib/expenses.ts).
    pub category: String,
    pub date: NaiveDate,
    pub note: String,
    /// "expense" or "income" (rows from before income existed are expenses).
    #[serde(default = "Expense::default_kind")]
    pub kind: String,
}

impl Expense {
    pub const KINDS: [&'static str; 2] = ["expense", "income"];

    fn default_kind() -> String {
        "expense".into()
    }
}

fn expense_from_row(r: &Row) -> DbResult<Expense> {
    Ok(Expense { id: r.get(0)?, amount: r.get(1)?, category: r.get(2)?, date: parse_d(r.get(3)?), note: r.get(4)?, kind: r.get(5)? })
}

impl Db {
    /// Expenses dated within `from..=to`, newest first.
    pub fn expenses_between(&self, from: NaiveDate, to: NaiveDate) -> DbResult<Vec<Expense>> {
        let mut st = self.conn.prepare(
            "SELECT id, amount, category, date, note, kind FROM expenses WHERE date >= ?1 AND date <= ?2 ORDER BY date DESC, id DESC",
        )?;
        st.query_map([from.format(D_FMT).to_string(), to.format(D_FMT).to_string()], expense_from_row)?.collect()
    }

    pub fn save_expense(&self, e: &Expense) -> DbResult<i64> {
        let date = e.date.format(D_FMT).to_string();
        if e.id == 0 {
            self.conn.execute(
                "INSERT INTO expenses(amount, category, date, note, created_at, kind) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![e.amount, e.category, date, e.note, now_ts(), e.kind],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute(
                "UPDATE expenses SET amount=?1, category=?2, date=?3, note=?4, kind=?5 WHERE id=?6",
                params![e.amount, e.category, date, e.note, e.kind, e.id],
            )?;
            Ok(e.id)
        }
    }

    /// Deletes an expense and returns it, so the deletion can be undone.
    pub fn delete_expense(&self, id: i64) -> DbResult<Option<Expense>> {
        let stored = self
            .conn
            .query_row("SELECT id, amount, category, date, note, kind FROM expenses WHERE id=?1", [id], expense_from_row)
            .optional()?;
        self.conn.execute("DELETE FROM expenses WHERE id=?1", [id])?;
        Ok(stored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::{d, temp_db};

    #[test]
    fn expenses_are_queried_by_date_range_newest_first() {
        let (db, path) = temp_db();
        let add = |amount, day| {
            db.save_expense(&Expense { id: 0, amount, category: "food".into(), date: d(day), note: String::new(), kind: "expense".into() }).unwrap()
        };
        add(4500.0, "2026-09-30");
        let a = add(12.5, "2026-10-01");
        add(8000.0, "2026-10-31");
        add(1.0, "2026-11-01");
        let october = db.expenses_between(d("2026-10-01"), d("2026-10-31")).unwrap();
        assert_eq!(october.iter().map(|e| e.amount).collect::<Vec<_>>(), [8000.0, 12.5]);

        let removed = db.delete_expense(a).unwrap().unwrap();
        assert_eq!(removed.amount, 12.5);
        assert_eq!(db.expenses_between(d("2026-10-01"), d("2026-10-31")).unwrap().len(), 1);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn income_is_kept_apart_from_expenses() {
        let (db, path) = temp_db();
        let row = |amount, category: &str, kind: &str| Expense {
            id: 0,
            amount,
            category: category.into(),
            date: d("2026-11-04"),
            note: String::new(),
            kind: kind.into(),
        };
        db.save_expense(&row(9000.0, "food", "expense")).unwrap();
        db.save_expense(&row(2_500_000.0, "salary", "income")).unwrap();
        let month = db.expenses_between(d("2026-11-01"), d("2026-11-30")).unwrap();
        let income: f64 = month.iter().filter(|e| e.kind == "income").map(|e| e.amount).sum();
        let spent: f64 = month.iter().filter(|e| e.kind == "expense").map(|e| e.amount).sum();
        assert_eq!((income, spent), (2_500_000.0, 9000.0));
        let _ = std::fs::remove_file(path);
    }
}
