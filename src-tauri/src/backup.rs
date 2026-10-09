//! Export, import and reset of all user data.
//!
//! A backup is a zip file holding a consistent copy of the database, the D-Day
//! images and a small manifest. Importing and resetting never touch the live
//! database: they *stage* the change in `<data dir>/pending` and the app restarts;
//! `apply_pending` then swaps the files at startup, before anything opens them.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::db::{Db, now_ts};
use crate::images;

/// Bumped when the backup layout changes in a way older apps cannot read.
pub const FORMAT: u32 = 1;
pub const DB_FILE: &str = "nora.sqlite3";
const MANIFEST: &str = "manifest.json";
const IMAGE_PREFIX: &str = "images/";
const PENDING: &str = "pending";
const READY: &str = "READY";
const RESET: &str = "RESET";
/// Refuse to unpack absurd entries (protects against zip bombs).
const MAX_ENTRY_BYTES: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Counts {
    pub events: u64,
    pub todos: u64,
    pub memos: u64,
    pub ddays: u64,
    pub bookmarks: u64,
    pub expenses: u64,
    /// Missing in backups made before the TRPG log existed.
    #[serde(default)]
    pub trpg: u64,
    /// Missing in backups made before the reading log existed.
    #[serde(default)]
    pub books: u64,
    pub images: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub format: u32,
    pub app_version: String,
    pub exported_at: i64,
    pub counts: Counts,
}

/// Error codes the frontend translates.
const INVALID: &str = "backup_invalid";
const TOO_NEW: &str = "backup_too_new";

fn io_err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

impl Db {
    /// Writes a consistent copy of the whole database to `path` (which must not exist).
    pub fn snapshot_to(&self, path: &Path) -> crate::db::DbResult<()> {
        self.conn.execute("VACUUM INTO ?1", [path.to_string_lossy()])?;
        Ok(())
    }

    pub fn counts(&self) -> crate::db::DbResult<Counts> {
        let count = |table: &str| -> crate::db::DbResult<u64> {
            self.conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get::<_, i64>(0)).map(|n| n as u64)
        };
        Ok(Counts {
            events: count("events")?,
            todos: count("todos")?,
            memos: count("memos")?,
            ddays: count("ddays")?,
            bookmarks: count("bookmarks")?,
            expenses: count("expenses")?,
            trpg: count("trpg_entries")?,
            books: count("books")?,
            images: 0,
        })
    }
}

/// Step 1 of an export (holds the database lock briefly): snapshot + counts.
pub fn prepare_export(db: &Db, scratch: &Path) -> Result<Counts, String> {
    let _ = fs::remove_file(scratch);
    db.snapshot_to(scratch).map_err(io_err)?;
    db.counts().map_err(io_err)
}

/// Step 2 of an export: write the zip. The database snapshot at `snapshot` is consumed.
pub fn write_backup(snapshot: &Path, images_dir: &Path, mut counts: Counts, out: &Path) -> Result<Manifest, String> {
    let image_files: Vec<_> = fs::read_dir(images_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| images::is_safe_name(&e.file_name().to_string_lossy()))
                .collect()
        })
        .unwrap_or_default();
    counts.images = image_files.len() as u64;
    let manifest = Manifest { format: FORMAT, app_version: env!("CARGO_PKG_VERSION").into(), exported_at: now_ts(), counts };

    // Write next to the destination, then rename, so a failed export never leaves a broken file.
    let partial = out.with_extension("partial");
    let result = (|| -> Result<(), String> {
        let mut zip = ZipWriter::new(File::create(&partial).map_err(io_err)?);
        let deflate = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

        zip.start_file(MANIFEST, deflate).map_err(io_err)?;
        zip.write_all(serde_json::to_string_pretty(&manifest).map_err(io_err)?.as_bytes()).map_err(io_err)?;

        zip.start_file(DB_FILE, deflate).map_err(io_err)?;
        io::copy(&mut File::open(snapshot).map_err(io_err)?, &mut zip).map_err(io_err)?;

        for entry in &image_files {
            // JPEGs are already compressed.
            zip.start_file(format!("{IMAGE_PREFIX}{}", entry.file_name().to_string_lossy()), stored).map_err(io_err)?;
            io::copy(&mut File::open(entry.path()).map_err(io_err)?, &mut zip).map_err(io_err)?;
        }
        zip.finish().map_err(io_err)?;
        Ok(())
    })();
    let _ = fs::remove_file(snapshot);
    match result {
        Ok(()) => {
            fs::rename(&partial, out).map_err(io_err)?;
            Ok(manifest)
        }
        Err(e) => {
            let _ = fs::remove_file(&partial);
            Err(e)
        }
    }
}

fn open_archive(path: &Path) -> Result<ZipArchive<File>, String> {
    ZipArchive::new(File::open(path).map_err(io_err)?).map_err(|_| INVALID.to_string())
}

/// Reads and checks a backup's manifest without changing anything.
pub fn inspect(path: &Path) -> Result<Manifest, String> {
    let mut zip = open_archive(path)?;
    let manifest: Manifest = {
        let mut text = String::new();
        zip.by_name(MANIFEST)
            .map_err(|_| INVALID.to_string())?
            .take(1024 * 1024)
            .read_to_string(&mut text)
            .map_err(|_| INVALID.to_string())?;
        serde_json::from_str(&text).map_err(|_| INVALID.to_string())?
    };
    if manifest.format > FORMAT {
        return Err(TOO_NEW.into());
    }
    zip.by_name(DB_FILE).map_err(|_| INVALID.to_string())?;
    Ok(manifest)
}

fn extract(zip: &mut ZipArchive<File>, name: &str, to: &Path) -> Result<(), String> {
    let entry = zip.by_name(name).map_err(|_| INVALID.to_string())?;
    if entry.size() > MAX_ENTRY_BYTES {
        return Err(INVALID.into());
    }
    let mut out = File::create(to).map_err(io_err)?;
    io::copy(&mut entry.take(MAX_ENTRY_BYTES), &mut out).map_err(io_err)?;
    Ok(())
}

/// Unpacks and validates a backup into `<data dir>/pending`. Applied on the next start.
pub fn stage_import(backup: &Path, data_dir: &Path) -> Result<Manifest, String> {
    let manifest = inspect(backup)?;
    let pending = data_dir.join(PENDING);
    let _ = fs::remove_dir_all(&pending);
    fs::create_dir_all(pending.join("images")).map_err(io_err)?;

    let result = (|| -> Result<(), String> {
        let mut zip = open_archive(backup)?;
        let db_path = pending.join(DB_FILE);
        extract(&mut zip, DB_FILE, &db_path)?;
        // Opening runs the schema migrations, so older backups are upgraded here.
        {
            let db = Db::open(&db_path).map_err(|_| INVALID.to_string())?;
            let ok: String = db.conn.query_row("PRAGMA integrity_check", [], |r| r.get(0)).map_err(|_| INVALID.to_string())?;
            if ok != "ok" {
                return Err(INVALID.into());
            }
            db.conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(())).map_err(io_err)?;
        }
        // Only image names we would have generated ourselves are unpacked.
        let names: Vec<String> = zip
            .file_names()
            .filter_map(|n| n.strip_prefix(IMAGE_PREFIX))
            .filter(|n| images::is_safe_name(n))
            .map(str::to_owned)
            .collect();
        for name in names {
            extract(&mut zip, &format!("{IMAGE_PREFIX}{name}"), &pending.join("images").join(&name))?;
        }
        fs::write(pending.join(READY), b"").map_err(io_err)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&pending);
    }
    result.map(|_| manifest)
}

/// Marks all data for deletion on the next start.
pub fn stage_reset(data_dir: &Path) -> Result<(), String> {
    let pending = data_dir.join(PENDING);
    let _ = fs::remove_dir_all(&pending);
    fs::create_dir_all(&pending).map_err(io_err)?;
    fs::write(pending.join(RESET), b"").map_err(io_err)
}

fn remove_live_data(data_dir: &Path) -> io::Result<()> {
    for suffix in ["", "-wal", "-shm"] {
        match fs::remove_file(data_dir.join(format!("{DB_FILE}{suffix}"))) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
    }
    match fs::remove_dir_all(data_dir.join("images")) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Called at startup before the database is opened: applies a staged import or reset.
pub fn apply_pending(data_dir: &Path) -> io::Result<()> {
    let pending = data_dir.join(PENDING);
    if !pending.exists() {
        return Ok(());
    }
    if pending.join(RESET).exists() {
        remove_live_data(data_dir)?;
    } else if pending.join(READY).exists() {
        remove_live_data(data_dir)?;
        fs::rename(pending.join(DB_FILE), data_dir.join(DB_FILE))?;
        fs::rename(pending.join("images"), data_dir.join("images"))?;
    }
    // An incomplete staging (no marker) is simply discarded.
    fs::remove_dir_all(&pending)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!("nora_backup_{tag}_{}_{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn export(dir: &Path, out: &Path) -> Manifest {
        let db = Db::open(&dir.join(DB_FILE)).unwrap();
        let scratch = dir.join("snapshot.tmp");
        let counts = prepare_export(&db, &scratch).unwrap();
        write_backup(&scratch, &dir.join("images"), counts, out).unwrap()
    }

    #[test]
    fn export_then_import_restores_data_and_images() {
        let old = temp_dir("old");
        {
            let db = Db::open(&old.join(DB_FILE)).unwrap();
            db.add_todo(None, None, "Pack bags", None, None).unwrap();
            db.set_setting("ui.locale", "ko").unwrap();
        }
        let image = images::store(&old.join("images"), b"jpeg bytes", "jpg").unwrap();
        let backup = old.join("Nora-backup.nora");
        let manifest = export(&old, &backup);
        assert_eq!((manifest.counts.todos, manifest.counts.images), (1, 1));
        assert!(!old.join("snapshot.tmp").exists(), "the snapshot is cleaned up");

        // A fresh install with its own (different) data.
        let new = temp_dir("new");
        Db::open(&new.join(DB_FILE)).unwrap().add_todo(None, None, "Other", None, None).unwrap();
        assert_eq!(inspect(&backup).unwrap().counts.todos, 1);
        stage_import(&backup, &new).unwrap();
        apply_pending(&new).unwrap();

        let db = Db::open(&new.join(DB_FILE)).unwrap();
        assert_eq!(db.todos().unwrap().iter().map(|t| t.title.as_str()).collect::<Vec<_>>(), ["Pack bags"]);
        assert_eq!(db.setting("ui.locale").unwrap().as_deref(), Some("ko"));
        assert_eq!(fs::read(new.join("images").join(&image)).unwrap(), b"jpeg bytes");
        assert!(!new.join(PENDING).exists());
        let _ = fs::remove_dir_all(old);
        let _ = fs::remove_dir_all(new);
    }

    #[test]
    fn reset_removes_everything_on_next_start() {
        let dir = temp_dir("reset");
        Db::open(&dir.join(DB_FILE)).unwrap().add_todo(None, None, "x", None, None).unwrap();
        images::store(&dir.join("images"), b"x", "jpg").unwrap();
        stage_reset(&dir).unwrap();
        assert!(dir.join(DB_FILE).exists(), "nothing is deleted until the restart");
        apply_pending(&dir).unwrap();
        assert!(!dir.join(DB_FILE).exists() && !dir.join("images").exists() && !dir.join(PENDING).exists());
        assert!(Db::open(&dir.join(DB_FILE)).unwrap().todos().unwrap().is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_files_that_are_not_backups_and_ignores_unsafe_entries() {
        let dir = temp_dir("bad");
        let junk = dir.join("junk.nora");
        fs::write(&junk, b"not a zip").unwrap();
        assert_eq!(inspect(&junk).unwrap_err(), INVALID);
        assert_eq!(stage_import(&junk, &dir).unwrap_err(), INVALID);
        assert!(!dir.join(PENDING).exists());

        // A real backup with an extra malicious path inside.
        let src = temp_dir("src");
        Db::open(&src.join(DB_FILE)).unwrap();
        let backup = src.join("b.nora");
        export(&src, &backup);
        {
            let mut zip = zip::ZipWriter::new_append(fs::OpenOptions::new().read(true).write(true).open(&backup).unwrap()).unwrap();
            zip.start_file("images/../../evil.jpg", SimpleFileOptions::default()).unwrap();
            zip.write_all(b"evil").unwrap();
            zip.finish().unwrap();
        }
        stage_import(&backup, &dir).unwrap();
        apply_pending(&dir).unwrap();
        assert!(!dir.join("evil.jpg").exists() && !dir.parent().unwrap().join("evil.jpg").exists());
        let _ = fs::remove_dir_all(dir);
        let _ = fs::remove_dir_all(src);
    }
}
