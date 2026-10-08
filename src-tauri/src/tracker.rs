//! Background working-time tracker.
//!
//! A dedicated thread samples the focused application/window once per second and
//! records contiguous focus "segments" into the `activity` table. Browser tabs are
//! captured through the window title, which browsers set to the active tab's title.
//! Time is not counted while paused, while the user is idle, or for ignored apps.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db;

const MAX_TITLE_LEN: usize = 300;
/// Samples further apart than this (e.g. after system sleep) start a new segment.
const MAX_GAP_SECS: i64 = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackState {
    #[default]
    Starting,
    Tracking,
    Idle,
    Paused,
    Ignored,
    Unavailable,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TrackerStatus {
    pub state: TrackState,
    pub app: String,
    pub title: String,
    /// How long the current app/window has been focused, in seconds.
    pub segment_secs: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackerSettings {
    pub paused: bool,
    pub idle_threshold_secs: u64,
    pub ignored_apps: Vec<String>,
}

impl TrackerSettings {
    pub fn load(db: &db::Db) -> Self {
        Self {
            paused: db.setting_or("tracker.paused", false),
            idle_threshold_secs: db.setting_or("tracker.idle_secs", 300),
            ignored_apps: db
                .setting("tracker.ignored")
                .ok()
                .flatten()
                .map(|s| s.lines().map(str::to_owned).filter(|l| !l.is_empty()).collect())
                .unwrap_or_default(),
        }
    }

    pub fn save(&self, db: &db::Db) -> db::DbResult<()> {
        db.set_setting("tracker.paused", &self.paused.to_string())?;
        db.set_setting("tracker.idle_secs", &self.idle_threshold_secs.to_string())?;
        db.set_setting("tracker.ignored", &self.ignored_apps.join("\n"))
    }
}

/// State shared between the UI and the tracker thread.
pub struct TrackerShared {
    pub paused: AtomicBool,
    /// 0 disables idle detection.
    pub idle_threshold_secs: AtomicU64,
    pub ignored_apps: Mutex<Vec<String>>,
    pub status: Mutex<TrackerStatus>,
}

impl TrackerShared {
    pub fn settings(&self) -> TrackerSettings {
        TrackerSettings {
            paused: self.paused.load(Ordering::Relaxed),
            idle_threshold_secs: self.idle_threshold_secs.load(Ordering::Relaxed),
            ignored_apps: self.ignored_apps.lock().map(|l| l.clone()).unwrap_or_default(),
        }
    }

    pub fn apply(&self, settings: TrackerSettings) {
        self.paused.store(settings.paused, Ordering::Relaxed);
        self.idle_threshold_secs.store(settings.idle_threshold_secs, Ordering::Relaxed);
        if let Ok(mut list) = self.ignored_apps.lock() {
            *list = settings.ignored_apps;
        }
    }

    pub fn status(&self) -> TrackerStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }

    fn set_status(&self, status: TrackerStatus) {
        if let Ok(mut s) = self.status.lock() {
            *s = status;
        }
    }

    fn is_ignored(&self, app: &str) -> bool {
        self.ignored_apps
            .lock()
            .map(|list| list.iter().any(|a| a.eq_ignore_ascii_case(app)))
            .unwrap_or(false)
    }
}

pub fn start(db_path: PathBuf, settings: TrackerSettings) -> Arc<TrackerShared> {
    let shared = Arc::new(TrackerShared {
        paused: AtomicBool::new(settings.paused),
        idle_threshold_secs: AtomicU64::new(settings.idle_threshold_secs),
        ignored_apps: Mutex::new(settings.ignored_apps),
        status: Mutex::new(TrackerStatus::default()),
    });
    let thread_shared = shared.clone();
    let spawned = std::thread::Builder::new()
        .name("focus-tracker".into())
        .spawn(move || run(db_path, thread_shared));
    if let Err(e) = spawned {
        eprintln!("failed to start focus tracker: {e}");
    }
    shared
}

struct Segment {
    id: i64,
    app: String,
    title: String,
    start: i64,
    last: i64,
}

fn run(db_path: PathBuf, shared: Arc<TrackerShared>) {
    let conn = match db::open_conn(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("focus tracker could not open database: {e}");
            shared.set_status(TrackerStatus { state: TrackState::Unavailable, ..Default::default() });
            return;
        }
    };
    let mut seg: Option<Segment> = None;

    loop {
        std::thread::sleep(Duration::from_secs(1));
        let now = db::now_ts();

        if shared.paused.load(Ordering::Relaxed) {
            seg = None;
            shared.set_status(TrackerStatus { state: TrackState::Paused, ..Default::default() });
            continue;
        }

        let threshold = shared.idle_threshold_secs.load(Ordering::Relaxed);
        if threshold > 0
            && let Some(idle) = idle_seconds()
            && idle >= threshold as f64
        {
            // Give back the idle tail that was counted before the threshold was hit.
            if let Some(s) = seg.take() {
                let end = (now - idle as i64).max(s.start);
                let _ = conn.execute("UPDATE activity SET end=?1 WHERE id=?2", params![end, s.id]);
            }
            shared.set_status(TrackerStatus { state: TrackState::Idle, ..Default::default() });
            continue;
        }

        let Ok(win) = active_win_pos_rs::get_active_window() else {
            seg = None;
            shared.set_status(TrackerStatus { state: TrackState::Unavailable, ..Default::default() });
            continue;
        };
        let app = if win.app_name.trim().is_empty() {
            win.process_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Unknown".to_owned())
        } else {
            win.app_name.trim().to_owned()
        };
        let title: String = win.title.trim().chars().take(MAX_TITLE_LEN).collect();

        if shared.is_ignored(&app) {
            seg = None;
            shared.set_status(TrackerStatus { state: TrackState::Ignored, app, title, segment_secs: 0 });
            continue;
        }

        let continues = matches!(&seg, Some(s) if s.app == app && s.title == title && now - s.last <= MAX_GAP_SECS);
        if continues {
            let s = seg.as_mut().unwrap();
            s.last = now;
            let _ = conn.execute("UPDATE activity SET end=?1 WHERE id=?2", params![now, s.id]);
        } else {
            // Each sample accounts for the second that just elapsed.
            let start = now - 1;
            let inserted = conn.execute(
                "INSERT INTO activity(app, title, start, end) VALUES(?1, ?2, ?3, ?4)",
                params![app, title, start, now],
            );
            seg = inserted.ok().map(|_| Segment {
                id: conn.last_insert_rowid(),
                app: app.clone(),
                title: title.clone(),
                start,
                last: now,
            });
        }

        let segment_secs = seg.as_ref().map_or(0, |s| s.last - s.start);
        shared.set_status(TrackerStatus { state: TrackState::Tracking, app, title, segment_secs });
    }
}

/// Seconds since the last keyboard/mouse input, if the platform supports it.
#[cfg(target_os = "macos")]
fn idle_seconds() -> Option<f64> {
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventSourceSecondsSinceLastEventType(source_state: i32, event_type: u32) -> f64;
    }
    const HID_SYSTEM_STATE: i32 = 1;
    const ANY_INPUT_EVENT: u32 = u32::MAX;
    Some(unsafe { CGEventSourceSecondsSinceLastEventType(HID_SYSTEM_STATE, ANY_INPUT_EVENT) })
}

#[cfg(target_os = "windows")]
fn idle_seconds() -> Option<f64> {
    #[repr(C)]
    struct LastInputInfo {
        cb_size: u32,
        dw_time: u32,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetLastInputInfo(plii: *mut LastInputInfo) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetTickCount() -> u32;
    }
    let mut info = LastInputInfo { cb_size: std::mem::size_of::<LastInputInfo>() as u32, dw_time: 0 };
    unsafe {
        if GetLastInputInfo(&mut info) == 0 {
            return None;
        }
        Some(GetTickCount().wrapping_sub(info.dw_time) as f64 / 1000.0)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn idle_seconds() -> Option<f64> {
    None
}
