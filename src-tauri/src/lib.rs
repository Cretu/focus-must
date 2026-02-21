use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{
    menu::{IconMenuItem, Menu, NativeIcon},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

mod app_monitor;
mod storage;

fn lock_mutex<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            eprintln!("Recovering from poisoned mutex");
            poisoned.into_inner()
        }
    }
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

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
    pub locale: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            is_restricted: true,
            default_whitelist: vec![
                "com.apple.finder".to_string(),
                "com.apple.systempreferences".to_string(),
                "com.focus-must".to_string(),
            ],
            session_whitelist: vec![],
            task_description: None,
            focus_started_at: None,
            free_activity_started_at: None,
            free_activity_end_at: None,
            locale: "system".to_string(),
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

fn detect_system_locale() -> &'static str {
    let lang = std::env::var("LANG")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if lang.starts_with("en") {
        "en-US"
    } else {
        "zh-CN"
    }
}

fn normalize_locale(locale: &str) -> &'static str {
    if locale.eq_ignore_ascii_case("system") {
        "system"
    } else if locale.to_ascii_lowercase().starts_with("en") {
        "en-US"
    } else {
        "zh-CN"
    }
}

fn resolve_effective_locale(locale: &str) -> &'static str {
    let normalized = normalize_locale(locale);
    if normalized == "system" {
        detect_system_locale()
    } else {
        normalized
    }
}

fn locale_is_en(locale: &str) -> bool {
    resolve_effective_locale(locale) == "en-US"
}

fn menu_text_show(locale: &str) -> &'static str {
    if locale_is_en(locale) {
        "Show Planner"
    } else {
        "显示计划窗口"
    }
}

fn menu_text_lock(locale: &str, active: bool) -> &'static str {
    if locale_is_en(locale) {
        if active {
            "End Focus"
        } else {
            "End Focus (Not Started)"
        }
    } else if active {
        "结束专注"
    } else {
        "结束专注 (未开始)"
    }
}

fn menu_text_break(locale: &str, active: bool) -> &'static str {
    if locale_is_en(locale) {
        if active {
            "End Break"
        } else {
            "End Break (Not Started)"
        }
    } else if active {
        "结束休息"
    } else {
        "结束休息 (未开始)"
    }
}

fn menu_text_settings(locale: &str) -> &'static str {
    if locale_is_en(locale) {
        "Settings"
    } else {
        "设置"
    }
}

fn menu_text_quit(locale: &str) -> &'static str {
    if locale_is_en(locale) {
        "Quit"
    } else {
        "退出"
    }
}

fn tray_title_planning(locale: &str) -> &'static str {
    if locale_is_en(locale) {
        "Planning"
    } else {
        "计划中"
    }
}

fn tray_title_focus(locale: &str, hours: u64, mins: u64, secs: u64) -> String {
    if locale_is_en(locale) {
        if hours > 0 {
            format!("Focusing {:02}:{:02}:{:02}", hours, mins, secs)
        } else {
            format!("Focusing {:02}:{:02}", mins, secs)
        }
    } else if hours > 0 {
        format!("专注中 {:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("专注中 {:02}:{:02}", mins, secs)
    }
}

fn tray_title_break(locale: &str, mins: u64, secs: u64) -> String {
    if locale_is_en(locale) {
        format!("On Break {:02}:{:02}", mins, secs)
    } else {
        format!("休息中 {:02}:{:02}", mins, secs)
    }
}

fn tray_title_break_minutes(locale: &str, mins: u64) -> String {
    if locale_is_en(locale) {
        format!("On Break {:02}:00", mins)
    } else {
        format!("休息中 {:02}:00", mins)
    }
}

// ---------------------------------------------------------------------------
// Tray Menu State — helpers to reduce repetition
// ---------------------------------------------------------------------------
struct TrayMenuState {
    locale: String,
    show_item: Option<IconMenuItem<tauri::Wry>>,
    lock_item: Option<IconMenuItem<tauri::Wry>>,
    end_break_item: Option<IconMenuItem<tauri::Wry>>,
    settings_item: Option<IconMenuItem<tauri::Wry>>,
    quit_item: Option<IconMenuItem<tauri::Wry>>,
}

impl TrayMenuState {
    fn set_locale(&mut self, locale: String) {
        self.locale = normalize_locale(&locale).to_string();
        self.refresh_static_labels();
    }

    fn refresh_static_labels(&self) {
        if let Some(item) = &self.show_item {
            let _ = item.set_text(menu_text_show(&self.locale));
        }
        if let Some(item) = &self.settings_item {
            let _ = item.set_text(menu_text_settings(&self.locale));
        }
        if let Some(item) = &self.quit_item {
            let _ = item.set_text(menu_text_quit(&self.locale));
        }
    }

    fn set_focus_active(&self) {
        if let Some(item) = &self.lock_item {
            let _ = item.set_enabled(true);
            let _ = item.set_text(menu_text_lock(&self.locale, true));
        }
        if let Some(item) = &self.end_break_item {
            let _ = item.set_enabled(false);
            let _ = item.set_text(menu_text_break(&self.locale, false));
        }
    }

    fn set_focus_inactive(&self) {
        if let Some(item) = &self.lock_item {
            let _ = item.set_enabled(false);
            let _ = item.set_text(menu_text_lock(&self.locale, false));
        }
    }

    fn set_break_active(&self) {
        if let Some(item) = &self.end_break_item {
            let _ = item.set_enabled(true);
            let _ = item.set_text(menu_text_break(&self.locale, true));
        }
        if let Some(item) = &self.lock_item {
            let _ = item.set_enabled(false);
            let _ = item.set_text(menu_text_lock(&self.locale, false));
        }
    }

    fn set_break_inactive(&self) {
        if let Some(item) = &self.end_break_item {
            let _ = item.set_enabled(false);
            let _ = item.set_text(menu_text_break(&self.locale, false));
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
        let mut s = lock_mutex(&state);

        // Log session if it was a focus session
        if let Some(start) = s.focus_started_at {
            let now = unix_now_secs();
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
        let now = unix_now_secs();
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
fn get_running_apps(include_icons: Option<bool>) -> Vec<AppInfo> {
    app_monitor::get_running_apps(include_icons.unwrap_or(false))
}

#[tauri::command]
fn get_app_icon(bundle_id: String) -> Option<String> {
    app_monitor::get_app_icon(&bundle_id)
}

#[tauri::command]
fn get_state(state: tauri::State<'_, Mutex<AppState>>) -> AppState {
    lock_mutex(&state).clone()
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
fn set_locale(
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
        let _ = tray.set_title(Some(tray_title_planning(&normalized)));
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

#[tauri::command]
fn get_history_page(offset: Option<u64>, limit: Option<u64>) -> storage::HistoryPage {
    storage::load_sessions_page(offset.unwrap_or(0), limit.unwrap_or(100))
}

#[tauri::command]
fn get_analytics() -> storage::AnalyticsData {
    storage::load_analytics()
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
        let mut s = lock_mutex(&state);
        let now = unix_now_secs();
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
        let locale = {
            let s = lock_mutex(&state);
            s.locale.clone()
        };
        let _ = tray.set_title(Some(&tray_title_break_minutes(&locale, duration_minutes)));
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
            state.locale = normalize_locale(&settings.locale).to_string();
            state
        }))
        .manage(Mutex::new(TrayMenuState {
            locale: "system".to_string(),
            show_item: None,
            lock_item: None,
            end_break_item: None,
            settings_item: None,
            quit_item: None,
        }))
        .invoke_handler(tauri::generate_handler![
            get_running_apps,
            get_app_icon,
            get_state,
            unlock_session,
            lock_session,
            switch_to_app,
            start_free_activity,
            update_settings,
            set_locale,
            get_history,
            get_history_page,
            get_analytics,
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
            let locale = {
                let state = app.state::<Mutex<AppState>>();
                let s = lock_mutex(&state);
                s.locale.clone()
            };

            let show_i = IconMenuItem::with_id_and_native_icon(
                app,
                "show",
                menu_text_show(&locale),
                true,
                Some(NativeIcon::Home),
                None::<&str>,
            )?;
            let lock_i = IconMenuItem::with_id_and_native_icon(
                app,
                "lock",
                menu_text_lock(&locale, false),
                false,
                Some(NativeIcon::LockLocked),
                None::<&str>,
            )?;
            let end_break_i = IconMenuItem::with_id_and_native_icon(
                app,
                "end_break",
                menu_text_break(&locale, false),
                false,
                Some(NativeIcon::Refresh),
                None::<&str>,
            )?;
            let settings_i = IconMenuItem::with_id_and_native_icon(
                app,
                "settings",
                menu_text_settings(&locale),
                true,
                Some(NativeIcon::RefreshFreestanding),
                None::<&str>,
            )?;
            let quit_i = IconMenuItem::with_id_and_native_icon(
                app,
                "quit",
                menu_text_quit(&locale),
                true,
                Some(NativeIcon::StopProgressFreestanding),
                None::<&str>,
            )?;

            // Store clones for dynamic updates
            {
                let ts = app.state::<Mutex<TrayMenuState>>();
                if let Ok(mut s) = ts.lock() {
                    s.locale = locale.clone();
                    s.show_item = Some(show_i.clone());
                    s.lock_item = Some(lock_i.clone());
                    s.end_break_item = Some(end_break_i.clone());
                    s.settings_item = Some(settings_i.clone());
                    s.quit_item = Some(quit_i.clone());
                };
            }

            let menu =
                Menu::with_items(app, &[&show_i, &lock_i, &end_break_i, &settings_i, &quit_i])?;

            let mut tray_builder = TrayIconBuilder::with_id("focus-tray");
            if let Some(icon) = app.default_window_icon().cloned() {
                tray_builder = tray_builder.icon(icon);
            } else {
                eprintln!(
                    "Default window icon is missing; tray icon will rely on platform defaults"
                );
            }

            let _tray = tray_builder
                .icon_as_template(true)
                .menu(&menu)
                .title(tray_title_planning(&locale))
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
                            let mut s = lock_mutex(&state);
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
                            let locale = {
                                let state = app.state::<Mutex<AppState>>();
                                let s = lock_mutex(&state);
                                s.locale.clone()
                            };
                            let _ = tray.set_title(Some(tray_title_planning(&locale)));
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
        .unwrap_or_else(|error| {
            eprintln!("error while running tauri application: {error}");
        });
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
        let (started_at, free_end_at, locale) = {
            let s = lock_mutex(&state);
            (s.focus_started_at, s.free_activity_end_at, s.locale.clone())
        };

        let title = if let Some(start_ts) = started_at {
            let now = unix_now_secs();
            let elapsed = now.saturating_sub(start_ts);

            let hours = elapsed / 3600;
            let mins = (elapsed % 3600) / 60;
            let secs = elapsed % 60;

            tray_title_focus(&locale, hours, mins, secs)
        } else if let Some(end_ts) = free_end_at {
            let now = unix_now_secs();

            if now >= end_ts {
                // Break over — disable menu item, reset state, show window
                {
                    let tray_menu_state = app.state::<Mutex<TrayMenuState>>();
                    if let Ok(guard) = tray_menu_state.lock() {
                        guard.set_break_inactive();
                    };
                }
                {
                    let mut s = lock_mutex(&state);
                    log_break_session(&mut s);
                    let _ = app.emit("state-changed", s.clone());
                }
                show_main_window(&app, false);
                tray_title_planning(&locale).to_string()
            } else {
                let remaining = end_ts - now;
                let mins = remaining / 60;
                let secs = remaining % 60;
                tray_title_break(&locale, mins, secs)
            }
        } else {
            tray_title_planning(&locale).to_string()
        };

        if let Some(tray) = app.tray_by_id("focus-tray") {
            let _ = tray.set_title(Some(&title));
        }
    }
}
