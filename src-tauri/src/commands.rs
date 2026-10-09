//! Tauri commands: the API the web frontend calls through `invoke`.
//! Argument names are camelCase on the JavaScript side (`groupId` → `group_id`).

use chrono::NaiveDate;
use tauri::State;
use tauri::ipc::{InvokeBody, Request};

use crate::AppState;
use crate::backup::{self, Manifest};
use crate::bookmarks::{Bookmark, BookmarkFolder};
use crate::expenses::Expense;
use crate::health::{self, Meal, Workout};
use crate::db::{ActivitySummary, Activity, DDay, Event, Memo, MemoGroup, PomodoroSession, Scope, Tag, Todo, TodoGroup};
use crate::tracker::{TrackerSettings, TrackerStatus};
use crate::reading::Book;
use crate::trpg::TrpgEntry;

type CmdResult<T> = Result<T, String>;

fn with_db<T>(state: &State<AppState>, f: impl FnOnce(&crate::db::Db) -> crate::db::DbResult<T>) -> CmdResult<T> {
    let db = state.db.lock().map_err(|_| "database lock poisoned".to_string())?;
    f(&db).map_err(|e| e.to_string())
}

// ---- window -----------------------------------------------------------------

/// Paints the native title bar in the app theme's colors (`0xRRGGBB`).
/// Windows 11 only; elsewhere (and on older Windows) this does nothing.
#[tauri::command]
pub fn set_titlebar_colors(window: tauri::WebviewWindow, caption: u32, text: u32, dark: bool) -> CmdResult<()> {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;
        #[link(name = "dwmapi")]
        unsafe extern "system" {
            fn DwmSetWindowAttribute(hwnd: isize, attribute: u32, value: *const c_void, size: u32) -> i32;
        }
        const USE_IMMERSIVE_DARK_MODE: u32 = 20;
        const BORDER_COLOR: u32 = 34;
        const CAPTION_COLOR: u32 = 35;
        const TEXT_COLOR: u32 = 36;
        // COLORREF is 0x00BBGGRR.
        let colorref = |rgb: u32| ((rgb & 0xff) << 16) | (rgb & 0xff00) | ((rgb >> 16) & 0xff);
        let hwnd = window.hwnd().map_err(|e| e.to_string())?.0 as isize;
        let set = |attribute: u32, value: u32| unsafe {
            DwmSetWindowAttribute(hwnd, attribute, &value as *const u32 as *const c_void, 4);
        };
        set(USE_IMMERSIVE_DARK_MODE, dark as u32);
        set(CAPTION_COLOR, colorref(caption));
        set(BORDER_COLOR, colorref(caption));
        set(TEXT_COLOR, colorref(text));
    }
    #[cfg(not(target_os = "windows"))]
    let _ = (window, caption, text, dark);
    Ok(())
}

// ---- settings ---------------------------------------------------------------

#[tauri::command]
pub fn get_setting(state: State<AppState>, key: String) -> CmdResult<Option<String>> {
    with_db(&state, |db| db.setting(&key))
}

#[tauri::command]
pub fn set_setting(state: State<AppState>, key: String, value: String) -> CmdResult<()> {
    with_db(&state, |db| db.set_setting(&key, &value))
}

// ---- events -------------------------------------------------------------------

#[tauri::command]
pub fn events_between(state: State<AppState>, from: NaiveDate, to: NaiveDate) -> CmdResult<Vec<Event>> {
    with_db(&state, |db| db.events_between(from, to))
}

#[tauri::command]
pub fn save_event(state: State<AppState>, event: Event, scope: Option<Scope>) -> CmdResult<i64> {
    if event.end < event.start {
        return Err("The event must end after it starts.".into());
    }
    with_db(&state, |db| db.save_event(&event, scope.unwrap_or(Scope::All)))
}

/// Deletes an event (or one occurrence of a repeating event) and returns the stored row for undo.
#[tauri::command]
pub fn delete_event(state: State<AppState>, id: i64, occurrence: Option<NaiveDate>, scope: Option<Scope>) -> CmdResult<Option<Event>> {
    with_db(&state, |db| db.delete_event(id, occurrence, scope.unwrap_or(Scope::All)))
}

#[tauri::command]
pub fn restore_occurrence(state: State<AppState>, id: i64, day: NaiveDate) -> CmdResult<()> {
    with_db(&state, |db| db.restore_occurrence(id, day))
}

// ---- tags ---------------------------------------------------------------------

#[tauri::command]
pub fn tags(state: State<AppState>) -> CmdResult<Vec<Tag>> {
    with_db(&state, |db| db.tags())
}

#[tauri::command]
pub fn save_tag(state: State<AppState>, mut tag: Tag) -> CmdResult<i64> {
    tag.name = tag.name.trim().to_owned();
    tag.category = tag.category.trim().to_owned();
    if tag.name.is_empty() {
        return Err("a tag needs a name".into());
    }
    with_db(&state, |db| db.save_tag(&tag))
}

#[tauri::command]
pub fn delete_tag(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_db(&state, |db| db.delete_tag(id))
}

// ---- d-days -------------------------------------------------------------------

#[tauri::command]
pub fn ddays(state: State<AppState>) -> CmdResult<Vec<DDay>> {
    with_db(&state, |db| db.ddays())
}

#[tauri::command]
pub fn save_dday(state: State<AppState>, mut dday: DDay) -> CmdResult<()> {
    dday.image = dday.image.filter(|name| crate::images::is_safe_name(name));
    if !DDay::SHAPES.contains(&dday.shape.as_str()) {
        dday.shape = DDay::SHAPES[0].into();
    }
    with_db(&state, |db| db.save_dday(&dday))?;
    // A replaced or removed cover image is no longer referenced: delete the file.
    remove_unused_images(state)
}

#[tauri::command]
pub fn delete_dday(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_db(&state, |db| db.delete_dday(id))?;
    remove_unused_images(state)
}

/// Optimizes an uploaded image (sent as raw bytes) and stores it. Returns the file name.
/// The `purpose` header (`cover`, `card`, `header` or `sticker`) sets the size and format.
/// The file is kept only once something references it; see `remove_unused_images`.
#[tauri::command]
pub async fn import_image(state: State<'_, AppState>, request: Request<'_>) -> CmdResult<String> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("expected raw image bytes".into());
    };
    let bytes = bytes.clone();
    let purpose = request
        .headers()
        .get("purpose")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| serde_json::from_value(serde_json::Value::String(v.to_owned())).ok())
        .unwrap_or_default();
    let dir = state.images_dir.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (file, ext) = crate::images::optimize(&bytes, purpose).map_err(|e| e.to_string())?;
        crate::images::store(&dir, &file, ext).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Deletes stored images no D-Day uses (e.g. after the D-Day editor was cancelled).
#[tauri::command]
pub fn remove_unused_images(state: State<AppState>) -> CmdResult<()> {
    let keep = with_db(&state, |db| db.referenced_images())?;
    crate::images::collect_garbage(&state.images_dir, &keep);
    Ok(())
}

// ---- todos --------------------------------------------------------------------

#[tauri::command]
pub fn todo_groups(state: State<AppState>) -> CmdResult<Vec<TodoGroup>> {
    with_db(&state, |db| db.todo_groups())
}

#[tauri::command]
pub fn add_todo_group(state: State<AppState>, name: String, color: u32) -> CmdResult<i64> {
    with_db(&state, |db| db.add_todo_group(name.trim(), color))
}

#[tauri::command]
pub fn update_todo_group(state: State<AppState>, group: TodoGroup) -> CmdResult<()> {
    with_db(&state, |db| db.update_todo_group(&group))
}

#[tauri::command]
pub fn delete_todo_group(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_db(&state, |db| db.delete_todo_group(id))
}

#[tauri::command]
pub fn todos(state: State<AppState>) -> CmdResult<Vec<Todo>> {
    with_db(&state, |db| db.todos())
}

#[tauri::command]
pub fn add_todo(
    state: State<AppState>,
    group_id: Option<i64>,
    parent_id: Option<i64>,
    title: String,
    due: Option<NaiveDate>,
    due_time: Option<String>,
) -> CmdResult<i64> {
    with_db(&state, |db| db.add_todo(group_id, parent_id, title.trim(), due, due_time.as_deref()))
}

#[tauri::command]
pub fn update_todo(state: State<AppState>, todo: Todo) -> CmdResult<()> {
    with_db(&state, |db| db.update_todo(&todo))
}

#[tauri::command]
pub fn set_todo_done(state: State<AppState>, id: i64, done: bool) -> CmdResult<()> {
    with_db(&state, |db| db.set_todo_done(id, done))
}

#[tauri::command]
pub fn delete_todo(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_db(&state, |db| db.delete_todo(id))
}

// ---- memos --------------------------------------------------------------------

#[tauri::command]
pub fn memos(state: State<AppState>) -> CmdResult<Vec<Memo>> {
    with_db(&state, |db| db.memos())
}

#[tauri::command]
pub fn create_memo(state: State<AppState>, title: String, group_id: Option<i64>) -> CmdResult<i64> {
    with_db(&state, |db| db.create_memo(&title, group_id))
}

#[tauri::command]
pub fn set_memo_group(state: State<AppState>, id: i64, group_id: Option<i64>) -> CmdResult<()> {
    with_db(&state, |db| db.set_memo_group(id, group_id))
}

#[tauri::command]
pub fn memo_groups(state: State<AppState>) -> CmdResult<Vec<MemoGroup>> {
    with_db(&state, |db| db.memo_groups())
}

#[tauri::command]
pub fn save_memo_group(state: State<AppState>, group: MemoGroup) -> CmdResult<i64> {
    let name = group.name.trim();
    if name.is_empty() {
        return Err("A group needs a name.".into());
    }
    with_db(&state, |db| db.save_memo_group(&MemoGroup { name: name.into(), ..group }))
}

#[tauri::command]
pub fn delete_memo_group(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_db(&state, |db| db.delete_memo_group(id))
}

#[tauri::command]
pub fn update_memo(state: State<AppState>, id: i64, title: String, body: String) -> CmdResult<()> {
    with_db(&state, |db| db.update_memo(id, &title, &body))
}

#[tauri::command]
pub fn delete_memo(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_db(&state, |db| db.delete_memo(id))
}

// ---- pomodoro -----------------------------------------------------------------

#[tauri::command]
pub fn add_pomodoro_session(state: State<AppState>, started_at: i64, ended_at: i64, label: String) -> CmdResult<()> {
    with_db(&state, |db| db.add_pomodoro_session(started_at, ended_at, label.trim()))
}

#[tauri::command]
pub fn pomodoro_sessions(state: State<AppState>, from_ts: i64, to_ts: i64) -> CmdResult<Vec<PomodoroSession>> {
    with_db(&state, |db| db.pomodoro_sessions(from_ts, to_ts))
}

// ---- working time -------------------------------------------------------------

#[tauri::command]
pub fn activity_summary(state: State<AppState>, days: Vec<NaiveDate>, title_limit: usize) -> CmdResult<ActivitySummary> {
    with_db(&state, |db| db.activity_summary(&days, title_limit))
}

#[tauri::command]
pub fn activity_timeline(state: State<AppState>, day: NaiveDate) -> CmdResult<Vec<Activity>> {
    with_db(&state, |db| db.activity_timeline(day))
}

#[tauri::command]
pub fn tracker_status(state: State<AppState>) -> TrackerStatus {
    state.tracker.status()
}

#[tauri::command]
pub fn tracker_settings(state: State<AppState>) -> TrackerSettings {
    state.tracker.settings()
}

#[tauri::command]
pub fn set_tracker_settings(state: State<AppState>, mut settings: TrackerSettings) -> CmdResult<()> {
    settings.tracked_apps.retain(|a| !a.trim().is_empty());
    settings.tracked_apps.sort();
    settings.tracked_apps.dedup();
    with_db(&state, |db| settings.save(db))?;
    state.tracker.apply(settings);
    Ok(())
}

// ---- bookmarks ------------------------------------------------------------------

#[tauri::command]
pub fn bookmark_folders(state: State<AppState>) -> CmdResult<Vec<BookmarkFolder>> {
    with_db(&state, |db| db.bookmark_folders())
}

#[tauri::command]
pub fn save_bookmark_folder(state: State<AppState>, folder: BookmarkFolder) -> CmdResult<i64> {
    let name = folder.name.trim();
    if name.is_empty() {
        return Err("A folder needs a name.".into());
    }
    // A folder cannot be moved inside itself.
    if folder.id != 0
        && let Some(parent) = folder.parent_id
        && with_db(&state, |db| db.folder_is_within(parent, folder.id))?
    {
        return Err("folder_cycle".into());
    }
    with_db(&state, |db| db.save_bookmark_folder(&BookmarkFolder { name: name.into(), ..folder }))
}

#[tauri::command]
pub fn delete_bookmark_folder(state: State<AppState>, id: i64) -> CmdResult<()> {
    with_db(&state, |db| db.delete_bookmark_folder(id))
}

#[tauri::command]
pub fn bookmarks(state: State<AppState>) -> CmdResult<Vec<Bookmark>> {
    with_db(&state, |db| db.bookmarks())
}

#[tauri::command]
pub fn save_bookmark(state: State<AppState>, bookmark: Bookmark) -> CmdResult<i64> {
    let url = bookmark.url.trim();
    if url.is_empty() || url.to_ascii_lowercase().starts_with("javascript:") {
        return Err("invalid_url".into());
    }
    with_db(&state, |db| db.save_bookmark(&Bookmark { url: url.into(), title: bookmark.title.trim().into(), ..bookmark }))
}

#[tauri::command]
pub fn reorder_bookmarks(state: State<AppState>, ids: Vec<i64>) -> CmdResult<()> {
    with_db(&state, |db| db.reorder_bookmarks(&ids))
}

#[tauri::command]
pub fn delete_bookmark(state: State<AppState>, id: i64) -> CmdResult<Option<Bookmark>> {
    with_db(&state, |db| db.delete_bookmark(id))
}

// ---- TRPG log -------------------------------------------------------------------

#[tauri::command]
pub fn trpg_entries(state: State<AppState>) -> CmdResult<Vec<TrpgEntry>> {
    with_db(&state, |db| db.trpg_entries())
}

#[tauri::command]
pub fn save_trpg_entry(state: State<AppState>, entry: TrpgEntry) -> CmdResult<i64> {
    let title = entry.title.trim();
    if title.is_empty() {
        return Err("title required".into());
    }
    if !TrpgEntry::KINDS.contains(&entry.kind.as_str()) {
        return Err("invalid entry".into());
    }
    let entry = TrpgEntry {
        title: title.into(),
        role: TrpgEntry::clean_roles(&entry.role),
        folder: entry.folder.trim().chars().take(60).collect(),
        writer: entry.writer.trim().into(),
        system: entry.system.trim().into(),
        memo: entry.memo.trim().into(),
        image: entry.image.filter(|name| crate::images::is_safe_name(name)),
        ..entry
    };
    let id = with_db(&state, |db| db.save_trpg_entry(&entry))?;
    // A replaced or removed cover is no longer referenced: delete the file.
    remove_unused_images(state)?;
    Ok(id)
}

#[tauri::command]
pub fn delete_trpg_entry(state: State<AppState>, id: i64) -> CmdResult<Option<TrpgEntry>> {
    with_db(&state, |db| db.delete_trpg_entry(id))
}

// ---- reading log ------------------------------------------------------------------

#[tauri::command]
pub fn books(state: State<AppState>) -> CmdResult<Vec<Book>> {
    with_db(&state, |db| db.books())
}

#[tauri::command]
pub fn save_book(state: State<AppState>, book: Book) -> CmdResult<i64> {
    let title = book.title.trim();
    if title.is_empty() {
        return Err("title required".into());
    }
    if !Book::STATUSES.contains(&book.status.as_str()) {
        return Err("invalid book".into());
    }
    let book = Book {
        title: title.into(),
        author: book.author.trim().into(),
        publisher: book.publisher.trim().into(),
        image: book.image.filter(|name| crate::images::is_safe_name(name)),
        rating: book.rating.min(Book::MAX_RATING),
        // Can't have read past the last page.
        current_page: if book.total_pages > 0 { book.current_page.min(book.total_pages) } else { book.current_page },
        ..book
    };
    let id = with_db(&state, |db| db.save_book(&book))?;
    // A replaced or removed cover is no longer referenced: delete the file.
    remove_unused_images(state)?;
    Ok(id)
}

#[tauri::command]
pub fn delete_book(state: State<AppState>, id: i64) -> CmdResult<Option<Book>> {
    with_db(&state, |db| db.delete_book(id))
}

// ---- expenses -------------------------------------------------------------------

#[tauri::command]
pub fn expenses_between(state: State<AppState>, from: NaiveDate, to: NaiveDate) -> CmdResult<Vec<Expense>> {
    with_db(&state, |db| db.expenses_between(from, to))
}

#[tauri::command]
pub fn save_expense(state: State<AppState>, expense: Expense) -> CmdResult<i64> {
    if !expense.amount.is_finite() || expense.amount <= 0.0 {
        return Err("invalid_amount".into());
    }
    with_db(&state, |db| db.save_expense(&expense))
}

#[tauri::command]
pub fn delete_expense(state: State<AppState>, id: i64) -> CmdResult<Option<Expense>> {
    with_db(&state, |db| db.delete_expense(id))
}

// ---- diet and exercise -----------------------------------------------------------

#[tauri::command]
pub fn meals_between(state: State<AppState>, from: NaiveDate, to: NaiveDate) -> CmdResult<Vec<Meal>> {
    with_db(&state, |db| db.meals_between(from, to))
}

#[tauri::command]
pub fn save_meal(state: State<AppState>, meal: Meal) -> CmdResult<i64> {
    let name = meal.name.trim();
    if name.is_empty() {
        return Err("name required".into());
    }
    if !Meal::SLOTS.contains(&meal.slot.as_str()) || !(0..=health::MAX_KCAL).contains(&meal.kcal) {
        return Err("invalid meal".into());
    }
    with_db(&state, |db| db.save_meal(&Meal { name: name.into(), ..meal }))
}

#[tauri::command]
pub fn delete_meal(state: State<AppState>, id: i64) -> CmdResult<Option<Meal>> {
    with_db(&state, |db| db.delete_meal(id))
}

#[tauri::command]
pub fn workouts_between(state: State<AppState>, from: NaiveDate, to: NaiveDate) -> CmdResult<Vec<Workout>> {
    with_db(&state, |db| db.workouts_between(from, to))
}

#[tauri::command]
pub fn save_workout(state: State<AppState>, workout: Workout) -> CmdResult<i64> {
    if workout.kind.trim().is_empty()
        || !(1..=health::MAX_MINUTES).contains(&workout.minutes)
        || !(0..=health::MAX_KCAL).contains(&workout.kcal)
    {
        return Err("invalid workout".into());
    }
    with_db(&state, |db| db.save_workout(&Workout { note: workout.note.trim().into(), ..workout }))
}

#[tauri::command]
pub fn delete_workout(state: State<AppState>, id: i64) -> CmdResult<Option<Workout>> {
    with_db(&state, |db| db.delete_workout(id))
}

// ---- backup, restore and reset -----------------------------------------------------

/// Saves everything (database, D-Day images, settings) to a backup file at `path`.
#[tauri::command]
pub async fn export_data(state: State<'_, AppState>, path: String) -> CmdResult<Manifest> {
    let scratch = state.data_dir.join("export-snapshot.tmp");
    let counts = with_db(&state, |db| Ok(backup::prepare_export(db, &scratch)))??;
    let images_dir = state.images_dir.clone();
    tauri::async_runtime::spawn_blocking(move || backup::write_backup(&scratch, &images_dir, counts, std::path::Path::new(&path)))
        .await
        .map_err(|e| e.to_string())?
}

/// Reads a backup's summary so the user can confirm before importing.
#[tauri::command]
pub fn inspect_backup(path: String) -> CmdResult<Manifest> {
    backup::inspect(std::path::Path::new(&path))
}

/// Replaces all data with a backup, then restarts the app to load it.
#[tauri::command]
pub fn import_data(app: tauri::AppHandle, state: State<AppState>, path: String) -> CmdResult<()> {
    backup::stage_import(std::path::Path::new(&path), &state.data_dir)?;
    app.restart()
}

/// Deletes all data, then restarts the app with a fresh start.
#[tauri::command]
pub fn reset_all_data(app: tauri::AppHandle, state: State<AppState>) -> CmdResult<()> {
    backup::stage_reset(&state.data_dir)?;
    app.restart()
}
