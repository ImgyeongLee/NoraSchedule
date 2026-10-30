//! TRPG log: rule books and scenario books the user owns, scenarios they have played,
//! and scenarios they want to play. All four lists share one table, told apart by `kind`.

use chrono::NaiveDate;
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::db::{D_FMT, Db, DbResult, now_ts, parse_d};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrpgEntry {
    pub id: i64,
    /// `rulebook`, `scenario_book`, `played` or `wishlist` (see `KINDS`).
    pub kind: String,
    pub title: String,
    /// Scenario writer, or the book's author / publisher.
    #[serde(default)]
    pub writer: String,
    /// The rule system, e.g. "CoC 7th".
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub links: Vec<String>,
    /// Cover image file name (books), see `images.rs`.
    #[serde(default)]
    pub image: Option<String>,
    /// When it was played (`played` entries).
    #[serde(default)]
    pub date: Option<NaiveDate>,
    /// Roles played, comma-separated: `gm`, `pl`, `HO1`, `PC2`… (`played` entries); empty for none.
    #[serde(default)]
    pub role: String,
    /// Kept from an earlier version (who played together); no longer shown.
    #[serde(default)]
    pub pair: String,
    /// User-named group the entry is filed under, e.g. "CoC 타이만"; empty for none.
    #[serde(default)]
    pub folder: String,
    #[serde(default)]
    pub memo: String,
    #[serde(default)]
    pub created_at: i64,
    /// Place in the custom (dragged) order of its list; ties are shown A–Z.
    #[serde(default)]
    pub position: i64,
    /// Times a played session was played again after the first (`played` entries), so it counts
    /// 1 + this. (Before 0.6 this counted plays of scenario books; those values are no longer used.)
    #[serde(default)]
    pub play_count: i64,
}

impl TrpgEntry {
    pub const KINDS: [&'static str; 4] = ["rulebook", "scenario_book", "played", "wishlist"];
    pub const MAX_PLAY_COUNT: i64 = 999;

    /// Cleans a comma-separated role list: trimmed, no empties or duplicates, sensible lengths.
    pub fn clean_roles(roles: &str) -> String {
        let mut out: Vec<String> = Vec::new();
        for role in roles.split(',').map(str::trim).filter(|r| !r.is_empty()) {
            let role: String = role.chars().take(24).collect();
            if !out.iter().any(|r| r.eq_ignore_ascii_case(&role)) {
                out.push(role);
            }
        }
        out.truncate(20);
        out.join(",")
    }
}

const COLUMNS: &str = "id, kind, title, writer, system, links, image, date, role, memo, created_at, pair, folder, position, play_count";

/// Links are stored one per line, like event links.
fn links_text(links: &[String]) -> String {
    links.iter().map(|l| l.trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
}

fn entry_from_row(r: &Row) -> DbResult<TrpgEntry> {
    let links: String = r.get(5)?;
    let date: Option<String> = r.get(7)?;
    Ok(TrpgEntry {
        id: r.get(0)?,
        kind: r.get(1)?,
        title: r.get(2)?,
        writer: r.get(3)?,
        system: r.get(4)?,
        links: links.lines().filter(|l| !l.trim().is_empty()).map(str::to_owned).collect(),
        image: r.get(6)?,
        date: date.map(parse_d),
        role: r.get(8)?,
        memo: r.get(9)?,
        created_at: r.get(10)?,
        pair: r.get(11)?,
        folder: r.get(12)?,
        position: r.get(13)?,
        play_count: r.get(14)?,
    })
}

impl Db {
    /// Every entry: played ones newest first, the rest in the order they were added.
    pub fn trpg_entries(&self) -> DbResult<Vec<TrpgEntry>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM trpg_entries ORDER BY kind, date DESC, created_at DESC, id DESC"
        ))?;
        st.query_map([], entry_from_row)?.collect()
    }

    pub fn save_trpg_entry(&self, e: &TrpgEntry) -> DbResult<i64> {
        let date = e.date.map(|d| d.format(D_FMT).to_string());
        if e.id == 0 {
            let created = if e.created_at > 0 { e.created_at } else { now_ts() };
            self.conn.execute(
                // New entries go last in the custom order of their list.
                "INSERT INTO trpg_entries(kind, title, writer, system, links, image, date, role, memo, created_at, pair, folder,
                                          play_count, position)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                        (SELECT COALESCE(MAX(position), 0) + 1 FROM trpg_entries WHERE kind = ?1))",
                params![
                    e.kind, e.title, e.writer, e.system, links_text(&e.links), e.image, date, e.role, e.memo, created,
                    e.pair.trim(), e.folder.trim(), e.play_count
                ],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute(
                // The custom order is kept; moving to the other list puts it last there.
                "UPDATE trpg_entries SET
                   position = CASE WHEN kind = ?1 THEN position
                                   ELSE (SELECT COALESCE(MAX(position), 0) + 1 FROM trpg_entries WHERE kind = ?1) END,
                   kind=?1, title=?2, writer=?3, system=?4, links=?5, image=?6, date=?7, role=?8, memo=?9,
                   pair=?10, folder=?11, play_count=?12 WHERE id=?13",
                params![
                    e.kind, e.title, e.writer, e.system, links_text(&e.links), e.image, date, e.role, e.memo,
                    e.pair.trim(), e.folder.trim(), e.play_count, e.id
                ],
            )?;
            Ok(e.id)
        }
    }

    /// Saves the custom order of a list (dragged into place); `ids` lists it first to last.
    pub fn reorder_trpg_entries(&self, ids: &[i64]) -> DbResult<()> {
        let tx = self.conn.unchecked_transaction()?;
        for (position, id) in ids.iter().enumerate() {
            tx.execute("UPDATE trpg_entries SET position=?1 WHERE id=?2", params![position as i64, id])?;
        }
        tx.commit()
    }

    /// Deletes an entry and returns it, so the deletion can be undone.
    pub fn delete_trpg_entry(&self, id: i64) -> DbResult<Option<TrpgEntry>> {
        let stored = self
            .conn
            .query_row(&format!("SELECT {COLUMNS} FROM trpg_entries WHERE id=?1"), [id], entry_from_row)
            .optional()?;
        self.conn.execute("DELETE FROM trpg_entries WHERE id=?1", [id])?;
        Ok(stored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::{d, temp_db};

    fn entry(kind: &str, title: &str, date: Option<&str>) -> TrpgEntry {
        TrpgEntry {
            id: 0,
            kind: kind.into(),
            title: title.into(),
            writer: "Writer".into(),
            system: "CoC 7th".into(),
            links: vec![],
            image: None,
            date: date.map(d),
            role: String::new(),
            pair: String::new(),
            folder: String::new(),
            memo: String::new(),
            created_at: 0,
            position: 0,
            play_count: 0,
        }
    }

    #[test]
    fn played_sessions_keep_how_many_times_they_were_played_again() {
        let (db, path) = temp_db();
        let id = db.save_trpg_entry(&TrpgEntry { play_count: 2, ..entry("played", "COSMOS", Some("2026-03-03")) }).unwrap();
        let stored = db.trpg_entries().unwrap().into_iter().find(|e| e.id == id).unwrap();
        assert_eq!(stored.play_count, 2);
        db.save_trpg_entry(&TrpgEntry { play_count: 3, ..stored }).unwrap();
        assert_eq!(db.trpg_entries().unwrap()[0].play_count, 3);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn books_keep_their_dragged_order_and_new_ones_go_last() {
        let (db, path) = temp_db();
        let book = |title: &str| db.save_trpg_entry(&entry("rulebook", title, None)).unwrap();
        let (a, b, c) = (book("A"), book("B"), book("C"));
        let positions = |db: &Db| {
            let mut list: Vec<_> = db.trpg_entries().unwrap().into_iter().filter(|e| e.kind == "rulebook").collect();
            list.sort_by_key(|e| e.position);
            list.iter().map(|e| e.id).collect::<Vec<_>>()
        };
        assert_eq!(positions(&db), [a, b, c]);

        db.reorder_trpg_entries(&[c, a, b]).unwrap();
        let d = book("D");
        assert_eq!(positions(&db), [c, a, b, d]);

        // Editing keeps the place; moving to the other shelf puts it last there.
        let stored = db.trpg_entries().unwrap().into_iter().find(|e| e.id == a).unwrap();
        db.save_trpg_entry(&TrpgEntry { title: "A2".into(), ..stored.clone() }).unwrap();
        assert_eq!(positions(&db), [c, a, b, d]);
        let scenario = db.save_trpg_entry(&entry("scenario_book", "S", None)).unwrap();
        db.save_trpg_entry(&TrpgEntry { kind: "scenario_book".into(), ..stored }).unwrap();
        let shelf: Vec<_> = db.trpg_entries().unwrap().into_iter().filter(|e| e.kind == "scenario_book").collect();
        let pos = |id| shelf.iter().find(|e| e.id == id).unwrap().position;
        assert!(pos(a) > pos(scenario));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn roles_are_cleaned_and_folders_round_trip() {
        assert_eq!(TrpgEntry::clean_roles(" gm, pl ,,HO1, gm ,PC2"), "gm,pl,HO1,PC2");
        assert_eq!(TrpgEntry::clean_roles(""), "");
        let (db, path) = temp_db();
        let id = db
            .save_trpg_entry(&TrpgEntry { role: "gm,HO2".into(), folder: "CoC 타이만".into(), ..entry("played", "Duo", None) })
            .unwrap();
        let saved = db.trpg_entries().unwrap().into_iter().find(|e| e.id == id).unwrap();
        assert_eq!((saved.role.as_str(), saved.folder.as_str()), ("gm,HO2", "CoC 타이만"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn entries_round_trip_and_played_ones_sort_newest_first() {
        let (db, path) = temp_db();
        db.save_trpg_entry(&entry("played", "Old", Some("2025-01-02"))).unwrap();
        let newer = db.save_trpg_entry(&entry("played", "New", Some("2026-03-03"))).unwrap();
        let book = TrpgEntry { links: vec!["https://a.example".into(), " ".into(), "https://b.example".into()], image: Some("x.jpg".into()), ..entry("rulebook", "Rules", None) };
        db.save_trpg_entry(&book).unwrap();

        let all = db.trpg_entries().unwrap();
        let played: Vec<_> = all.iter().filter(|e| e.kind == "played").map(|e| e.title.as_str()).collect();
        assert_eq!(played, ["New", "Old"]);
        let rules = all.iter().find(|e| e.kind == "rulebook").unwrap();
        assert_eq!(rules.links, ["https://a.example", "https://b.example"], "blank links are dropped");
        assert!(rules.date.is_none());
        assert!(db.referenced_images().unwrap().contains("x.jpg"), "book covers are kept by the image cleanup");

        // A wishlist entry becomes a played one by changing its kind.
        let wish = db.save_trpg_entry(&entry("wishlist", "Someday", None)).unwrap();
        let mut done = db.trpg_entries().unwrap().into_iter().find(|e| e.id == wish).unwrap();
        done.kind = "played".into();
        done.date = Some(d("2026-10-08"));
        db.save_trpg_entry(&done).unwrap();
        assert_eq!(db.trpg_entries().unwrap().iter().filter(|e| e.kind == "played").count(), 3);

        let removed = db.delete_trpg_entry(newer).unwrap().unwrap();
        assert_eq!(removed.title, "New");
        assert!(db.delete_trpg_entry(newer).unwrap().is_none());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn single_links_from_the_first_version_become_link_lists() {
        let path = std::env::temp_dir().join(format!("nora_trpg_migrate_{}.sqlite3", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE trpg_entries (
                    id INTEGER PRIMARY KEY, kind TEXT NOT NULL, title TEXT NOT NULL,
                    writer TEXT NOT NULL DEFAULT '', system TEXT NOT NULL DEFAULT '', link TEXT NOT NULL DEFAULT '',
                    date TEXT, role TEXT NOT NULL DEFAULT '', memo TEXT NOT NULL DEFAULT '', created_at INTEGER NOT NULL);
                 INSERT INTO trpg_entries(kind, title, link, date, created_at) VALUES('played', 'Old', 'https://old.example', '2026-03-03', 1);",
            )
            .unwrap();
        }
        let db = Db::open(&path).unwrap();
        let old = db.trpg_entries().unwrap().pop().unwrap();
        assert_eq!(old.links, ["https://old.example"]);
        // New rows save fine although the old `link` column is still there.
        db.save_trpg_entry(&entry("wishlist", "New", None)).unwrap();
        assert_eq!(db.trpg_entries().unwrap().len(), 2);
        drop(db);
        let _ = std::fs::remove_file(path);
    }
}
