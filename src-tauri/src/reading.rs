//! Reading log: books the user wants to read, is reading (with how far they got) and has read,
//! with a star rating and a review.

use chrono::NaiveDate;
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::db::{D_FMT, Db, DbResult, now_ts, parse_d};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Book {
    pub id: i64,
    pub title: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub publisher: String,
    /// `want`, `reading` or `read` (see `STATUSES`).
    pub status: String,
    /// Cover image file name, see `images.rs`.
    #[serde(default)]
    pub image: Option<String>,
    /// 0 when unknown.
    #[serde(default)]
    pub total_pages: u32,
    /// The page the user has read up to.
    #[serde(default)]
    pub current_page: u32,
    /// Half stars: 0 (not rated) to 10 (five stars).
    #[serde(default)]
    pub rating: u8,
    #[serde(default)]
    pub started: Option<NaiveDate>,
    #[serde(default)]
    pub finished: Option<NaiveDate>,
    /// Markdown.
    #[serde(default)]
    pub review: String,
    #[serde(default)]
    pub created_at: i64,
}

impl Book {
    pub const STATUSES: [&'static str; 3] = ["want", "reading", "read"];
    pub const MAX_RATING: u8 = 10;
}

const COLUMNS: &str =
    "id, title, author, publisher, status, image, total_pages, current_page, rating, started, finished, review, created_at";

fn book_from_row(r: &Row) -> DbResult<Book> {
    let started: Option<String> = r.get(9)?;
    let finished: Option<String> = r.get(10)?;
    Ok(Book {
        id: r.get(0)?,
        title: r.get(1)?,
        author: r.get(2)?,
        publisher: r.get(3)?,
        status: r.get(4)?,
        image: r.get(5)?,
        total_pages: r.get(6)?,
        current_page: r.get(7)?,
        rating: r.get(8)?,
        started: started.map(parse_d),
        finished: finished.map(parse_d),
        review: r.get(11)?,
        created_at: r.get(12)?,
    })
}

impl Db {
    /// Every book, most recently added first.
    pub fn books(&self) -> DbResult<Vec<Book>> {
        let mut st = self.conn.prepare(&format!("SELECT {COLUMNS} FROM books ORDER BY created_at DESC, id DESC"))?;
        st.query_map([], book_from_row)?.collect()
    }

    pub fn save_book(&self, b: &Book) -> DbResult<i64> {
        let day = |d: Option<NaiveDate>| d.map(|d| d.format(D_FMT).to_string());
        if b.id == 0 {
            let created = if b.created_at > 0 { b.created_at } else { now_ts() };
            self.conn.execute(
                "INSERT INTO books(title, author, publisher, status, image, total_pages, current_page, rating, started, finished,
                                   review, created_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    b.title, b.author, b.publisher, b.status, b.image, b.total_pages, b.current_page, b.rating,
                    day(b.started), day(b.finished), b.review, created
                ],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute(
                "UPDATE books SET title=?1, author=?2, publisher=?3, status=?4, image=?5, total_pages=?6, current_page=?7,
                                  rating=?8, started=?9, finished=?10, review=?11
                 WHERE id=?12",
                params![
                    b.title, b.author, b.publisher, b.status, b.image, b.total_pages, b.current_page, b.rating,
                    day(b.started), day(b.finished), b.review, b.id
                ],
            )?;
            Ok(b.id)
        }
    }

    /// Deletes a book and returns it, so the deletion can be undone.
    pub fn delete_book(&self, id: i64) -> DbResult<Option<Book>> {
        let stored = self
            .conn
            .query_row(&format!("SELECT {COLUMNS} FROM books WHERE id=?1"), [id], book_from_row)
            .optional()?;
        self.conn.execute("DELETE FROM books WHERE id=?1", [id])?;
        Ok(stored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::{d, temp_db};

    #[test]
    fn books_round_trip_with_progress_rating_and_review() {
        let (db, path) = temp_db();
        let id = db
            .save_book(&Book {
                id: 0,
                title: "소년이 온다".into(),
                author: "한강".into(),
                publisher: String::new(),
                status: "reading".into(),
                image: Some("cover.jpg".into()),
                total_pages: 216,
                current_page: 80,
                rating: 0,
                started: Some(d("2026-10-01")),
                finished: None,
                review: String::new(),
                created_at: 0,
            })
            .unwrap();

        let mut book = db.books().unwrap().pop().unwrap();
        assert_eq!((book.current_page, book.total_pages, book.started), (80, 216, Some(d("2026-10-01"))));
        assert!(db.referenced_images().unwrap().contains("cover.jpg"), "covers are kept by the image cleanup");

        book.status = "read".into();
        book.current_page = 216;
        book.finished = Some(d("2026-10-08"));
        book.rating = 9;
        book.review = "# 좋았다\n\n여운이 길다.".into();
        db.save_book(&book).unwrap();
        let saved = db.books().unwrap().pop().unwrap();
        assert_eq!((saved.status.as_str(), saved.rating), ("read", 9));
        assert_eq!(saved.review, "# 좋았다\n\n여운이 길다.");

        assert_eq!(db.delete_book(id).unwrap().unwrap().title, "소년이 온다");
        assert!(db.books().unwrap().is_empty());
        let _ = std::fs::remove_file(path);
    }
}
