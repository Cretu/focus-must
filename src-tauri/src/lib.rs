use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

mod app_monitor;
mod storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub bundle_id: String,
    pub icon_data_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub is_restricted: bool,
    pub default_whitelist: Vec<String>,
    pub session_whitelist: Vec<String>,
    pub task_description: Option<String>,
    pub focus_started_at: Option<u64>,
    pub free_activity_started_at: Option<u64>,
    pub free_activity_end_at: Option<u64>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            is_restricted: true,
            default_whitelist: vec![
                "com.apple.finder".to_string(),
                "com.apple.systempreferences".to_string(),
                "com.focus-must.app".to_string(),
            ],
            session_whitelist: vec![],
            task_description: None,
            focus_started_at: None,
            free_activity_started_at: None,
            free_activity_end_at: None,
        }
    }
}

impl AppState {
    pub fn is_app_allowed(&self, bundle_id: &str) -> bool {
        if self.free_activity_end_at.is_some() {
            return true;
        }
        if !self.is_restricted {
            return true;
        }
        self.default_whitelist.iter().any(|id| id == bundle_id)
            || self.session_whitelist.iter().any(|id| id == bundle_id)
    }
}

// ---------------------------------------------------------------------------
// Tray Menu State — helpers to reduce repetition
// ---------------------------------------------------------------------------
struct TrayMenuState {
    lock_item: Option<MenuItem<tauri::Wry>>,
    end_break_item: Option<MenuItem<tauri::Wry>>,
}

impl TrayMenuState {
    fn set_focus_active(&self) {
        if let Some(item) = &self.lock_item {
            let _ = item.set_enabled(true);
            let _ = item.set_text("🔒 结束专注");
        }
        if let Some(item) = &self.end_break_item {
            let _ = item.set_enabled(false);
            let _ = item.set_text("☕️ 结束休息 (未开始)");
        }
    }

    fn set_focus_inactive(&self) {
        if let Some(item) = &self.lock_item {
            let _ = item.set_enabled(false);
            let _ = item.set_text("🔒 结束专注 (未开始)");
        }
    }

    fn set_break_active(&self) {
        if let Some(item) = &self.end_break_item {
            let _ = item.set_enabled(true);
            let _ = item.set_text("☕️ 结束休息");
        }
        if let Some(item) = &self.lock_item {
            let _ = item.set_enabled(false);
            let _ = item.set_text("🔒 结束专注 (未开始)");
        }
    }

    fn set_break_inactive(&self) {
        if let Some(item) = &self.end_break_item {
            let _ = item.set_enabled(false);
            let _ = item.set_text("☕️ 结束休息 (未开始)");
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Show the main window (optionally always-on-top) and focus it.
fn show_main_window(app: &tauri::AppHandle, always_on_top: bool) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_always_on_top(always_on_top);
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// Shared lock-session logic used by both the command and the tray handler.
fn do_lock_session(app: &tauri::AppHandle) {
    {
        let state = app.state::<Mutex<AppState>>();
        let mut s = state.lock().unwrap();

        // Log session if it was a focus session
        if let Some(start) = s.focus_started_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let duration = now.saturating_sub(start);

            // Only log significant sessions (> 10s for testing)
            if duration >= 10 {
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

fn log_break_session(s: &mut AppState) {
    if let (Some(start), Some(_)) = (s.free_activity_started_at, s.free_activity_end_at) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let duration = now.saturating_sub(start);

        if duration >= 10 {
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
fn get_running_apps() -> Vec<AppInfo> {
    app_monitor::get_running_apps()
}

#[tauri::command]
fn get_state(state: tauri::State<'_, Mutex<AppState>>) -> AppState {
    state.lock().unwrap().clone()
}

/// Start focus session — hide window, enable tray "End Focus"
#[tauri::command]
fn unlock_session(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    tray_state: tauri::State<'_, Mutex<TrayMenuState>>,
    whitelist: Vec<String>,
    task: String,
) {
    {
        let mut s = state.lock().unwrap();
        // Log previous break if exists
        log_break_session(&mut s);

        s.session_whitelist = whitelist;
        s.task_description = Some(task);
        s.focus_started_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
        s.free_activity_end_at = None;
        let _ = app.emit("state-changed", s.clone());
    }

    if let Ok(ts) = tray_state.lock() {
        ts.set_focus_active();
    }

    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

/// End focus — clear whitelist, show blocking window
#[tauri::command]
fn lock_session(app: tauri::AppHandle) {
    do_lock_session(&app);
}

#[tauri::command]
fn update_settings(
    state: tauri::State<'_, Mutex<AppState>>,
    default_whitelist: Option<Vec<String>>,
) {
    let mut s = state.lock().unwrap();
    if let Some(wl) = default_whitelist {
        s.default_whitelist = wl.clone();
        storage::save_settings(&storage::UserSettings {
            default_whitelist: wl,
        });
    }
}

#[tauri::command]
fn switch_to_app(bundle_id: String) {
    let _ = std::process::Command::new("open")
        .args(["-b", &bundle_id])
        .spawn();
}

#[tauri::command]
fn get_history() -> Vec<storage::SessionRecord> {
    storage::load_sessions()
}

/// Start a free-activity (break) session
#[tauri::command]
fn start_free_activity(
    app: tauri::AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    tray_state: tauri::State<'_, Mutex<TrayMenuState>>,
    duration_minutes: u64,
) {
    {
        let mut s = state.lock().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        s.free_activity_started_at = Some(now);
        s.free_activity_end_at = Some(now + duration_minutes * 60);
        s.focus_started_at = None;
        let _ = app.emit("state-changed", s.clone());
    } // s dropped

    if let Ok(ts) = tray_state.lock() {
        ts.set_break_active();
    }

    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }

    if let Some(tray) = app.tray_by_id("focus-tray") {
        let _ = tray.set_title(Some(&format!("☕️ 休息中 {:02}:00", duration_minutes)));
    }
}

// ---------------------------------------------------------------------------
// Application entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new({
            let mut state = AppState::default();
            // Load persistent settings
            let settings = storage::load_settings();
            if !settings.default_whitelist.is_empty() {
                state.default_whitelist = settings.default_whitelist;
            }
            state
        }))
        .manage(Mutex::new(TrayMenuState {
            lock_item: None,
            end_break_item: None,
        }))
        .invoke_handler(tauri::generate_handler![
            get_running_apps,
            get_state,
            unlock_session,
            lock_session,
            switch_to_app,
            start_free_activity,
            update_settings,
            get_history,
        ])
        .setup(|app| {
            // --- Hide from Dock at runtime ---
            #[cfg(target_os = "macos")]
            {
                use objc2::MainThreadMarker;
                use objc2_app_kit::NSApplication;
                use objc2_app_kit::NSApplicationActivationPolicy;
                // Safety: setup runs on the main thread
                let mtm = unsafe { MainThreadMarker::new_unchecked() };
                let ns_app = NSApplication::sharedApplication(mtm);
                ns_app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
            }

            // --- System Tray ---
            let show_i = MenuItem::with_id(app, "show", "📋 显示计划窗口", true, None::<&str>)?;
            let lock_i =
                MenuItem::with_id(app, "lock", "🔒 结束专注 (未开始)", false, None::<&str>)?;
            let end_break_i = MenuItem::with_id(
                app,
                "end_break",
                "☕️ 结束休息 (未开始)",
                false,
                None::<&str>,
            )?;
            let settings_i = MenuItem::with_id(app, "settings", "⚙️ 设置", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

            // Store clones for dynamic updates
            {
                let ts = app.state::<Mutex<TrayMenuState>>();
                if let Ok(mut s) = ts.lock() {
                    s.lock_item = Some(lock_i.clone());
                    s.end_break_item = Some(end_break_i.clone());
                };
            }

            let menu =
                Menu::with_items(app, &[&show_i, &lock_i, &end_break_i, &settings_i, &quit_i])?;

            let _tray = TrayIconBuilder::with_id("focus-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(true)
                .menu(&menu)
                .title("🔒 计划中...")
                .tooltip("Focus Must")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        let _ = app.emit("show-view", "planning");
                        show_main_window(app, false);
                    }
                    "settings" => {
                        let _ = app.emit("show-view", "settings");
                        show_main_window(app, false);
                    }
                    "lock" => {
                        do_lock_session(app);
                    }
                    "end_break" => {
                        {
                            let state = app.state::<Mutex<AppState>>();
                            let mut s = state.lock().unwrap();
                            log_break_session(&mut s);
                            let _ = app.emit("state-changed", s.clone());
                        }
                        {
                            let ts = app.state::<Mutex<TrayMenuState>>();
                            if let Ok(ts) = ts.lock() {
                                ts.set_break_inactive();
                            };
                        }
                        show_main_window(app, false);
                        if let Some(tray) = app.tray_by_id("focus-tray") {
                            let _ = tray.set_title(Some("🔒 计划中..."));
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle(), false);
                    }
                })
                .build(app)?;

            // --- Setup blocking window ---
            if let Some(win) = app.get_webview_window("main") {
                if let Some(monitor) = win.primary_monitor().ok().flatten() {
                    let size = monitor.size();
                    let pos = monitor.position();
                    let _ = win.set_size(tauri::PhysicalSize::new(size.width, size.height));
                    let _ = win.set_position(tauri::PhysicalPosition::new(pos.x, pos.y));
                }

                // Visible on all macOS Spaces
                #[cfg(target_os = "macos")]
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

            show_main_window(app.handle(), false);

            // --- Background threads ---
            let monitor_handle = app.handle().clone();
            std::thread::spawn(move || app_monitor::start_monitoring(monitor_handle));

            let timer_handle = app.handle().clone();
            std::thread::spawn(move || tray_title_updater(timer_handle));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ---------------------------------------------------------------------------
// Tray title updater — runs every second on a background thread
// ---------------------------------------------------------------------------
fn tray_title_updater(app: tauri::AppHandle) {
    use std::thread;
    use std::time::Duration;

    loop {
        thread::sleep(Duration::from_secs(1));

        let state = app.state::<Mutex<AppState>>();
        let (started_at, free_end_at) = {
            let s = state.lock().unwrap();
            (s.focus_started_at, s.free_activity_end_at)
        };

        let title = if let Some(start_ts) = started_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let elapsed = now.saturating_sub(start_ts);

            let hours = elapsed / 3600;
            let mins = (elapsed % 3600) / 60;
            let secs = elapsed % 60;

            if hours > 0 {
                format!("🎯 专注中 {:02}:{:02}:{:02}", hours, mins, secs)
            } else {
                format!("🎯 专注中 {:02}:{:02}", mins, secs)
            }
        } else if let Some(end_ts) = free_end_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if now >= end_ts {
                // Break over — disable menu item, reset state, show window
                {
                    let tray_menu_state = app.state::<Mutex<TrayMenuState>>();
                    if let Ok(guard) = tray_menu_state.lock() {
                        guard.set_break_inactive();
                    };
                }
                {
                    let mut s = state.lock().unwrap();
                    log_break_session(&mut s);
                    let _ = app.emit("state-changed", s.clone());
                }
                show_main_window(&app, false);
                "🔒 计划中...".to_string()
            } else {
                let remaining = end_ts - now;
                let mins = remaining / 60;
                let secs = remaining % 60;
                format!("☕️ 休息中 {:02}:{:02}", mins, secs)
            }
        } else {
            "🔒 计划中...".to_string()
        };

        if let Some(tray) = app.tray_by_id("focus-tray") {
            let _ = tray.set_title(Some(&title));
        }
    }
}
