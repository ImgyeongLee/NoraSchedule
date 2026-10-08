//! Event reminders.
//!
//! A background thread looks for events whose reminder time ("N minutes before") has
//! arrived and sends each one to the web frontend as a `reminder` event; the frontend
//! shows the notification (in the user's language) and plays the chime. Running here
//! rather than in the webview keeps reminders on time while the window is minimized.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use chrono::{Local, NaiveDateTime, NaiveTime};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::db::{self, Event};

const CHECK_EVERY: Duration = Duration::from_secs(15);
/// The longest reminder offered (a week), which bounds how far ahead to look.
const MAX_REMINDER_DAYS: i64 = 7;

#[derive(Clone, Debug, Serialize)]
pub struct Reminder {
    pub event: Event,
    /// When the event starts (09:00 on the day for all-day events).
    pub starts_at: NaiveDateTime,
}

/// The moment a reminder counts down to. All-day events are treated as starting at 09:00.
fn starts_at(e: &Event) -> NaiveDateTime {
    if e.all_day { e.start.date().and_time(NaiveTime::from_hms_opt(9, 0, 0).unwrap()) } else { e.start }
}

/// Reminders due at `now` that are not in `fired` yet (and records them there).
fn due(events: Vec<Event>, now: NaiveDateTime, fired: &mut HashSet<(i64, NaiveDateTime)>) -> Vec<Reminder> {
    events
        .into_iter()
        .filter(|e| !e.cancelled)
        .filter_map(|e| {
            let minutes = e.reminder?;
            let start = starts_at(&e);
            let at = start - chrono::Duration::minutes(minutes);
            // Still useful until the event starts (e.g. the app was opened late).
            (at <= now && now < start && fired.insert((e.id, start))).then_some(Reminder { event: e, starts_at: start })
        })
        .collect()
}

pub fn start(app: AppHandle, db_path: PathBuf) {
    let spawned = std::thread::Builder::new().name("reminders".into()).spawn(move || {
        let db = match db::open_conn(&db_path) {
            Ok(conn) => db::Db { conn },
            Err(e) => return eprintln!("reminders could not open database: {e}"),
        };
        let mut fired = HashSet::new();
        loop {
            let now = Local::now().naive_local();
            let today = now.date();
            match db.events_between(today, today + chrono::Duration::days(MAX_REMINDER_DAYS + 1)) {
                Ok(events) => {
                    for reminder in due(events, now, &mut fired) {
                        let _ = app.emit("reminder", reminder);
                    }
                }
                Err(e) => eprintln!("reminders could not read events: {e}"),
            }
            std::thread::sleep(CHECK_EVERY);
        }
    });
    if let Err(e) = spawned {
        eprintln!("failed to start reminders: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    fn event(start: &str, reminder: Option<i64>, all_day: bool) -> Event {
        Event {
            id: 1, title: "Meeting".into(), start: dt(start), end: dt(start), all_day, color: 0,
            location: String::new(), links: vec![], memo: String::new(), repeat: None, exdates: vec![],
            occurrence: None, cancelled: false, tags: vec![], reminder,
        }
    }

    #[test]
    fn fires_once_between_the_reminder_time_and_the_start() {
        let mut fired = HashSet::new();
        let ev = || vec![event("2026-10-08 10:00", Some(10), false)];
        assert!(due(ev(), dt("2026-10-08 09:49"), &mut fired).is_empty());
        assert_eq!(due(ev(), dt("2026-10-08 09:50"), &mut fired).len(), 1);
        assert!(due(ev(), dt("2026-10-08 09:55"), &mut fired).is_empty());
        let mut fresh = HashSet::new();
        assert!(due(ev(), dt("2026-10-08 10:00"), &mut fresh).is_empty());
    }

    #[test]
    fn skips_cancelled_events_and_events_without_a_reminder() {
        let mut fired = HashSet::new();
        let mut cancelled = event("2026-10-08 10:00", Some(30), false);
        cancelled.cancelled = true;
        let none = event("2026-10-08 10:00", None, false);
        assert!(due(vec![cancelled, none], dt("2026-10-08 09:45"), &mut fired).is_empty());
    }

    #[test]
    fn all_day_events_count_down_to_nine_in_the_morning() {
        let mut fired = HashSet::new();
        let ev = vec![event("2026-10-08 00:00", Some(60), true)];
        assert_eq!(due(ev, dt("2026-10-08 08:00"), &mut fired)[0].starts_at, dt("2026-10-08 09:00"));
    }
}
