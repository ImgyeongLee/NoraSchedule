#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backup;
mod bookmarks;
mod commands;
mod db;
mod expenses;
mod health;
mod images;
mod reading;
mod recurrence;
mod reminders;
mod tracker;
mod trpg;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::Manager;
use tauri_plugin_window_state::StateFlags;

pub struct AppState {
    pub db: Mutex<db::Db>,
    pub tracker: Arc<tracker::TrackerShared>,
    pub images_dir: PathBuf,
    /// Folder holding the database and images (backups are staged here).
    pub data_dir: PathBuf,
}

/// Serves stored D-Day images to the webview as `noraimg://localhost/<file>`
/// (`http://noraimg.localhost/<file>` on Windows). Only names we generated are served.
fn serve_image(app: &tauri::AppHandle, request: tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    let name = request.uri().path().trim_start_matches('/');
    let file = app
        .try_state::<AppState>()
        .filter(|_| images::is_safe_name(name))
        .and_then(|state| std::fs::read(state.images_dir.join(name)).ok());
    let builder = tauri::http::Response::builder();
    match file {
        Some(bytes) => builder
            .header("Content-Type", images::content_type(name))
            .header("Cache-Control", "max-age=31536000, immutable")
            .body(bytes),
        None => builder.status(404).body(Vec::new()),
    }
    .unwrap_or_default()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        // Reopen at the size and position the window had when the app was last closed.
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED)
                .build(),
        )
        .register_uri_scheme_protocol("noraimg", |ctx, request| serve_image(ctx.app_handle(), request))
        .setup(|app| {
            // NORA_DATA_DIR lets you point the app at a scratch database while developing.
            let dir = match std::env::var_os("NORA_DATA_DIR") {
                Some(dir) => dir.into(),
                None => app.path().app_data_dir()?,
            };
            std::fs::create_dir_all(&dir)?;
            // Finish an import or "delete all data" requested before the restart.
            if let Err(e) = backup::apply_pending(&dir) {
                eprintln!("could not apply pending import/reset: {e}");
            }
            let db_path = dir.join(backup::DB_FILE);
            let db = db::Db::open(&db_path)?;
            let images_dir = dir.join("images");
            // Clean up images left behind by an interrupted edit.
            if let Ok(keep) = db.referenced_images() {
                images::collect_garbage(&images_dir, &keep);
            }
            reminders::start(app.handle().clone(), db_path.clone());
            let tracker = tracker::start(db_path, tracker::TrackerSettings::load(&db));
            app.manage(AppState { db: Mutex::new(db), tracker, images_dir, data_dir: dir });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::set_titlebar_colors,
            commands::get_setting,
            commands::export_data,
            commands::inspect_backup,
            commands::import_data,
            commands::reset_all_data,
            commands::bookmark_folders,
            commands::save_bookmark_folder,
            commands::delete_bookmark_folder,
            commands::bookmarks,
            commands::save_bookmark,
            commands::delete_bookmark,
            commands::reorder_bookmarks,
            commands::expenses_between,
            commands::save_expense,
            commands::delete_expense,
            commands::meals_between,
            commands::save_meal,
            commands::delete_meal,
            commands::workouts_between,
            commands::save_workout,
            commands::delete_workout,
            commands::trpg_entries,
            commands::save_trpg_entry,
            commands::delete_trpg_entry,
            commands::books,
            commands::save_book,
            commands::delete_book,
            commands::set_setting,
            commands::events_between,
            commands::save_event,
            commands::delete_event,
            commands::restore_occurrence,
            commands::tags,
            commands::save_tag,
            commands::delete_tag,
            commands::ddays,
            commands::save_dday,
            commands::delete_dday,
            commands::import_image,
            commands::remove_unused_images,
            commands::todo_groups,
            commands::add_todo_group,
            commands::update_todo_group,
            commands::delete_todo_group,
            commands::todos,
            commands::add_todo,
            commands::update_todo,
            commands::set_todo_done,
            commands::delete_todo,
            commands::memos,
            commands::create_memo,
            commands::set_memo_group,
            commands::memo_groups,
            commands::save_memo_group,
            commands::delete_memo_group,
            commands::update_memo,
            commands::delete_memo,
            commands::add_pomodoro_session,
            commands::pomodoro_sessions,
            commands::activity_summary,
            commands::activity_timeline,
            commands::tracker_status,
            commands::tracker_settings,
            commands::set_tracker_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Nora Schedule");
}
