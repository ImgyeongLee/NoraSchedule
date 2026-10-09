//! Overlay panel: a small borderless window (label `panel`) that shows a few Overview
//! widgets, stays out of the taskbar, can stay above other windows, and can start
//! by itself when the user signs in to Windows (`--panel`, see `main.rs`).
//!
//! It loads the same frontend as the main window; `src/main.ts` mounts `PanelApp`
//! when the window label is `panel`. Everything here uses Tauri's own window API.
//!
//! Settings (in the database): `panel.enabled` (open at start-up), `panel.onTop`,
//! `panel.collapsed` (folded up to its top bar), `panel.bounds` (last position and size,
//! physical pixels; the height is the unfolded one).

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;

use crate::AppState;

pub const LABEL: &str = "panel";
/// Command-line flag used by the sign-in entry: open only the panel, keep the main window hidden.
pub const START_ARG: &str = "--panel";

const DEFAULT_WIDTH: f64 = 340.0;
const DEFAULT_HEIGHT: f64 = 600.0;
const MARGIN: f64 = 16.0;
const MIN_WIDTH: f64 = 260.0;
const MIN_HEIGHT: f64 = 260.0;
/// Height when folded: the panel's top bar (40px) plus its 1px top and bottom border (PanelApp.svelte).
const COLLAPSED_HEIGHT: f64 = 42.0;

/// Window outer position and content size, in physical pixels.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Serialize)]
pub struct PanelStatus {
    pub open: bool,
    pub enabled: bool,
    pub on_top: bool,
    pub autostart: bool,
    pub collapsed: bool,
}

fn setting(app: &AppHandle, key: &str) -> Option<String> {
    let state = app.try_state::<AppState>()?;
    let db = state.db.lock().ok()?;
    db.setting(key).ok().flatten()
}

fn set_setting(app: &AppHandle, key: &str, value: &str) {
    if let Some(state) = app.try_state::<AppState>()
        && let Ok(db) = state.db.lock()
    {
        let _ = db.set_setting(key, value);
    }
}

pub fn enabled(app: &AppHandle) -> bool {
    setting(app, "panel.enabled").is_some_and(|v| v == "true")
}

fn on_top(app: &AppHandle) -> bool {
    setting(app, "panel.onTop").is_none_or(|v| v == "true")
}

fn collapsed(app: &AppHandle) -> bool {
    setting(app, "panel.collapsed").is_some_and(|v| v == "true")
}

fn saved_bounds(app: &AppHandle) -> Option<Bounds> {
    setting(app, "panel.bounds").and_then(|s| serde_json::from_str::<Bounds>(&s).ok())
}

pub fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

/// Opens the panel (or brings it forward if it is already open).
pub fn open(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(w) = window(app) {
        w.show()?;
        return Ok(w);
    }
    // Read before the window exists: creating it fires move/resize events.
    let saved = saved_bounds(app);
    // Created hidden, placed, then shown, so it never flashes in the wrong spot.
    let w = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Nora Schedule")
        .decorations(false)
        .skip_taskbar(true)
        .always_on_top(on_top(app))
        .resizable(true)
        .inner_size(DEFAULT_WIDTH, DEFAULT_HEIGHT)
        .min_inner_size(MIN_WIDTH, MIN_HEIGHT)
        .shadow(true)
        .focused(false)
        .visible(false)
        .build()?;
    // The stored rectangle is only data: use it only if it is a sane size on a real screen.
    let sane = |b: &Bounds| (200..=10_000).contains(&b.w) && (200..=10_000).contains(&b.h) && on_some_monitor(app, b);
    match saved.filter(sane) {
        Some(b) => {
            set_inner_size_checked(&w, PhysicalSize::new(b.w, b.h))?;
            w.set_position(Position::Physical(PhysicalPosition::new(b.x, b.y)))?;
        }
        None => place(&w, "top-right")?,
    }
    if collapsed(app) {
        fold(&w, true)?;
    }
    w.show()?;
    // Showing a window that was created hidden drops its topmost flag, so set it again.
    w.set_always_on_top(on_top(app))?;
    Ok(w)
}

/// Sets the content size and checks it. Right after a borderless window is created on
/// Windows, the first resize comes out one (undrawn) title bar taller than asked; later ones
/// are exact. Retrying until it matches keeps the panel from changing size on every start.
fn set_inner_size_checked(w: &WebviewWindow, target: PhysicalSize<u32>) -> tauri::Result<()> {
    for _ in 0..3 {
        w.set_size(Size::Physical(target))?;
        if w.inner_size()? == target {
            break;
        }
    }
    Ok(())
}

/// Folds the panel up to its top bar, or unfolds it to the height it had before.
/// Folding instead of minimizing: the panel has no taskbar button to bring it back from.
fn fold(w: &WebviewWindow, on: bool) -> tauri::Result<()> {
    let scale = w.scale_factor()?;
    let width = w.inner_size()?.width;
    if on {
        w.set_min_size(None::<Size>)?;
        w.set_resizable(false)?;
        set_inner_size_checked(w, PhysicalSize::new(width, (COLLAPSED_HEIGHT * scale).round() as u32))?;
    } else {
        let height = saved_bounds(w.app_handle())
            .map(|b| b.h)
            .filter(|h| (200..=10_000).contains(h))
            .unwrap_or((DEFAULT_HEIGHT * scale).round() as u32);
        w.set_resizable(true)?;
        set_inner_size_checked(w, PhysicalSize::new(width, height))?;
        w.set_min_size(Some(Size::Logical(tauri::LogicalSize::new(MIN_WIDTH, MIN_HEIGHT))))?;
    }
    Ok(())
}

/// Whether at least part of the saved rectangle is on a connected screen.
fn on_some_monitor(app: &AppHandle, b: &Bounds) -> bool {
    let Ok(monitors) = app.available_monitors() else { return false };
    monitors.iter().any(|m| {
        let (p, s) = (m.position(), m.size());
        // At least 40 px of the panel must be visible horizontally and vertically.
        b.x + b.w as i32 > p.x + 40
            && b.x < p.x + s.width as i32 - 40
            && b.y + 40 > p.y
            && b.y < p.y + s.height as i32 - 40
    })
}

/// Moves the panel into a corner of the screen it is on (inside the taskbar-free area).
pub fn place(w: &WebviewWindow, corner: &str) -> tauri::Result<()> {
    let Some(monitor) = w.current_monitor()?.or(w.primary_monitor()?) else { return Ok(()) };
    let area = monitor.work_area();
    let margin = (MARGIN * monitor.scale_factor()).round() as i32;
    let size = w.outer_size()?;
    let left = area.position.x + margin;
    let right = area.position.x + area.size.width as i32 - size.width as i32 - margin;
    let top = area.position.y + margin;
    let bottom = area.position.y + area.size.height as i32 - size.height as i32 - margin;
    let (x, y) = match corner {
        "top-left" => (left, top),
        "bottom-left" => (left, bottom),
        "bottom-right" => (right, bottom),
        _ => (right, top),
    };
    w.set_position(Position::Physical(PhysicalPosition::new(x, y)))?;
    save_bounds(w);
    Ok(())
}

/// Remembers where the panel is, so the next start puts it back there.
pub fn save_bounds(w: &WebviewWindow) {
    // Not while it is still being set up (hidden): those positions are not the user's.
    if !w.is_visible().unwrap_or(false) {
        return;
    }
    let (Ok(p), Ok(s)) = (w.outer_position(), w.inner_size()) else { return };
    // Minimizing reports a far-off position; don't store that.
    if p.x <= -30000 || p.y <= -30000 || s.width == 0 {
        return;
    }
    // While folded, keep the unfolded height so unfolding (and the next start) restores it.
    let h = match saved_bounds(w.app_handle()) {
        Some(old) if collapsed(w.app_handle()) => old.h,
        _ => s.height,
    };
    let b = Bounds { x: p.x, y: p.y, w: s.width, h };
    if let Ok(json) = serde_json::to_string(&b) {
        set_setting(w.app_handle(), "panel.bounds", &json);
    }
}

/// Window events for both windows (registered on the app builder in `main.rs`).
pub fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    let app = window.app_handle();
    match (window.label(), event) {
        (LABEL, WindowEvent::Moved(_) | WindowEvent::Resized(_)) => {
            if let Some(w) = self::window(app) {
                save_bounds(&w);
            }
        }
        // The panel is turned off only from the app (Settings → Overlay panel), never by
        // closing it (e.g. Alt+F4): that could leave the app running with nothing on screen.
        // Settings removes it with `destroy`, which does not ask.
        (LABEL, WindowEvent::CloseRequested { api, .. }) => api.prevent_close(),
        // While the panel is open, closing the main window only hides it, so the panel
        // keeps working and "Open Nora" can bring the same window back.
        ("main", WindowEvent::CloseRequested { api, .. }) if self::window(app).is_some() => {
            api.prevent_close();
            let _ = window.hide();
        }
        _ => {}
    }
}

/// Shows the main window, optionally on a page (and a day for the calendar, or a memo).
/// The main window checks the page name before using it.
pub fn show_main(app: &AppHandle, page: Option<String>, day: Option<String>, memo: Option<i64>) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
        if page.is_some() {
            let _ = app.emit_to("main", "nora://navigate", serde_json::json!({ "page": page, "day": day, "memo": memo }));
        }
    }
}

/// Handles a second launch of the app (single instance): `--panel` opens the panel,
/// anything else brings the main window forward.
pub fn on_second_instance(app: &AppHandle, args: Vec<String>) {
    if args.iter().any(|a| a == START_ARG) {
        let _ = open(app);
    } else {
        show_main(app, None, None, None);
    }
}

// ---- commands -------------------------------------------------------------

#[tauri::command]
pub fn panel_status(app: AppHandle) -> PanelStatus {
    PanelStatus {
        open: window(&app).is_some(),
        enabled: enabled(&app),
        on_top: on_top(&app),
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        collapsed: collapsed(&app),
    }
}

/// Turns the panel on or off (and remembers it for the next start).
/// Async: creating a window from a synchronous command can deadlock on Windows.
#[tauri::command]
pub async fn panel_set_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    set_setting(&app, "panel.enabled", &enabled.to_string());
    if enabled {
        open(&app).map_err(|e| e.to_string())?;
    } else if let Some(w) = window(&app) {
        save_bounds(&w);
        w.destroy().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn panel_set_on_top(app: AppHandle, on: bool) -> Result<(), String> {
    set_setting(&app, "panel.onTop", &on.to_string());
    if let Some(w) = window(&app) {
        w.set_always_on_top(on).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Folds the panel up to its top bar (`true`) or unfolds it, and remembers the choice.
#[tauri::command]
pub fn panel_set_collapsed(app: AppHandle, collapsed: bool) -> Result<(), String> {
    let Some(w) = window(&app) else { return Ok(()) };
    if collapsed {
        save_bounds(&w); // remember the unfolded height first
        set_setting(&app, "panel.collapsed", "true");
    } else {
        set_setting(&app, "panel.collapsed", "false");
    }
    fold(&w, collapsed).map_err(|e| e.to_string())
}

/// `corner`: `top-left`, `top-right`, `bottom-left` or `bottom-right`.
#[tauri::command]
pub fn panel_place(app: AppHandle, corner: String) -> Result<(), String> {
    match window(&app) {
        Some(w) => place(&w, &corner).map_err(|e| e.to_string()),
        None => Ok(()),
    }
}

#[tauri::command]
pub fn panel_show_main(app: AppHandle, page: Option<String>, day: Option<String>, memo: Option<i64>) {
    show_main(&app, page, day, memo);
}

/// Starts the app (as the panel) when the user signs in to Windows.
#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let launcher = app.autolaunch();
    if enabled { launcher.enable() } else { launcher.disable() }.map_err(|e| e.to_string())?;
    launcher.is_enabled().map_err(|e| e.to_string())
}
