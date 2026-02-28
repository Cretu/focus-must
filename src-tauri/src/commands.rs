use crate::app_monitor;
use crate::state::{
    lock_mutex, normalize_locale, unix_now_secs, AppInfo, AppState, MIN_SESSION_DURATION_SECS,
};
use crate::storage;
use crate::tray::{tray_locale, tray_title_break_minutes, TrayMenuState};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Show the main window (optionally always-on-top) and focus it.
/// Also shows all overlay windows on secondary monitors.
pub fn show_main_window(app: &tauri::AppHandle, always_on_top: bool) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_always_on_top(always_on_top);
        let _ = win.show();
        let _ = win.set_focus();
    }
    show_all_overlays(app, always_on_top);
}

/// Hide the main window and all overlay windows.
pub fn hide_all_windows(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    hide_all_overlays(app);
}

/// Show all overlay windows on secondary monitors.
fn show_all_overlays(app: &tauri::AppHandle, always_on_top: bool) {
    for (label, win) in app.webview_windows() {
        if label.starts_with("overlay-") {
            let _ = win.set_always_on_top(always_on_top);
            let _ = win.show();
        }
    }
}

/// Hide all overlay windows.
fn hide_all_overlays(app: &tauri::AppHandle) {
    for (label, win) in app.webview_windows() {
        if label.starts_with("overlay-") {
            let _ = win.hide();
        }
    }
}

/// Shared lock-session logic used by both the command and the tray handler.
pub fn do_lock_session(app: &tauri::AppHandle) {
    {
        let state = app.state::<Mutex<AppState>>();
        let mut s = lock_mutex(&state);

        // Log session if it was a focus session
        if let Some(start) = s.focus_started_at {
            let now = unix_now_secs();
            let duration = now.saturating_sub(start);

            if duration >= MIN_SESSION_DURATION_SECS {
                storage::append_session(&storage::SessionRecord {
                    session_type: "focus".to_string(),
                    started_at: start,
                    ended_at: now,
                    duration_secs: duration,
                    task: s.task_description.clone(),
                    whitelist: s.session_whitelist.clone(),
                });
            }
        }

        s.session_whitelist.clear();
        s.task_description = None;
        s.is_restricted = true;
        s.focus_started_at = None;
        let _ = app.emit("state-changed", s.clone());
    }
    {
        let ts = app.state::<Mutex<TrayMenuState>>();
        if let Ok(ts) = ts.lock() {
            ts.set_focus_inactive();
        };
    }
    show_main_window(app, false);
}

pub fn log_break_session(s: &mut AppState) {
    if let (Some(start), Some(_)) = (s.free_activity_started_at, s.free_activity_end_at) {
        let now = unix_now_secs();
        let duration = now.saturating_sub(start);

        if duration >= MIN_SESSION_DURATION_SECS {
            storage::append_session(&storage::SessionRecord {
                session_type: "break".to_string(),
                started_at: start,
                ended_at: now,
                duration_secs: duration,
                task: None,
                whitelist: vec![],
            });
        }
    }
    s.free_activity_started_at = None;
    s.free_activity_end_at = None;
}

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_running_apps(include_icons: Option<bool>) -> Vec<AppInfo> {
    app_monitor::get_running_apps(include_icons.unwrap_or(false))
}

#[tauri::command]
pub fn get_app_icon(bundle_id: String) -> Option<String> {
    app_monitor::get_app_icon(&bundle_id)
}

#[tauri::command]
pub fn get_app_info(bundle_id: String, include_icon: Option<bool>) -> Option<AppInfo> {
    app_monitor::get_app_info(&bundle_id, include_icon.unwrap_or(false))
}

#[tauri::command]
pub fn get_state(state: tauri::State<'_, Mutex<AppState>>) -> AppState {
    lock_mutex(&state).clone()
}

/// Start focus session — hide window, enable tray "End Focus"
#[tauri::command]
pub fn unlock_session(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    tray_state: tauri::State<'_, Mutex<TrayMenuState>>,
    whitelist: Vec<String>,
    task: String,
) {
    {
        let mut s = lock_mutex(&state);
        // Log previous break if exists
        log_break_session(&mut s);

        s.session_whitelist = whitelist;
        s.task_description = Some(task);
        s.focus_started_at = Some(unix_now_secs());
        s.free_activity_end_at = None;
        let _ = app.emit("state-changed", s.clone());
    }

    if let Ok(ts) = tray_state.lock() {
        ts.set_focus_active();
    }

    hide_all_windows(&app);
}

/// End focus — clear whitelist, show blocking window
#[tauri::command]
pub fn lock_session(app: tauri::AppHandle) {
    do_lock_session(&app);
}

#[tauri::command]
pub fn update_settings(
    state: tauri::State<'_, Mutex<AppState>>,
    default_whitelist: Option<Vec<String>>,
) {
    let mut s = lock_mutex(&state);
    if let Some(wl) = default_whitelist {
        s.default_whitelist = wl.clone();
        storage::save_settings(&storage::UserSettings {
            default_whitelist: wl,
            locale: s.locale.clone(),
        });
    }
}

#[tauri::command]
pub fn set_locale(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    tray_state: tauri::State<'_, Mutex<TrayMenuState>>,
    locale: String,
) {
    let normalized = normalize_locale(&locale).to_string();

    let (has_focus_session, has_break_session, default_whitelist) = {
        let mut s = lock_mutex(&state);
        s.locale = normalized.clone();
        let _ = app.emit("state-changed", s.clone());
        (
            s.focus_started_at.is_some(),
            s.free_activity_end_at.is_some(),
            s.default_whitelist.clone(),
        )
    };

    storage::save_settings(&storage::UserSettings {
        default_whitelist,
        locale: normalized.clone(),
    });

    if let Ok(mut ts) = tray_state.lock() {
        ts.set_locale(normalized.clone());
        if has_break_session {
            ts.set_break_active();
        } else if has_focus_session {
            ts.set_focus_active();
        } else {
            ts.set_focus_inactive();
            ts.set_break_inactive();
        }
    }

    if let Some(tray) = app.tray_by_id("focus-tray") {
        let _ = tray.set_title(Some(tray_locale(&normalized).planning));
    }
}

#[tauri::command]
pub fn switch_to_app(bundle_id: String) {
    let _ = std::process::Command::new("open")
        .args(["-b", &bundle_id])
        .spawn();
}

#[tauri::command]
pub fn get_history() -> Vec<storage::SessionRecord> {
    storage::load_sessions()
}

#[tauri::command]
pub fn get_history_page(offset: Option<u64>, limit: Option<u64>) -> storage::HistoryPage {
    storage::load_sessions_page(offset.unwrap_or(0), limit.unwrap_or(100))
}

#[tauri::command]
pub fn get_analytics() -> storage::AnalyticsData {
    storage::load_analytics()
}

/// Start a free-activity (break) session
#[tauri::command]
pub fn start_free_activity(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    tray_state: tauri::State<'_, Mutex<TrayMenuState>>,
    duration_minutes: u64,
) {
    {
        let mut s = lock_mutex(&state);
        let now = unix_now_secs();
        s.free_activity_started_at = Some(now);
        s.free_activity_end_at = Some(now + duration_minutes * 60);
        s.focus_started_at = None;
        let _ = app.emit("state-changed", s.clone());
    }

    if let Ok(ts) = tray_state.lock() {
        ts.set_break_active();
    }

    hide_all_windows(&app);

    if let Some(tray) = app.tray_by_id("focus-tray") {
        let locale = {
            let s = lock_mutex(&state);
            s.locale.clone()
        };
        let _ = tray.set_title(Some(&tray_title_break_minutes(&locale, duration_minutes)));
    }
}
