//! Bookmarks: links grouped in (nestable) folders.

use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::db::{Db, DbResult, now_ts};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BookmarkFolder {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub color: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bookmark {
    pub id: i64,
    pub folder_id: Option<i64>,
    pub title: String,
    pub url: String,
    /// Preset icon, e.g. "gdoc", "gsheet", "blog" (see src/lib/bookmarks.ts).
    pub kind: String,
    pub note: String,
    #[serde(default)]
    pub created_at: i64,
}

fn bookmark_from_row(r: &Row) -> DbResult<Bookmark> {
    Ok(Bookmark {
        id: r.get(0)?,
        folder_id: r.get(1)?,
        title: r.get(2)?,
        url: r.get(3)?,
        kind: r.get(4)?,
        note: r.get(5)?,
        created_at: r.get(6)?,
    })
}

impl Db {
    pub fn bookmark_folders(&self) -> DbResult<Vec<BookmarkFolder>> {
        let mut st = self.conn.prepare("SELECT id, name, parent_id, color FROM bookmark_folders ORDER BY name COLLATE NOCASE")?;
        st.query_map([], |r| Ok(BookmarkFolder { id: r.get(0)?, name: r.get(1)?, parent_id: r.get(2)?, color: r.get(3)? }))?
            .collect()
    }

    /// True if `folder` is `ancestor` or lies somewhere below it.
    pub fn folder_is_within(&self, folder: i64, ancestor: i64) -> DbResult<bool> {
        let mut current = Some(folder);
        let mut steps = 0;
        while let Some(id) = current {
            if id == ancestor {
                return Ok(true);
            }
            steps += 1;
            if steps > 1000 {
                break;
            }
            current = self
                .conn
                .query_row("SELECT parent_id FROM bookmark_folders WHERE id=?1", [id], |r| r.get(0))
                .optional()?
                .flatten();
        }
        Ok(false)
    }

    pub fn save_bookmark_folder(&self, f: &BookmarkFolder) -> DbResult<i64> {
        if f.id == 0 {
            self.conn.execute(
                "INSERT INTO bookmark_folders(name, parent_id, color) VALUES(?1, ?2, ?3)",
                params![f.name, f.parent_id, f.color],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute(
                "UPDATE bookmark_folders SET name=?1, parent_id=?2, color=?3 WHERE id=?4",
                params![f.name, f.parent_id, f.color, f.id],
            )?;
            Ok(f.id)
        }
    }

    /// Deletes a folder. Its links and sub-folders move up to the folder's parent.
    pub fn delete_bookmark_folder(&self, id: i64) -> DbResult<()> {
        let parent: Option<i64> = self
            .conn
            .query_row("SELECT parent_id FROM bookmark_folders WHERE id=?1", [id], |r| r.get(0))
            .optional()?
            .flatten();
        self.conn.execute("UPDATE bookmarks SET folder_id=?1 WHERE folder_id=?2", params![parent, id])?;
        self.conn.execute("UPDATE bookmark_folders SET parent_id=?1 WHERE parent_id=?2", params![parent, id])?;
        self.conn.execute("DELETE FROM bookmark_folders WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn bookmarks(&self) -> DbResult<Vec<Bookmark>> {
        let mut st = self.conn.prepare(
            "SELECT id, folder_id, title, url, kind, note, created_at FROM bookmarks ORDER BY position, created_at DESC, id DESC",
        )?;
        st.query_map([], bookmark_from_row)?.collect()
    }

    pub fn save_bookmark(&self, b: &Bookmark) -> DbResult<i64> {
        if b.id == 0 {
            self.conn.execute(
                // New links go first, like before ordering existed.
                "INSERT INTO bookmarks(folder_id, title, url, kind, note, created_at, position)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, (SELECT COALESCE(MIN(position), 0) - 1 FROM bookmarks))",
                params![b.folder_id, b.title, b.url, b.kind, b.note, if b.created_at > 0 { b.created_at } else { now_ts() }],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute(
                "UPDATE bookmarks SET folder_id=?1, title=?2, url=?3, kind=?4, note=?5 WHERE id=?6",
                params![b.folder_id, b.title, b.url, b.kind, b.note, b.id],
            )?;
            Ok(b.id)
        }
    }

    /// Saves the order links are shown in (dragged into place); `ids` lists them first to last.
    pub fn reorder_bookmarks(&self, ids: &[i64]) -> DbResult<()> {
        let tx = self.conn.unchecked_transaction()?;
        for (position, id) in ids.iter().enumerate() {
            tx.execute("UPDATE bookmarks SET position=?1 WHERE id=?2", params![position as i64, id])?;
        }
        tx.commit()
    }

    /// Deletes a link and returns it, so the deletion can be undone.
    pub fn delete_bookmark(&self, id: i64) -> DbResult<Option<Bookmark>> {
        let stored = self
            .conn
            .query_row("SELECT id, folder_id, title, url, kind, note, created_at FROM bookmarks WHERE id=?1", [id], bookmark_from_row)
            .optional()?;
        self.conn.execute("DELETE FROM bookmarks WHERE id=?1", [id])?;
        Ok(stored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::temp_db;

    fn folder(db: &Db, name: &str, parent: Option<i64>) -> i64 {
        db.save_bookmark_folder(&BookmarkFolder { id: 0, name: name.into(), parent_id: parent, color: 0 }).unwrap()
    }

    fn link(db: &Db, folder: Option<i64>, title: &str) -> i64 {
        let b = Bookmark { id: 0, folder_id: folder, title: title.into(), url: "https://example.com".into(), kind: "article".into(), note: String::new(), created_at: 0 };
        db.save_bookmark(&b).unwrap()
    }

    #[test]
    fn deleting_a_folder_moves_its_contents_up() {
        let (db, path) = temp_db();
        let work = folder(&db, "Work", None);
        let docs = folder(&db, "Docs", Some(work));
        let specs = folder(&db, "Specs", Some(docs));
        let a = link(&db, Some(docs), "Design doc");

        db.delete_bookmark_folder(docs).unwrap();
        let folders = db.bookmark_folders().unwrap();
        assert_eq!(folders.iter().find(|f| f.id == specs).unwrap().parent_id, Some(work));
        assert_eq!(db.bookmarks().unwrap().iter().find(|b| b.id == a).unwrap().folder_id, Some(work));
        assert!(db.bookmarks().unwrap()[0].created_at > 0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn links_keep_the_order_they_are_dragged_into_and_new_ones_come_first() {
        let (db, path) = temp_db();
        let a = link(&db, None, "A");
        let b = link(&db, None, "B");
        let c = link(&db, None, "C");
        let order = |db: &Db| db.bookmarks().unwrap().iter().map(|x| x.id).collect::<Vec<_>>();
        assert_eq!(order(&db), [c, b, a]);
        db.reorder_bookmarks(&[a, c, b]).unwrap();
        assert_eq!(order(&db), [a, c, b]);
        let d = link(&db, None, "D");
        assert_eq!(order(&db), [d, a, c, b]);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn detects_folders_nested_inside_another() {
        let (db, path) = temp_db();
        let a = folder(&db, "A", None);
        let b = folder(&db, "B", Some(a));
        let c = folder(&db, "C", Some(b));
        assert!(db.folder_is_within(c, a).unwrap());
        assert!(db.folder_is_within(a, a).unwrap());
        assert!(!db.folder_is_within(a, c).unwrap());
        let removed = db.delete_bookmark(link(&db, None, "x")).unwrap().unwrap();
        assert_eq!(removed.title, "x");
        assert!(db.bookmarks().unwrap().is_empty());
        let _ = std::fs::remove_file(path);
    }
}
