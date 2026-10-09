//! SQLite persistence layer. Every entity in the app lives in a single database file.
//!
//! The command layer owns one [`Db`] behind a mutex; the focus tracker thread opens
//! its own connection via [`open_conn`] (WAL mode lets both write safely).

use std::collections::HashMap;
use std::path::Path;

use chrono::{Duration, Local, NaiveDate, NaiveDateTime, TimeZone};

use crate::recurrence::{self, Repeat};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

pub type DbResult<T> = rusqlite::Result<T>;

const DT_FMT: &str = "%Y-%m-%d %H:%M";
pub(crate) const D_FMT: &str = "%Y-%m-%d";

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS events (
    id       INTEGER PRIMARY KEY,
    title    TEXT    NOT NULL,
    start    TEXT    NOT NULL,
    end      TEXT    NOT NULL,
    all_day  INTEGER NOT NULL DEFAULT 0,
    color    INTEGER NOT NULL,
    location TEXT    NOT NULL DEFAULT '',
    links    TEXT    NOT NULL DEFAULT '',
    memo     TEXT    NOT NULL DEFAULT '',
    repeat   TEXT,
    exdates  TEXT    NOT NULL DEFAULT '[]',
    cancelled INTEGER NOT NULL DEFAULT 0,
    tags     TEXT    NOT NULL DEFAULT '[]',
    reminder INTEGER
);
CREATE INDEX IF NOT EXISTS idx_events_start ON events(start);

CREATE TABLE IF NOT EXISTS ddays (
    id    INTEGER PRIMARY KEY,
    title TEXT    NOT NULL,
    date  TEXT    NOT NULL,
    color INTEGER NOT NULL,
    image TEXT,
    yearly INTEGER NOT NULL DEFAULT 0,
    count_from_one INTEGER NOT NULL DEFAULT 0,
    shape TEXT NOT NULL DEFAULT 'normal'
);

CREATE TABLE IF NOT EXISTS todo_groups (
    id       INTEGER PRIMARY KEY,
    name     TEXT    NOT NULL,
    color    INTEGER NOT NULL,
    position INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS todos (
    id           INTEGER PRIMARY KEY,
    group_id     INTEGER REFERENCES todo_groups(id) ON DELETE SET NULL,
    parent_id    INTEGER REFERENCES todos(id) ON DELETE CASCADE,
    title        TEXT    NOT NULL,
    notes        TEXT    NOT NULL DEFAULT '',
    done         INTEGER NOT NULL DEFAULT 0,
    due          TEXT,
    due_time     TEXT,
    created_at   INTEGER NOT NULL,
    completed_at INTEGER
);
CREATE INDEX IF NOT EXISTS idx_todos_parent ON todos(parent_id);

CREATE TABLE IF NOT EXISTS memo_groups (
    id       INTEGER PRIMARY KEY,
    name     TEXT    NOT NULL,
    color    INTEGER NOT NULL,
    position INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS memos (
    id         INTEGER PRIMARY KEY,
    title      TEXT    NOT NULL,
    body       TEXT    NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    group_id   INTEGER REFERENCES memo_groups(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS pomodoro_sessions (
    id         INTEGER PRIMARY KEY,
    started_at INTEGER NOT NULL,
    ended_at   INTEGER NOT NULL,
    label      TEXT    NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS activity (
    id    INTEGER PRIMARY KEY,
    app   TEXT    NOT NULL,
    title TEXT    NOT NULL,
    start INTEGER NOT NULL,
    end   INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_activity_end ON activity(end);

CREATE TABLE IF NOT EXISTS bookmark_folders (
    id        INTEGER PRIMARY KEY,
    name      TEXT    NOT NULL,
    parent_id INTEGER REFERENCES bookmark_folders(id) ON DELETE SET NULL,
    color     INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS bookmarks (
    id         INTEGER PRIMARY KEY,
    folder_id  INTEGER REFERENCES bookmark_folders(id) ON DELETE SET NULL,
    title      TEXT    NOT NULL,
    url        TEXT    NOT NULL,
    kind       TEXT    NOT NULL,
    note       TEXT    NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    position   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS expenses (
    id         INTEGER PRIMARY KEY,
    amount     REAL    NOT NULL,
    category   TEXT    NOT NULL,
    date       TEXT    NOT NULL,
    note       TEXT    NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_expenses_date ON expenses(date);

CREATE TABLE IF NOT EXISTS tags (
    id       INTEGER PRIMARY KEY,
    name     TEXT    NOT NULL,
    color    INTEGER NOT NULL,
    category TEXT    NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS trpg_entries (
    id         INTEGER PRIMARY KEY,
    kind       TEXT    NOT NULL,
    title      TEXT    NOT NULL,
    writer     TEXT    NOT NULL DEFAULT '',
    system     TEXT    NOT NULL DEFAULT '',
    links      TEXT    NOT NULL DEFAULT '',
    image      TEXT,
    date       TEXT,
    role       TEXT    NOT NULL DEFAULT '',
    memo       TEXT    NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    pair       TEXT    NOT NULL DEFAULT '',
    folder     TEXT    NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS books (
    id           INTEGER PRIMARY KEY,
    title        TEXT    NOT NULL,
    author       TEXT    NOT NULL DEFAULT '',
    publisher    TEXT    NOT NULL DEFAULT '',
    status       TEXT    NOT NULL,
    image        TEXT,
    total_pages  INTEGER NOT NULL DEFAULT 0,
    current_page INTEGER NOT NULL DEFAULT 0,
    rating       INTEGER NOT NULL DEFAULT 0,
    started      TEXT,
    finished     TEXT,
    review       TEXT    NOT NULL DEFAULT '',
    created_at   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

pub fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Unix timestamp of local midnight at the start of `date`.
pub fn day_start_ts(date: NaiveDate) -> i64 {
    let naive = date.and_hms_opt(0, 0, 0).unwrap();
    Local
        .from_local_datetime(&naive)
        .earliest()
        .map(|d| d.timestamp())
        .unwrap_or_else(|| naive.and_utc().timestamp())
}

pub fn open_conn(path: &Path) -> DbResult<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.query_row("PRAGMA journal_mode=WAL", [], |_| Ok(()))?;
    conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=NORMAL;")?;
    Ok(conn)
}

// ---------------------------------------------------------------------------
// Models (serialized to the frontend as JSON with snake_case fields)

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: i64,
    pub title: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub all_day: bool,
    pub color: u32,
    pub location: String,
    pub links: Vec<String>,
    pub memo: String,
    /// Repeat rule; `None` for one-off events.
    #[serde(default)]
    pub repeat: Option<Repeat>,
    /// Dates of occurrences removed from the series ("delete just this one").
    #[serde(default)]
    pub exdates: Vec<NaiveDate>,
    /// For an expanded occurrence of a repeating event: the date this occurrence
    /// originally falls on. `None` for one-off events and stored rows.
    #[serde(default)]
    pub occurrence: Option<NaiveDate>,
    /// Called off but kept on the calendar (shown struck through).
    #[serde(default)]
    pub cancelled: bool,
    /// Ids of the tags attached to this event.
    #[serde(default)]
    pub tags: Vec<i64>,
    /// Remind this many minutes before the start; `None` for no reminder.
    #[serde(default)]
    pub reminder: Option<i64>,
}

/// A user-defined label for events, optionally grouped under a category.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: u32,
    pub category: String,
}

/// Which part of a repeating series an edit or delete applies to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    One,
    All,
}

fn repeat_json(r: &Option<Repeat>) -> Option<String> {
    r.as_ref().and_then(|r| serde_json::to_string(r).ok())
}

fn ids_json(ids: &[i64]) -> String {
    serde_json::to_string(ids).unwrap_or_else(|_| "[]".into())
}

fn dates_json(d: &[NaiveDate]) -> String {
    serde_json::to_string(d).unwrap_or_else(|_| "[]".into())
}

impl Event {
    /// First and last calendar date this event occupies.
    /// All-day events store their last day in `end`; timed events end exclusively.
    pub fn date_span(&self) -> (NaiveDate, NaiveDate) {
        let first = self.start.date();
        let last = if self.all_day {
            self.end.date()
        } else if self.end > self.start {
            (self.end - Duration::minutes(1)).date()
        } else {
            first
        };
        (first, last.max(first))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DDay {
    pub id: i64,
    pub title: String,
    pub date: NaiveDate,
    pub color: u32,
    /// File name of the cover image in the images folder, if any.
    pub image: Option<String>,
    /// Counts toward the same month and day every year (birthdays, anniversaries).
    #[serde(default)]
    pub yearly: bool,
    /// For past dates, count the date itself as day 1 (D+1) instead of day 0.
    #[serde(default)]
    pub count_from_one: bool,
    /// Card shape on the D-Day page: `normal`, `wide` (landscape) or `tall` (portrait).
    #[serde(default = "DDay::default_shape")]
    pub shape: String,
}

impl DDay {
    pub const SHAPES: [&'static str; 3] = ["normal", "wide", "tall"];

    fn default_shape() -> String {
        "normal".into()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TodoGroup {
    pub id: i64,
    pub name: String,
    pub color: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Todo {
    pub id: i64,
    pub group_id: Option<i64>,
    pub parent_id: Option<i64>,
    pub title: String,
    pub notes: String,
    pub done: bool,
    pub due: Option<NaiveDate>,
    /// Optional deadline time (`HH:MM`); only meaningful together with `due`.
    pub due_time: Option<String>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Memo {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub group_id: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoGroup {
    pub id: i64,
    pub name: String,
    pub color: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct PomodoroSession {
    pub started_at: i64,
    pub ended_at: i64,
    pub label: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Activity {
    pub app: String,
    pub title: String,
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Serialize)]
pub struct AppTime {
    pub app: String,
    pub secs: i64,
}

#[derive(Debug, Serialize)]
pub struct TitleTime {
    pub app: String,
    pub title: String,
    pub secs: i64,
}

#[derive(Debug, Serialize)]
pub struct ActivitySummary {
    pub total: i64,
    /// Seconds per requested day, in the same order as the request.
    pub per_day: Vec<i64>,
    pub apps: Vec<AppTime>,
    pub titles: Vec<TitleTime>,
}

// ---------------------------------------------------------------------------

fn parse_dt(s: String) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(&s, DT_FMT).unwrap_or_default()
}

pub(crate) fn parse_d(s: String) -> NaiveDate {
    NaiveDate::parse_from_str(&s, D_FMT).unwrap_or_default()
}

fn event_from_row(r: &Row) -> DbResult<Event> {
    let links: String = r.get(7)?;
    Ok(Event {
        id: r.get(0)?,
        title: r.get(1)?,
        start: parse_dt(r.get(2)?),
        end: parse_dt(r.get(3)?),
        all_day: r.get(4)?,
        color: r.get(5)?,
        location: r.get(6)?,
        links: links.lines().filter(|l| !l.trim().is_empty()).map(str::to_owned).collect(),
        memo: r.get(8)?,
        repeat: r.get::<_, Option<String>>(9)?.and_then(|s| serde_json::from_str(&s).ok()),
        exdates: serde_json::from_str(&r.get::<_, String>(10)?).unwrap_or_default(),
        occurrence: None,
        cancelled: r.get(11)?,
        tags: serde_json::from_str(&r.get::<_, String>(12)?).unwrap_or_default(),
        reminder: r.get(13)?,
    })
}

fn todo_from_row(r: &Row) -> DbResult<Todo> {
    let due: Option<String> = r.get(6)?;
    Ok(Todo {
        id: r.get(0)?,
        group_id: r.get(1)?,
        parent_id: r.get(2)?,
        title: r.get(3)?,
        notes: r.get(4)?,
        done: r.get(5)?,
        due: due.map(parse_d),
        due_time: r.get(9)?,
        created_at: r.get(7)?,
        completed_at: r.get(8)?,
    })
}

pub struct Db {
    /// Shared with the `impl Db` blocks in `bookmarks.rs`, `expenses.rs`, `trpg.rs` and `reading.rs`.
    pub(crate) conn: Connection,
}

/// Settings whose JSON values reference stored images (see `referenced_images`).
const IMAGE_SETTINGS: [&str; 3] = ["home.header", "home.layout", "stickers"];

fn collect_strings(value: &serde_json::Value, out: &mut std::collections::HashSet<String>) {
    match value {
        serde_json::Value::String(s) => {
            out.insert(s.clone());
        }
        serde_json::Value::Array(items) => items.iter().for_each(|v| collect_strings(v, out)),
        serde_json::Value::Object(map) => map.values().for_each(|v| collect_strings(v, out)),
        _ => {}
    }
}

/// Brings databases created by older versions up to the current schema.
fn migrate(conn: &Connection) -> DbResult<()> {
    let has_due_time = conn
        .prepare("SELECT 1 FROM pragma_table_info('todos') WHERE name = 'due_time'")?
        .exists([])?;
    if !has_due_time {
        conn.execute_batch("ALTER TABLE todos ADD COLUMN due_time TEXT")?;
    }
    let has_repeat = conn
        .prepare("SELECT 1 FROM pragma_table_info('events') WHERE name = 'repeat'")?
        .exists([])?;
    if !has_repeat {
        conn.execute_batch(
            "ALTER TABLE events ADD COLUMN repeat TEXT;
             ALTER TABLE events ADD COLUMN exdates TEXT NOT NULL DEFAULT '[]';",
        )?;
    }
    let has_image = conn
        .prepare("SELECT 1 FROM pragma_table_info('ddays') WHERE name = 'image'")?
        .exists([])?;
    if !has_image {
        conn.execute_batch("ALTER TABLE ddays ADD COLUMN image TEXT")?;
    }
    let has_yearly = conn
        .prepare("SELECT 1 FROM pragma_table_info('ddays') WHERE name = 'yearly'")?
        .exists([])?;
    if !has_yearly {
        conn.execute_batch(
            "ALTER TABLE ddays ADD COLUMN yearly INTEGER NOT NULL DEFAULT 0;
             ALTER TABLE ddays ADD COLUMN count_from_one INTEGER NOT NULL DEFAULT 0;",
        )?;
    }
    let has_shape = conn
        .prepare("SELECT 1 FROM pragma_table_info('ddays') WHERE name = 'shape'")?
        .exists([])?;
    if !has_shape {
        conn.execute_batch("ALTER TABLE ddays ADD COLUMN shape TEXT NOT NULL DEFAULT 'normal'")?;
    }
    let has_cancelled = conn
        .prepare("SELECT 1 FROM pragma_table_info('events') WHERE name = 'cancelled'")?
        .exists([])?;
    if !has_cancelled {
        conn.execute_batch("ALTER TABLE events ADD COLUMN cancelled INTEGER NOT NULL DEFAULT 0")?;
    }
    let has_memo_group = conn
        .prepare("SELECT 1 FROM pragma_table_info('memos') WHERE name = 'group_id'")?
        .exists([])?;
    if !has_memo_group {
        conn.execute_batch("ALTER TABLE memos ADD COLUMN group_id INTEGER REFERENCES memo_groups(id) ON DELETE SET NULL")?;
    }
    // The first TRPG log had a single `link` and no cover image.
    let has_trpg_links = conn
        .prepare("SELECT 1 FROM pragma_table_info('trpg_entries') WHERE name = 'links'")?
        .exists([])?;
    if !has_trpg_links {
        conn.execute_batch(
            "ALTER TABLE trpg_entries ADD COLUMN links TEXT NOT NULL DEFAULT '';
             ALTER TABLE trpg_entries ADD COLUMN image TEXT;
             UPDATE trpg_entries SET links = link;",
        )?;
    }
    let has_trpg_pair = conn
        .prepare("SELECT 1 FROM pragma_table_info('trpg_entries') WHERE name = 'pair'")?
        .exists([])?;
    if !has_trpg_pair {
        conn.execute_batch("ALTER TABLE trpg_entries ADD COLUMN pair TEXT NOT NULL DEFAULT ''")?;
    }
    // Bookmarks used to be listed newest first; keep that order as the starting point.
    let has_bookmark_position = conn
        .prepare("SELECT 1 FROM pragma_table_info('bookmarks') WHERE name = 'position'")?
        .exists([])?;
    if !has_bookmark_position {
        conn.execute_batch(
            "ALTER TABLE bookmarks ADD COLUMN position INTEGER NOT NULL DEFAULT 0;
             UPDATE bookmarks SET position = (
                 SELECT COUNT(*) FROM bookmarks b2
                 WHERE b2.created_at > bookmarks.created_at OR (b2.created_at = bookmarks.created_at AND b2.id > bookmarks.id)
             );",
        )?;
    }
    let has_trpg_folder = conn
        .prepare("SELECT 1 FROM pragma_table_info('trpg_entries') WHERE name = 'folder'")?
        .exists([])?;
    if !has_trpg_folder {
        conn.execute_batch("ALTER TABLE trpg_entries ADD COLUMN folder TEXT NOT NULL DEFAULT ''")?;
    }
    let has_tags = conn
        .prepare("SELECT 1 FROM pragma_table_info('events') WHERE name = 'tags'")?
        .exists([])?;
    if !has_tags {
        conn.execute_batch(
            "ALTER TABLE events ADD COLUMN tags TEXT NOT NULL DEFAULT '[]';
             ALTER TABLE events ADD COLUMN reminder INTEGER;",
        )?;
    }
    Ok(())
}

/// Accepts `HH:MM` (or `HH:MM:SS`) and normalizes it to `HH:MM`.
pub fn normalize_time(t: &str) -> Option<String> {
    chrono::NaiveTime::parse_from_str(t, "%H:%M")
        .or_else(|_| chrono::NaiveTime::parse_from_str(t, "%H:%M:%S"))
        .ok()
        .map(|t| t.format("%H:%M").to_string())
}

impl Db {
    pub fn open(path: &Path) -> DbResult<Self> {
        let conn = open_conn(path)?;
        conn.execute_batch(SCHEMA)?;
        migrate(&conn)?;
        Ok(Self { conn })
    }

    // ---- settings ---------------------------------------------------------

    pub fn setting(&self, key: &str) -> DbResult<Option<String>> {
        self.conn
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |r| r.get(0))
            .optional()
    }

    pub fn setting_or<T: std::str::FromStr>(&self, key: &str, default: T) -> T {
        self.setting(key).ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(default)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> DbResult<()> {
        self.conn.execute(
            "INSERT INTO settings(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    // ---- events -----------------------------------------------------------

    const EVENT_COLUMNS: &'static str = "id, title, start, end, all_day, color, location, links, memo, repeat, exdates, cancelled, tags, reminder";

    fn event(&self, id: i64) -> DbResult<Option<Event>> {
        self.conn
            .query_row(&format!("SELECT {} FROM events WHERE id=?1", Self::EVENT_COLUMNS), [id], event_from_row)
            .optional()
    }

    /// Events that touch any day in `from..=to`. Repeating events are expanded into one
    /// entry per occurrence; each carries the series id and its `occurrence` date.
    pub fn events_between(&self, from: NaiveDate, to: NaiveDate) -> DbResult<Vec<Event>> {
        let lo = from.and_hms_opt(0, 0, 0).unwrap().format(DT_FMT).to_string();
        let hi = (to + Duration::days(1)).and_hms_opt(0, 0, 0).unwrap().format(DT_FMT).to_string();
        let mut st = self.conn.prepare(&format!(
            "SELECT {} FROM events
             WHERE (repeat IS NULL AND start < ?1 AND end >= ?2) OR (repeat IS NOT NULL AND start < ?1)
             ORDER BY all_day DESC, start",
            Self::EVENT_COLUMNS
        ))?;
        let rows: Vec<Event> = st.query_map([hi, lo], event_from_row)?.collect::<DbResult<_>>()?;

        let mut out = Vec::new();
        for e in rows {
            match e.repeat.clone() {
                None => out.push(e),
                Some(rule) => {
                    let (first, last) = e.date_span();
                    let span_days = (last - first).num_days();
                    let length = e.end - e.start;
                    // Start a little early so multi-day occurrences that began before `from` are kept.
                    for day in recurrence::occurrences(first, &rule, from - Duration::days(span_days), to) {
                        if e.exdates.contains(&day) {
                            continue;
                        }
                        let start = day.and_time(e.start.time());
                        out.push(Event { start, end: start + length, occurrence: Some(day), ..e.clone() });
                    }
                }
            }
        }
        out.retain(|e| {
            let (a, b) = e.date_span();
            a <= to && b >= from
        });
        out.sort_by(|a, b| b.all_day.cmp(&a.all_day).then(a.start.cmp(&b.start)));
        Ok(out)
    }

    fn insert_event(&self, e: &Event) -> DbResult<i64> {
        self.conn.execute(
            "INSERT INTO events(title, start, end, all_day, color, location, links, memo, repeat, exdates, cancelled,
             tags, reminder)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                e.title, e.start.format(DT_FMT).to_string(), e.end.format(DT_FMT).to_string(), e.all_day, e.color,
                e.location, e.links.join("\n"), e.memo, repeat_json(&e.repeat), dates_json(&e.exdates), e.cancelled,
                ids_json(&e.tags), e.reminder
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn update_event_row(&self, e: &Event) -> DbResult<()> {
        self.conn.execute(
            "UPDATE events SET title=?1, start=?2, end=?3, all_day=?4, color=?5, location=?6, links=?7, memo=?8,
             repeat=?9, exdates=?10, cancelled=?11, tags=?12, reminder=?13 WHERE id=?14",
            params![
                e.title, e.start.format(DT_FMT).to_string(), e.end.format(DT_FMT).to_string(), e.all_day, e.color,
                e.location, e.links.join("\n"), e.memo, repeat_json(&e.repeat), dates_json(&e.exdates), e.cancelled,
                ids_json(&e.tags), e.reminder, e.id
            ],
        )?;
        Ok(())
    }

    /// Creates or updates an event. For an occurrence of a repeating event, `scope`
    /// decides whether the change applies to that occurrence only or to the whole series.
    /// Returns the id of the saved event.
    pub fn save_event(&self, e: &Event, scope: Scope) -> DbResult<i64> {
        if e.id == 0 {
            return self.insert_event(&Event { exdates: vec![], ..e.clone() });
        }
        let Some(stored) = self.event(e.id)? else { return self.insert_event(e) };
        let occurrence = e.occurrence.filter(|_| stored.repeat.is_some());
        let Some(occurrence) = occurrence else {
            self.update_event_row(&Event { exdates: stored.exdates, ..e.clone() })?;
            return Ok(e.id);
        };
        match scope {
            Scope::One => {
                // Detach: skip this date in the series and store the edited copy on its own.
                self.add_exdate(&stored, occurrence)?;
                self.insert_event(&Event { id: 0, repeat: None, exdates: vec![], occurrence: None, ..e.clone() })
            }
            Scope::All => {
                // Move the series by however much this occurrence was moved.
                let delta = e.start - occurrence.and_time(stored.start.time());
                let start = stored.start + delta;
                let day_shift = Duration::days((start.date() - stored.start.date()).num_days());
                let exdates = stored.exdates.iter().map(|d| *d + day_shift).collect();
                self.update_event_row(&Event { start, end: start + (e.end - e.start), exdates, ..e.clone() })?;
                Ok(e.id)
            }
        }
    }

    fn add_exdate(&self, series: &Event, day: NaiveDate) -> DbResult<()> {
        let mut exdates = series.exdates.clone();
        if !exdates.contains(&day) {
            exdates.push(day);
            exdates.sort();
        }
        self.conn.execute("UPDATE events SET exdates=?1 WHERE id=?2", params![dates_json(&exdates), series.id])?;
        Ok(())
    }

    /// Deletes an event, or just one occurrence of a repeating event.
    /// Returns the stored row (before deletion) so the action can be undone.
    pub fn delete_event(&self, id: i64, occurrence: Option<NaiveDate>, scope: Scope) -> DbResult<Option<Event>> {
        let Some(stored) = self.event(id)? else { return Ok(None) };
        match (scope, occurrence, stored.repeat.is_some()) {
            (Scope::One, Some(day), true) => self.add_exdate(&stored, day)?,
            _ => {
                self.conn.execute("DELETE FROM events WHERE id=?1", [id])?;
            }
        }
        Ok(Some(stored))
    }

    /// Brings back a single occurrence that was deleted with `Scope::One`.
    pub fn restore_occurrence(&self, id: i64, day: NaiveDate) -> DbResult<()> {
        if let Some(mut stored) = self.event(id)? {
            stored.exdates.retain(|d| *d != day);
            self.conn.execute("UPDATE events SET exdates=?1 WHERE id=?2", params![dates_json(&stored.exdates), id])?;
        }
        Ok(())
    }

    // ---- tags -------------------------------------------------------------

    pub fn tags(&self) -> DbResult<Vec<Tag>> {
        let mut st = self.conn.prepare("SELECT id, name, color, category FROM tags ORDER BY category, name")?;
        st.query_map([], |r| Ok(Tag { id: r.get(0)?, name: r.get(1)?, color: r.get(2)?, category: r.get(3)? }))?
            .collect()
    }

    /// Creates (`id == 0`) or updates a tag. Returns its id.
    pub fn save_tag(&self, t: &Tag) -> DbResult<i64> {
        if t.id == 0 {
            self.conn.execute(
                "INSERT INTO tags(name, color, category) VALUES(?1, ?2, ?3)",
                params![t.name, t.color, t.category],
            )?;
            return Ok(self.conn.last_insert_rowid());
        }
        self.conn.execute(
            "UPDATE tags SET name=?1, color=?2, category=?3 WHERE id=?4",
            params![t.name, t.color, t.category, t.id],
        )?;
        Ok(t.id)
    }

    /// Deletes a tag and takes it off every event.
    pub fn delete_tag(&self, id: i64) -> DbResult<()> {
        let tagged: Vec<(i64, String)> = self
            .conn
            .prepare("SELECT id, tags FROM events WHERE tags != '[]'")?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<DbResult<_>>()?;
        for (event_id, json) in tagged {
            let mut ids: Vec<i64> = serde_json::from_str(&json).unwrap_or_default();
            if ids.contains(&id) {
                ids.retain(|t| *t != id);
                self.conn.execute("UPDATE events SET tags=?1 WHERE id=?2", params![ids_json(&ids), event_id])?;
            }
        }
        self.conn.execute("DELETE FROM tags WHERE id=?1", [id])?;
        Ok(())
    }

    // ---- d-days -----------------------------------------------------------

    pub fn ddays(&self) -> DbResult<Vec<DDay>> {
        let mut st = self
            .conn
            .prepare("SELECT id, title, date, color, image, yearly, count_from_one, shape FROM ddays ORDER BY date")?;
        st.query_map([], |r| {
            Ok(DDay {
                id: r.get(0)?,
                title: r.get(1)?,
                date: parse_d(r.get(2)?),
                color: r.get(3)?,
                image: r.get(4)?,
                yearly: r.get(5)?,
                count_from_one: r.get(6)?,
                shape: r.get(7)?,
            })
        })?
        .collect()
    }

    pub fn save_dday(&self, d: &DDay) -> DbResult<()> {
        let date = d.date.format(D_FMT).to_string();
        if d.id == 0 {
            self.conn.execute(
                "INSERT INTO ddays(title, date, color, image, yearly, count_from_one, shape) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![d.title, date, d.color, d.image, d.yearly, d.count_from_one, d.shape],
            )?;
        } else {
            self.conn.execute(
                "UPDATE ddays SET title=?1, date=?2, color=?3, image=?4, yearly=?5, count_from_one=?6, shape=?7 WHERE id=?8",
                params![d.title, date, d.color, d.image, d.yearly, d.count_from_one, d.shape, d.id],
            )?;
        }
        Ok(())
    }

    /// Image files still referenced by some D-Day.
    pub fn referenced_images(&self) -> DbResult<std::collections::HashSet<String>> {
        let mut st = self.conn.prepare(
            "SELECT image FROM ddays WHERE image IS NOT NULL
             UNION SELECT image FROM trpg_entries WHERE image IS NOT NULL
             UNION SELECT image FROM books WHERE image IS NOT NULL",
        )?;
        let mut names: std::collections::HashSet<String> = st.query_map([], |r| r.get(0))?.collect::<DbResult<_>>()?;
        // The Overview header, image cards and stickers live in JSON settings; keep every image they mention.
        for key in IMAGE_SETTINGS {
            if let Some(json) = self.setting(key)? {
                collect_strings(&serde_json::from_str(&json).unwrap_or_default(), &mut names);
            }
        }
        Ok(names)
    }

    pub fn delete_dday(&self, id: i64) -> DbResult<()> {
        self.conn.execute("DELETE FROM ddays WHERE id=?1", [id])?;
        Ok(())
    }

    // ---- todo groups ------------------------------------------------------

    pub fn todo_groups(&self) -> DbResult<Vec<TodoGroup>> {
        let mut st = self.conn.prepare("SELECT id, name, color FROM todo_groups ORDER BY position, id")?;
        st.query_map([], |r| Ok(TodoGroup { id: r.get(0)?, name: r.get(1)?, color: r.get(2)? }))?
            .collect()
    }

    pub fn add_todo_group(&self, name: &str, color: u32) -> DbResult<i64> {
        self.conn.execute(
            "INSERT INTO todo_groups(name, color, position)
             VALUES(?1, ?2, (SELECT COALESCE(MAX(position), 0) + 1 FROM todo_groups))",
            params![name, color],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_todo_group(&self, g: &TodoGroup) -> DbResult<()> {
        self.conn.execute(
            "UPDATE todo_groups SET name=?1, color=?2 WHERE id=?3",
            params![g.name, g.color, g.id],
        )?;
        Ok(())
    }

    /// Deletes the group; its todos become ungrouped.
    pub fn delete_todo_group(&self, id: i64) -> DbResult<()> {
        self.conn.execute("DELETE FROM todo_groups WHERE id=?1", [id])?;
        Ok(())
    }

    // ---- todos ------------------------------------------------------------

    pub fn todos(&self) -> DbResult<Vec<Todo>> {
        let mut st = self.conn.prepare(
            "SELECT id, group_id, parent_id, title, notes, done, due, created_at, completed_at, due_time
             FROM todos ORDER BY id",
        )?;
        st.query_map([], todo_from_row)?.collect()
    }

    pub fn add_todo(
        &self,
        group_id: Option<i64>,
        parent_id: Option<i64>,
        title: &str,
        due: Option<NaiveDate>,
        due_time: Option<&str>,
    ) -> DbResult<i64> {
        let due_text = due.map(|d| d.format(D_FMT).to_string());
        let due_time = due.and(due_time.and_then(normalize_time));
        self.conn.execute(
            "INSERT INTO todos(group_id, parent_id, title, due, due_time, created_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![group_id, parent_id, title, due_text, due_time, now_ts()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Updates title/notes/deadline, and moves the todo (with all sub-todos) to `group_id`.
    pub fn update_todo(&self, t: &Todo) -> DbResult<()> {
        let due = t.due.map(|d| d.format(D_FMT).to_string());
        let due_time = t.due.and(t.due_time.as_deref().and_then(normalize_time));
        self.conn.execute(
            "UPDATE todos SET title=?1, notes=?2, due=?3, due_time=?4 WHERE id=?5",
            params![t.title, t.notes, due, due_time, t.id],
        )?;
        self.conn.execute(
            "WITH RECURSIVE sub(id) AS (
                SELECT ?1 UNION ALL SELECT t.id FROM todos t JOIN sub ON t.parent_id = sub.id)
             UPDATE todos SET group_id=?2 WHERE id IN sub",
            params![t.id, t.group_id],
        )?;
        Ok(())
    }

    /// Completing a todo also completes all of its sub-todos.
    pub fn set_todo_done(&self, id: i64, done: bool) -> DbResult<()> {
        if done {
            self.conn.execute(
                "WITH RECURSIVE sub(id) AS (
                    SELECT ?1 UNION ALL SELECT t.id FROM todos t JOIN sub ON t.parent_id = sub.id)
                 UPDATE todos SET done=1, completed_at=?2 WHERE id IN sub AND done=0",
                params![id, now_ts()],
            )?;
        } else {
            self.conn.execute("UPDATE todos SET done=0, completed_at=NULL WHERE id=?1", [id])?;
        }
        Ok(())
    }

    /// Deletes a todo and (via ON DELETE CASCADE) all of its sub-todos.
    pub fn delete_todo(&self, id: i64) -> DbResult<()> {
        self.conn.execute("DELETE FROM todos WHERE id=?1", [id])?;
        Ok(())
    }

    // ---- memos ------------------------------------------------------------

    pub fn memos(&self) -> DbResult<Vec<Memo>> {
        let mut st = self.conn.prepare(
            "SELECT id, title, body, created_at, updated_at, group_id FROM memos ORDER BY updated_at DESC, id DESC",
        )?;
        st.query_map([], |r| {
            Ok(Memo {
                id: r.get(0)?,
                title: r.get(1)?,
                body: r.get(2)?,
                created_at: r.get(3)?,
                updated_at: r.get(4)?,
                group_id: r.get(5)?,
            })
        })?
        .collect()
    }

    pub fn create_memo(&self, title: &str, group_id: Option<i64>) -> DbResult<i64> {
        let now = now_ts();
        self.conn.execute(
            "INSERT INTO memos(title, body, created_at, updated_at, group_id) VALUES(?1, '', ?2, ?2, ?3)",
            params![title, now, group_id],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Moves a memo to a group (`None` = ungrouped). Does not change its "edited" time.
    pub fn set_memo_group(&self, id: i64, group_id: Option<i64>) -> DbResult<()> {
        self.conn.execute("UPDATE memos SET group_id=?1 WHERE id=?2", params![group_id, id])?;
        Ok(())
    }

    pub fn memo_groups(&self) -> DbResult<Vec<MemoGroup>> {
        let mut st = self.conn.prepare("SELECT id, name, color FROM memo_groups ORDER BY position, id")?;
        st.query_map([], |r| Ok(MemoGroup { id: r.get(0)?, name: r.get(1)?, color: r.get(2)? }))?
            .collect()
    }

    pub fn save_memo_group(&self, g: &MemoGroup) -> DbResult<i64> {
        if g.id == 0 {
            self.conn.execute(
                "INSERT INTO memo_groups(name, color, position)
                 VALUES(?1, ?2, (SELECT COALESCE(MAX(position), 0) + 1 FROM memo_groups))",
                params![g.name, g.color],
            )?;
            Ok(self.conn.last_insert_rowid())
        } else {
            self.conn.execute("UPDATE memo_groups SET name=?1, color=?2 WHERE id=?3", params![g.name, g.color, g.id])?;
            Ok(g.id)
        }
    }

    /// Deletes a group; its memos become ungrouped (they are never deleted with it).
    pub fn delete_memo_group(&self, id: i64) -> DbResult<()> {
        self.conn.execute("UPDATE memos SET group_id=NULL WHERE group_id=?1", [id])?;
        self.conn.execute("DELETE FROM memo_groups WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn update_memo(&self, id: i64, title: &str, body: &str) -> DbResult<()> {
        self.conn.execute(
            "UPDATE memos SET title=?1, body=?2, updated_at=?3 WHERE id=?4",
            params![title, body, now_ts(), id],
        )?;
        Ok(())
    }

    pub fn delete_memo(&self, id: i64) -> DbResult<()> {
        self.conn.execute("DELETE FROM memos WHERE id=?1", [id])?;
        Ok(())
    }

    // ---- pomodoro ---------------------------------------------------------

    pub fn add_pomodoro_session(&self, started_at: i64, ended_at: i64, label: &str) -> DbResult<()> {
        self.conn.execute(
            "INSERT INTO pomodoro_sessions(started_at, ended_at, label) VALUES(?1, ?2, ?3)",
            params![started_at, ended_at, label],
        )?;
        Ok(())
    }

    pub fn pomodoro_sessions(&self, from_ts: i64, to_ts: i64) -> DbResult<Vec<PomodoroSession>> {
        let mut st = self.conn.prepare(
            "SELECT started_at, ended_at, label FROM pomodoro_sessions
             WHERE ended_at > ?1 AND started_at < ?2 ORDER BY started_at",
        )?;
        st.query_map([from_ts, to_ts], |r| {
            Ok(PomodoroSession { started_at: r.get(0)?, ended_at: r.get(1)?, label: r.get(2)? })
        })?
        .collect()
    }

    // ---- activity ---------------------------------------------------------

    /// Activity segments overlapping `[from_ts, to_ts)`, clipped to that range.
    pub fn activity(&self, from_ts: i64, to_ts: i64) -> DbResult<Vec<Activity>> {
        let mut st = self.conn.prepare(
            "SELECT app, title, start, end FROM activity
             WHERE end > ?1 AND start < ?2 ORDER BY start",
        )?;
        st.query_map([from_ts, to_ts], |r| {
            let start: i64 = r.get(2)?;
            let end: i64 = r.get(3)?;
            Ok(Activity { app: r.get(0)?, title: r.get(1)?, start: start.max(from_ts), end: end.min(to_ts) })
        })?
        .collect()
    }

    /// One day's segments, with back-to-back segments of the same app merged
    /// so the timeline stays small.
    pub fn activity_timeline(&self, day: NaiveDate) -> DbResult<Vec<Activity>> {
        let segments = self.activity(day_start_ts(day), day_start_ts(day + Duration::days(1)))?;
        let mut merged: Vec<Activity> = Vec::new();
        for a in segments {
            match merged.last_mut() {
                Some(last) if last.app == a.app && a.start - last.end <= 5 => {
                    last.end = last.end.max(a.end);
                    if last.title.is_empty() {
                        last.title = a.title;
                    }
                }
                _ => merged.push(a),
            }
        }
        Ok(merged)
    }

    /// Totals per day, per app and per window title for the given (sorted) days.
    pub fn activity_summary(&self, days: &[NaiveDate], title_limit: usize) -> DbResult<ActivitySummary> {
        let (Some(first), Some(last)) = (days.first(), days.last()) else {
            return Ok(ActivitySummary { total: 0, per_day: vec![], apps: vec![], titles: vec![] });
        };
        let segments = self.activity(day_start_ts(*first), day_start_ts(*last + Duration::days(1)))?;

        let per_day = days
            .iter()
            .map(|d| {
                let (s, e) = (day_start_ts(*d), day_start_ts(*d + Duration::days(1)));
                segments.iter().map(|a| (a.end.min(e) - a.start.max(s)).max(0)).sum()
            })
            .collect::<Vec<i64>>();

        let mut apps: HashMap<&str, i64> = HashMap::new();
        let mut titles: HashMap<(&str, &str), i64> = HashMap::new();
        for a in &segments {
            let secs = a.end - a.start;
            *apps.entry(&a.app).or_default() += secs;
            if !a.title.is_empty() {
                *titles.entry((&a.app, &a.title)).or_default() += secs;
            }
        }
        let mut apps: Vec<AppTime> = apps.into_iter().map(|(app, secs)| AppTime { app: app.into(), secs }).collect();
        apps.sort_by(|a, b| b.secs.cmp(&a.secs).then_with(|| a.app.cmp(&b.app)));
        let mut titles: Vec<TitleTime> = titles
            .into_iter()
            .map(|((app, title), secs)| TitleTime { app: app.into(), title: title.into(), secs })
            .collect();
        titles.sort_by_key(|t| std::cmp::Reverse(t.secs));
        titles.truncate(title_limit);

        Ok(ActivitySummary { total: per_day.iter().sum(), per_day, apps, titles })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub(crate) fn temp_db() -> (Db, std::path::PathBuf) {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("nora_test_{}_{n}.sqlite3", std::process::id()));
        let _ = std::fs::remove_file(&path);
        (Db::open(&path).unwrap(), path)
    }

    pub(crate) fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, D_FMT).unwrap()
    }

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, DT_FMT).unwrap()
    }

    #[test]
    fn completing_and_deleting_todos_cascades_to_sub_todos() {
        let (db, path) = temp_db();
        let parent = db.add_todo(None, None, "parent", None, None).unwrap();
        let child = db.add_todo(None, Some(parent), "child", None, None).unwrap();
        db.add_todo(None, Some(child), "grandchild", None, None).unwrap();

        db.set_todo_done(parent, true).unwrap();
        assert!(db.todos().unwrap().iter().all(|t| t.done && t.completed_at.is_some()));

        db.set_todo_done(child, false).unwrap();
        assert!(!db.todos().unwrap().iter().find(|t| t.id == child).unwrap().done);

        let group = db.add_todo_group("Work", 0).unwrap();
        let mut p = db.todos().unwrap().into_iter().find(|t| t.id == parent).unwrap();
        p.group_id = Some(group);
        db.update_todo(&p).unwrap();
        assert!(db.todos().unwrap().iter().all(|t| t.group_id == Some(group)));

        db.delete_todo(parent).unwrap();
        assert!(db.todos().unwrap().is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn deadlines_keep_time_only_with_a_date() {
        let (db, path) = temp_db();
        let a = db.add_todo(None, None, "a", Some(d("2026-10-09")), Some("18:30")).unwrap();
        let b = db.add_todo(None, None, "b", None, Some("09:00")).unwrap();
        let todos = db.todos().unwrap();
        let get = |id| todos.iter().find(|t| t.id == id).unwrap().clone();
        assert_eq!(get(a).due_time.as_deref(), Some("18:30"));
        assert_eq!(get(b).due_time, None, "a time without a date is dropped");

        let mut t = get(a);
        t.due_time = Some("07:05:00".into());
        db.update_todo(&t).unwrap();
        assert_eq!(db.todos().unwrap()[0].due_time.as_deref(), Some("07:05"));
        t.due = None;
        db.update_todo(&t).unwrap();
        assert_eq!(db.todos().unwrap()[0].due_time, None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn migration_adds_due_time_to_old_databases() {
        static N: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!("nora_old_{}_{}.sqlite3", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let _ = std::fs::remove_file(&path);
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE todos (id INTEGER PRIMARY KEY, group_id INTEGER, parent_id INTEGER, title TEXT NOT NULL,
                 notes TEXT NOT NULL DEFAULT '', done INTEGER NOT NULL DEFAULT 0, due TEXT, created_at INTEGER NOT NULL,
                 completed_at INTEGER);
                 INSERT INTO todos(title, due, created_at) VALUES('old', '2026-01-01', 0);",
            )
            .unwrap();
        }
        let db = Db::open(&path).unwrap();
        let todos = db.todos().unwrap();
        assert_eq!(todos[0].title, "old");
        assert_eq!(todos[0].due_time, None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn header_card_and_sticker_images_are_kept_by_cleanup() {
        let (db, path) = temp_db();
        db.set_setting("home.header", r#"{"image":"a-1.jpg","framing":{"x":0.5,"y":0.5,"zoom":1},"height":"md"}"#).unwrap();
        db.set_setting("home.layout", r#"[{"uid":"u","id":"image","size":"sm","image":"b-2.jpg"}]"#).unwrap();
        db.set_setting("stickers", r#"{"library":["c-3.png"],"placed":[{"id":"s","image":"c-3.png","page":"home"}]}"#).unwrap();
        db.set_setting("ui.unrelated", r#""d-4.jpg""#).unwrap();
        let keep = db.referenced_images().unwrap();
        for name in ["a-1.jpg", "b-2.jpg", "c-3.png"] {
            assert!(keep.contains(name), "{name} must survive cleanup");
        }
        assert!(!keep.contains("d-4.jpg"), "only image settings are scanned");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn events_are_found_by_every_day_they_cover() {
        let (db, path) = temp_db();
        let ev = |title: &str, start, end, all_day| Event {
            id: 0,
            title: title.into(),
            start,
            end,
            all_day,
            color: 0,
            location: String::new(),
            links: vec!["https://a.example".into(), "https://b.example".into()],
            memo: String::new(),
            repeat: None,
            exdates: vec![],
            occurrence: None,
            cancelled: false,
            tags: vec![],
            reminder: None,
        };
        db.save_event(&ev("Trip", dt("2026-10-05 00:00"), dt("2026-10-07 00:00"), true), Scope::All).unwrap();
        db.save_event(&ev("Meeting", dt("2026-10-08 23:00"), dt("2026-10-09 00:00"), false), Scope::All).unwrap();
        let on = |day| db.events_between(d(day), d(day)).unwrap().into_iter().map(|e| e.title).collect::<Vec<_>>();
        assert_eq!(on("2026-10-04"), Vec::<String>::new());
        assert_eq!(on("2026-10-07"), vec!["Trip"]);
        assert_eq!(on("2026-10-08"), vec!["Meeting"]);
        // A timed event ending exactly at midnight does not spill into the next day.
        assert_eq!(on("2026-10-09"), Vec::<String>::new());
        assert_eq!(db.events_between(d("2026-10-01"), d("2026-10-31")).unwrap()[0].links.len(), 2);
        let _ = std::fs::remove_file(path);
    }

    fn series(db: &Db, start: &str, end: &str, rule: Repeat) -> i64 {
        db.save_event(
            &Event {
                id: 0, title: "Standup".into(), start: dt(start), end: dt(end), all_day: false, color: 0,
                location: String::new(), links: vec![], memo: String::new(), repeat: Some(rule), exdates: vec![],
                occurrence: None, cancelled: false, tags: vec![], reminder: None,
            },
            Scope::All,
        )
        .unwrap()
    }

    fn daily(until: &str) -> Repeat {
        Repeat { freq: recurrence::Freq::Daily, interval: 1, weekdays: vec![], until: Some(d(until)), count: None }
    }

    #[test]
    fn repeating_events_expand_and_can_skip_one_occurrence() {
        let (db, path) = temp_db();
        let id = series(&db, "2026-10-05 09:00", "2026-10-05 09:30", daily("2026-10-08"));
        let week = db.events_between(d("2026-10-04"), d("2026-10-10")).unwrap();
        assert_eq!(week.len(), 4);
        assert!(week.iter().all(|e| e.id == id && e.occurrence.is_some()));
        assert_eq!(week[2].start, dt("2026-10-07 09:00"));

        let removed = db.delete_event(id, Some(d("2026-10-06")), Scope::One).unwrap().unwrap();
        assert_eq!(removed.start, dt("2026-10-05 09:00"), "returns the stored series for undo");
        let days: Vec<_> = db.events_between(d("2026-10-04"), d("2026-10-10")).unwrap().iter().map(|e| e.start.date()).collect();
        assert_eq!(days, [d("2026-10-05"), d("2026-10-07"), d("2026-10-08")]);

        db.restore_occurrence(id, d("2026-10-06")).unwrap();
        assert_eq!(db.events_between(d("2026-10-04"), d("2026-10-10")).unwrap().len(), 4);

        db.delete_event(id, Some(d("2026-10-06")), Scope::All).unwrap();
        assert!(db.events_between(d("2026-10-04"), d("2026-10-10")).unwrap().is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn editing_one_occurrence_detaches_it_and_editing_all_moves_the_series() {
        let (db, path) = temp_db();
        let id = series(&db, "2026-10-05 09:00", "2026-10-05 09:30", daily("2026-10-08"));
        let mut occ = db.events_between(d("2026-10-06"), d("2026-10-06")).unwrap().remove(0);

        occ.title = "Moved standup".into();
        occ.start = dt("2026-10-06 14:00");
        occ.end = dt("2026-10-06 15:00");
        let new_id = db.save_event(&occ, Scope::One).unwrap();
        assert_ne!(new_id, id);
        let tue = db.events_between(d("2026-10-06"), d("2026-10-06")).unwrap();
        assert_eq!(tue.len(), 1);
        assert_eq!((tue[0].title.as_str(), tue[0].repeat.is_none()), ("Moved standup", true));

        // Shift the whole series one hour later via its Wednesday occurrence.
        let mut wed = db.events_between(d("2026-10-07"), d("2026-10-07")).unwrap().remove(0);
        wed.start = dt("2026-10-07 10:00");
        wed.end = dt("2026-10-07 10:30");
        db.save_event(&wed, Scope::All).unwrap();
        let series_days: Vec<_> = db
            .events_between(d("2026-10-04"), d("2026-10-10"))
            .unwrap()
            .into_iter()
            .filter(|e| e.id == id)
            .map(|e| e.start)
            .collect();
        assert_eq!(series_days, [dt("2026-10-05 10:00"), dt("2026-10-07 10:00"), dt("2026-10-08 10:00")]);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn memo_groups_keep_their_memos_safe() {
        let (db, path) = temp_db();
        let work = db.save_memo_group(&MemoGroup { id: 0, name: "Work".into(), color: 1 }).unwrap();
        let a = db.create_memo("Plan", Some(work)).unwrap();
        let b = db.create_memo("Loose", None).unwrap();
        db.update_memo(b, "Loose", "text").unwrap();
        let before = db.memos().unwrap().iter().find(|m| m.id == b).unwrap().updated_at;
        db.set_memo_group(b, Some(work)).unwrap();
        let memos = db.memos().unwrap();
        assert!(memos.iter().all(|m| m.group_id == Some(work)));
        assert_eq!(memos.iter().find(|m| m.id == b).unwrap().updated_at, before, "moving is not an edit");

        db.save_memo_group(&MemoGroup { id: work, name: "Office".into(), color: 2 }).unwrap();
        assert_eq!(db.memo_groups().unwrap()[0].name, "Office");
        db.delete_memo_group(work).unwrap();
        assert!(db.memo_groups().unwrap().is_empty());
        let memos = db.memos().unwrap();
        assert_eq!(memos.len(), 2, "deleting a group keeps its memos");
        assert!(memos.iter().all(|m| m.group_id.is_none()));
        assert!(memos.iter().any(|m| m.id == a));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn migration_adds_memo_groups_to_old_databases() {
        static N: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!("nora_oldmemo_{}_{}.sqlite3", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let _ = std::fs::remove_file(&path);
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE memos (id INTEGER PRIMARY KEY, title TEXT NOT NULL, body TEXT NOT NULL DEFAULT '',
                 created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
                 INSERT INTO memos(title, body, created_at, updated_at) VALUES('old note', 'hi', 0, 0);",
            )
            .unwrap();
        }
        let db = Db::open(&path).unwrap();
        let memos = db.memos().unwrap();
        assert_eq!((memos[0].title.as_str(), memos[0].group_id), ("old note", None));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn activity_summary_splits_segments_at_midnight() {
        let (db, path) = temp_db();
        let midnight = day_start_ts(d("2026-10-08"));
        let insert = |app: &str, title: &str, s: i64, e: i64| {
            db.conn
                .execute("INSERT INTO activity(app, title, start, end) VALUES(?1, ?2, ?3, ?4)", params![app, title, s, e])
                .unwrap();
        };
        insert("Code", "main.rs", midnight - 600, midnight + 300);
        insert("Chrome", "", midnight + 400, midnight + 500);
        let s = db.activity_summary(&[d("2026-10-07"), d("2026-10-08")], 10).unwrap();
        assert_eq!(s.per_day, vec![600, 400]);
        assert_eq!(s.total, 1000);
        assert_eq!(s.apps[0].app, "Code");
        assert_eq!(s.titles.len(), 1, "empty titles are not listed");
        let _ = std::fs::remove_file(path);
    }
}
