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

/// Hide the main window, the distraction prompt, and all overlay windows.
/// Also clears the shared `prompt_active` flag.
pub fn hide_all_windows(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    if let Some(win) = app.get_webview_window("prompt") {
        let _ = win.hide();
    }
    hide_all_overlays(app);

    if let Some(state) = app.try_state::<Mutex<AppState>>() {
        lock_mutex(&state).prompt_active = false;
    }
}

/// Show the distraction prompt centered on the monitor under the cursor
/// (falling back to the primary monitor). Small modal window — not a cover.
/// Call on the main thread.
pub fn show_prompt_window(app: &tauri::AppHandle) {
    let Some(prompt) = app.get_webview_window("prompt") else {
        return;
    };

    let target = prompt
        .cursor_position()
        .ok()
        .and_then(|p| prompt.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| prompt.primary_monitor().ok().flatten());

    if let Some(mon) = target {
        let mpos = mon.position();
        let msize = mon.size();
        if let Ok(wsize) = prompt.outer_size() {
            let x = mpos.x + (msize.width as i32 - wsize.width as i32) / 2;
            let y = mpos.y + (msize.height as i32 - wsize.height as i32) / 2;
            let _ = prompt.set_position(tauri::PhysicalPosition::new(x, y));
        }
    }

    let _ = prompt.set_always_on_top(true);
    let _ = prompt.show();
    let _ = prompt.set_focus();
}

/// Play a built-in macOS system sound by name (e.g. "Submarine", "Glass").
/// Non-blocking; failures are ignored (e.g. on non-macOS).
pub fn play_sound(name: &str) {
    let _ = std::process::Command::new("afplay")
        .arg(format!("/System/Library/Sounds/{name}.aiff"))
        .spawn();
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

/// Create the small distraction-prompt window (hidden until needed). macOS only.
#[cfg(target_os = "macos")]
pub fn create_prompt_window(app: &tauri::AppHandle) {
    use tauri::{WebviewUrl, WebviewWindowBuilder};

    if app.get_webview_window("prompt").is_some() {
        return;
    }

    let url = WebviewUrl::App("index.html".into());
    let win = match WebviewWindowBuilder::new(app, "prompt", url)
        .title("")
        .inner_size(460.0, 400.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .resizable(false)
        .closable(false)
        .skip_taskbar(true)
        .visible(false)
        .center()
        .build()
    {
        Ok(win) => win,
        Err(error) => {
            eprintln!("Failed to create prompt window: {error}");
            return;
        }
    };

    // Visible across all Spaces, like the other windows.
    {
        use objc2::msg_send;
        if let Ok(ns_window) = win.ns_window() {
            let ns_win: *mut objc2::runtime::AnyObject = ns_window.cast();
            unsafe {
                let behavior: isize = (1 << 0) | (1 << 4);
                let _: () = msg_send![&*ns_win, setCollectionBehavior: behavior];
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
    focus_goal_minutes: Option<u64>,
) {
    {
        let mut s = lock_mutex(&state);
        // Log previous break if exists
        log_break_session(&mut s);

        s.session_whitelist = whitelist;
        s.task_description = Some(task);
        s.focus_started_at = Some(unix_now_secs());
        s.free_activity_end_at = None;
        s.focus_goal_minutes = focus_goal_minutes.unwrap_or(0);
        // Start each focus session with a clean slate of temporary passes.
        s.temp_allowed.clear();
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

    let (next_state, has_focus_session, has_break_session, default_whitelist) = {
        let mut s = lock_mutex(&state);
        s.locale = normalized.clone();
        (
            s.clone(),
            s.focus_started_at.is_some(),
            s.free_activity_end_at.is_some(),
            s.default_whitelist.clone(),
        )
    };

    let _ = app.emit("state-changed", next_state);

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
pub fn switch_to_app(bundle_id: String) -> Result<(), String> {
    std::process::Command::new("open")
        .args(["-b", &bundle_id])
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Failed to switch to app {bundle_id}: {error}"))
}

/// Dismiss the distraction prompt and keep focusing (the app stays collected).
#[tauri::command]
pub fn dismiss_distraction(app: tauri::AppHandle) {
    hide_all_windows(&app);
}

/// Grant a distracting app a temporary pass: allow it for `duration_minutes`,
/// bring it back to the foreground, and dismiss the distraction prompt. Once the
/// grace period expires the app is collected again on the next check.
#[tauri::command]
pub fn allow_app_temporarily(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    bundle_id: String,
    duration_minutes: u64,
) {
    {
        let mut s = lock_mutex(&state);
        let until = unix_now_secs() + duration_minutes.max(1) * 60;
        s.temp_allowed.insert(bundle_id.clone(), until);
        let _ = app.emit("state-changed", s.clone());
    }

    // Bring the app back so the user can use it during the grace period.
    let _ = std::process::Command::new("open")
        .args(["-b", &bundle_id])
        .spawn();

    hide_all_windows(&app);
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
