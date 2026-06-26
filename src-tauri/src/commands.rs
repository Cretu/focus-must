use crate::app_monitor;
use crate::state::{
    lock_mutex, normalize_locale, unix_now_secs, AppInfo, AppState, MonitorInfo,
    MIN_SESSION_DURATION_SECS,
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

/// Send a native notification that a distracting app was minimized. The app is
/// already collected, so this is the only cue (no intrusive window).
pub fn notify_distraction(app: &tauri::AppHandle, app_name: &str, locale: &str) {
    use tauri_plugin_notification::NotificationExt;

    let body = if crate::state::locale_is_en(locale) {
        format!("Minimized {app_name} — stay focused")
    } else {
        format!("已收起 {app_name}，继续专注")
    };

    let _ = app
        .notification()
        .builder()
        .title("Focus Must")
        .body(body)
        .show();
}

/// Play a built-in macOS system sound by name (e.g. "Submarine", "Glass").
/// Non-blocking; failures are ignored (e.g. on non-macOS).
pub fn play_sound(name: &str) {
    let _ = std::process::Command::new("afplay")
        .arg(format!("/System/Library/Sounds/{name}.aiff"))
        .spawn();
}

/// Collect the connected displays for diagnostics. Call on the main thread.
pub fn collect_monitors_info(app: &tauri::AppHandle) -> Vec<MonitorInfo> {
    let Some(win) = app.get_webview_window("main") else {
        return Vec::new();
    };

    let primary_pos = win.primary_monitor().ok().flatten().map(|m| {
        let p = m.position();
        (p.x, p.y)
    });

    win.available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|m| {
            let p = m.position();
            let s = m.size();
            MonitorInfo {
                name: m.name().cloned().unwrap_or_default(),
                width: s.width,
                height: s.height,
                x: p.x,
                y: p.y,
                is_primary: Some((p.x, p.y)) == primary_pos,
                scale: m.scale_factor(),
            }
        })
        .collect()
}

/// Refresh the cached display list in shared state. Call on the main thread.
pub fn refresh_monitors_info(app: &tauri::AppHandle) {
    let monitors = collect_monitors_info(app);
    if let Some(state) = app.try_state::<Mutex<AppState>>() {
        lock_mutex(&state).monitors_info = monitors;
    }
}

/// Diagnostics snapshot for the self-check panel.
#[derive(serde::Serialize)]
pub struct SelfCheckReport {
    /// Last non-self frontmost app the monitor saw (None if not seen yet).
    pub last_frontmost: Option<String>,
    /// Connected displays.
    pub monitors: Vec<MonitorInfo>,
    /// App version.
    pub version: String,
}

/// Run the self-check: report monitoring liveness, displays, and version.
/// Reads the display list cached on the main thread (startup + hot-plug refresh)
/// so the command stays safe off the main thread.
#[tauri::command]
pub fn run_self_check(state: tauri::State<'_, Mutex<AppState>>) -> SelfCheckReport {
    let s = lock_mutex(&state);
    SelfCheckReport {
        last_frontmost: s.last_frontmost.clone(),
        monitors: s.monitors_info.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Play the reminder sound so the user can confirm audio works.
#[tauri::command]
pub fn test_sound() {
    play_sound("Glass");
}

/// Send a sample notification so the user can confirm notifications are allowed.
#[tauri::command]
pub fn test_notification(app: tauri::AppHandle, state: tauri::State<'_, Mutex<AppState>>) {
    let locale = lock_mutex(&state).locale.clone();
    notify_distraction(&app, "Self-check", &locale);
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

/// Size the main window to the primary monitor and (re)create an overlay window
/// covering every other monitor, so distraction blocking can be enforced on all
/// displays. The primary monitor is identified by position — names can be empty
/// or duplicated on some setups, which previously caused secondary monitors to
/// be skipped. Existing overlays are torn down first, so this also handles
/// monitors being connected/disconnected after launch.
///
/// Must be called on the main thread (it creates windows). macOS only.
#[cfg(target_os = "macos")]
pub fn sync_overlays(app: &tauri::AppHandle) {
    use std::sync::atomic::{AtomicU32, Ordering};
    use tauri::{PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

    // Monotonic suffix so a rebuilt overlay never reuses a still-closing label.
    static OVERLAY_SEQ: AtomicU32 = AtomicU32::new(0);

    let Some(main) = app.get_webview_window("main") else {
        return;
    };

    let monitors = main.available_monitors().unwrap_or_default();
    let primary = main.primary_monitor().ok().flatten();
    let primary_pos = primary.as_ref().map(|m| {
        let p = m.position();
        (p.x, p.y)
    });

    // Keep the main window matched to the primary monitor.
    if let Some(ref p) = primary {
        let size = p.size();
        let pos = p.position();
        let _ = main.set_size(PhysicalSize::new(size.width, size.height));
        let _ = main.set_position(PhysicalPosition::new(pos.x, pos.y));
    }

    // Tear down existing overlays before rebuilding for the current layout.
    for (label, win) in app.webview_windows() {
        if label.starts_with("overlay-") {
            let _ = win.destroy();
        }
    }

    let frontend_url = WebviewUrl::App("index.html".into());

    for monitor in monitors.iter() {
        let pos = monitor.position();
        // Skip the primary monitor (already handled by the main window).
        if Some((pos.x, pos.y)) == primary_pos {
            continue;
        }

        let label = format!("overlay-{}", OVERLAY_SEQ.fetch_add(1, Ordering::Relaxed));
        let size = monitor.size();

        let overlay = match WebviewWindowBuilder::new(app, &label, frontend_url.clone())
            .title("")
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .resizable(false)
            .closable(false)
            .skip_taskbar(true)
            .visible(false)
            .build()
        {
            Ok(win) => win,
            Err(error) => {
                eprintln!("Failed to create overlay window {label}: {error}");
                continue;
            }
        };

        let _ = overlay.set_size(PhysicalSize::new(size.width, size.height));
        let _ = overlay.set_position(PhysicalPosition::new(pos.x, pos.y));

        // Visible across all Spaces, like the main window.
        {
            use objc2::msg_send;
            if let Ok(ns_window) = overlay.ns_window() {
                let ns_win: *mut objc2::runtime::AnyObject = ns_window.cast();
                unsafe {
                    let behavior: isize = (1 << 0) | (1 << 4);
                    let _: () = msg_send![&*ns_win, setCollectionBehavior: behavior];
                }
            }
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
        s.paused = false;
        s.paused_at = None;
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
        s.paused = false;
        s.paused_at = None;
        let _ = app.emit("state-changed", s.clone());
    }

    if let Ok(ts) = tray_state.lock() {
        ts.set_focus_active();
    }

    hide_all_windows(&app);
}

/// Pause the focus session: stop blocking and surface the planner so the user
/// can adjust the whitelist before resuming. The elapsed timer is frozen.
#[tauri::command]
pub fn pause_focus(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    tray_state: tauri::State<'_, Mutex<TrayMenuState>>,
) {
    {
        let mut s = lock_mutex(&state);
        if s.focus_started_at.is_none() || s.paused {
            return;
        }
        s.paused = true;
        s.paused_at = Some(unix_now_secs());
        let _ = app.emit("state-changed", s.clone());
    }
    if let Ok(ts) = tray_state.lock() {
        ts.set_paused();
    }
    show_main_window(&app, false);
}

/// Resume a paused focus session with the (possibly edited) whitelist, excluding
/// the paused time from the elapsed duration.
#[tauri::command]
pub fn resume_focus(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    tray_state: tauri::State<'_, Mutex<TrayMenuState>>,
    whitelist: Option<Vec<String>>,
) {
    {
        let mut s = lock_mutex(&state);
        if !s.paused {
            return;
        }
        if let Some(wl) = whitelist {
            s.session_whitelist = wl;
        }
        // Shift the start forward by the paused duration so elapsed excludes it.
        if let (Some(start), Some(paused_at)) = (s.focus_started_at, s.paused_at) {
            let paused_for = unix_now_secs().saturating_sub(paused_at);
            s.focus_started_at = Some(start + paused_for);
        }
        s.paused = false;
        s.paused_at = None;
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

/// Hide the main window and overlays without touching session state.
/// Used by "Continue Focus" when the user manually peeks at the app
/// during an active session.
#[tauri::command]
pub fn hide_windows(app: tauri::AppHandle) {
    hide_all_windows(&app);
}

/// Persist the user-facing settings from the current app state.
fn persist_settings(s: &AppState) {
    storage::save_settings(&storage::UserSettings {
        default_whitelist: s.default_whitelist.clone(),
        locale: s.locale.clone(),
        break_reminder_minutes: s.focus_goal_minutes,
    });
}

#[tauri::command]
pub fn update_settings(
    state: tauri::State<'_, Mutex<AppState>>,
    default_whitelist: Option<Vec<String>>,
) {
    let mut s = lock_mutex(&state);
    if let Some(wl) = default_whitelist {
        s.default_whitelist = wl;
        persist_settings(&s);
    }
}

/// Set the break-reminder interval (minutes; 0 = disabled) and persist it.
#[tauri::command]
pub fn set_break_reminder(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    minutes: u64,
) {
    let next = {
        let mut s = lock_mutex(&state);
        s.focus_goal_minutes = minutes;
        s.clone()
    };
    let _ = app.emit("state-changed", next.clone());
    persist_settings(&next);
}

#[tauri::command]
pub fn set_locale(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    tray_state: tauri::State<'_, Mutex<TrayMenuState>>,
    locale: String,
) {
    let normalized = normalize_locale(&locale).to_string();

    let (next_state, has_focus_session, has_break_session) = {
        let mut s = lock_mutex(&state);
        s.locale = normalized.clone();
        (
            s.clone(),
            s.focus_started_at.is_some(),
            s.free_activity_end_at.is_some(),
        )
    };

    persist_settings(&next_state);
    let _ = app.emit("state-changed", next_state);

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
pub fn switch_to_app(bundle_id: String) -> Result<(), String> {
    std::process::Command::new("open")
        .args(["-b", &bundle_id])
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Failed to switch to app {bundle_id}: {error}"))
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
