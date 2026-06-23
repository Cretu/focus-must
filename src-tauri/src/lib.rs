use state::{lock_mutex, normalize_locale, unix_now_secs, AppState};
use std::sync::Mutex;
use tauri::{
    menu::{IconMenuItem, Menu, NativeIcon},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tray::{tray_locale, tray_title_break, tray_title_focus, TrayMenuState};

pub mod app_monitor;
pub mod commands;
pub mod state;
pub mod storage;
pub mod tray;

// ---------------------------------------------------------------------------
// Tray title updater — runs every second on a background thread
// ---------------------------------------------------------------------------

/// Interval in seconds for updating the tray title.
const TRAY_TITLE_UPDATE_INTERVAL_SECS: u64 = 1;

fn tray_title_updater(app: tauri::AppHandle) {
    use std::thread;
    use std::time::Duration;

    // Track the focus session we're reminding for, and how many break reminders
    // have fired (so reminders repeat every `focus_goal_minutes`).
    let mut reminder_session: Option<u64> = None;
    let mut reminders_fired: u64 = 0;

    loop {
        thread::sleep(Duration::from_secs(TRAY_TITLE_UPDATE_INTERVAL_SECS));

        let state = app.state::<Mutex<AppState>>();
        let (started_at, free_end_at, locale, temp_allowed, focus_goal_minutes) = {
            let s = lock_mutex(&state);
            (
                s.focus_started_at,
                s.free_activity_end_at,
                s.locale.clone(),
                s.temp_allowed.clone(),
                s.focus_goal_minutes,
            )
        };

        let title = if let Some(start_ts) = started_at {
            let now = unix_now_secs();
            let elapsed = now.saturating_sub(start_ts);

            let hours = elapsed / 3600;
            let mins = (elapsed % 3600) / 60;
            let secs = elapsed % 60;

            // Break reminder: fire a sound + popup every `focus_goal_minutes`.
            if reminder_session != Some(start_ts) {
                reminder_session = Some(start_ts);
                reminders_fired = 0;
            }
            if focus_goal_minutes > 0 {
                let goal_secs = focus_goal_minutes * 60;
                if elapsed >= goal_secs * (reminders_fired + 1) {
                    reminders_fired += 1;
                    commands::play_sound("Glass");
                    commands::show_main_window(&app, false);
                    let _ = app.emit(
                        "focus-goal-reached",
                        serde_json::json!({ "minutes": focus_goal_minutes * reminders_fired }),
                    );
                }
            }

            let mut title = tray_title_focus(&locale, hours, mins, secs);

            // Append the soonest active temporary-pass countdown, if any.
            if let Some(min_until) = temp_allowed.values().copied().filter(|u| *u > now).min() {
                let remaining = min_until - now;
                title = format!("{title}  ⏳{}:{:02}", remaining / 60, remaining % 60);
            }

            title
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
                    commands::log_break_session(&mut s);
                    let _ = app.emit("state-changed", s.clone());
                }
                commands::show_main_window(&app, false);
                tray_locale(&locale).planning.to_string()
            } else {
                let remaining = end_ts - now;
                let mins = remaining / 60;
                let secs = remaining % 60;
                tray_title_break(&locale, mins, secs)
            }
        } else {
            tray_locale(&locale).planning.to_string()
        };

        if let Some(tray) = app.tray_by_id("focus-tray") {
            let _ = tray.set_title(Some(&title));
        }
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
            let mut app_state = AppState::default();
            // Load persistent settings
            let settings = storage::load_settings();
            if !settings.default_whitelist.is_empty() {
                app_state.default_whitelist = settings.default_whitelist;
            }
            app_state.locale = normalize_locale(&settings.locale).to_string();
            app_state
        }))
        .manage(Mutex::new(TrayMenuState::new()))
        .invoke_handler(tauri::generate_handler![
            commands::get_running_apps,
            commands::get_app_icon,
            commands::get_app_info,
            commands::get_state,
            commands::unlock_session,
            commands::lock_session,
            commands::hide_windows,
            commands::switch_to_app,
            commands::allow_app_temporarily,
            commands::dismiss_distraction,
            commands::start_free_activity,
            commands::update_settings,
            commands::set_locale,
            commands::get_history_page,
            commands::get_analytics,
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
                tray_locale(&locale).show,
                true,
                Some(NativeIcon::Home),
                None::<&str>,
            )?;
            let lock_i = IconMenuItem::with_id_and_native_icon(
                app,
                "lock",
                tray_locale(&locale).lock_inactive,
                false,
                Some(NativeIcon::LockLocked),
                None::<&str>,
            )?;
            let end_break_i = IconMenuItem::with_id_and_native_icon(
                app,
                "end_break",
                tray_locale(&locale).break_inactive,
                false,
                Some(NativeIcon::Refresh),
                None::<&str>,
            )?;
            let settings_i = IconMenuItem::with_id_and_native_icon(
                app,
                "settings",
                tray_locale(&locale).settings,
                true,
                Some(NativeIcon::RefreshFreestanding),
                None::<&str>,
            )?;
            let quit_i = IconMenuItem::with_id_and_native_icon(
                app,
                "quit",
                tray_locale(&locale).quit,
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
                .title(tray_locale(&locale).planning)
                .tooltip("Focus Must")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        let _ = app.emit("show-view", "planning");
                        commands::show_main_window(app, false);
                    }
                    "settings" => {
                        let _ = app.emit("show-view", "settings");
                        commands::show_main_window(app, false);
                    }
                    "lock" => {
                        commands::do_lock_session(app);
                    }
                    "end_break" => {
                        {
                            let state = app.state::<Mutex<AppState>>();
                            let mut s = lock_mutex(&state);
                            commands::log_break_session(&mut s);
                            let _ = app.emit("state-changed", s.clone());
                        }
                        {
                            let ts = app.state::<Mutex<TrayMenuState>>();
                            if let Ok(ts) = ts.lock() {
                                ts.set_break_inactive();
                            };
                        }
                        commands::show_main_window(app, false);
                        if let Some(tray) = app.tray_by_id("focus-tray") {
                            let locale = {
                                let state = app.state::<Mutex<AppState>>();
                                let s = lock_mutex(&state);
                                s.locale.clone()
                            };
                            let _ = tray.set_title(Some(tray_locale(&locale).planning));
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
                        commands::show_main_window(tray.app_handle(), false);
                    }
                })
                .build(app)?;

            // --- Setup blocking windows for all monitors ---
            #[cfg(target_os = "macos")]
            {
                // Main window visible on all macOS Spaces.
                if let Some(win) = app.get_webview_window("main") {
                    use objc2::msg_send;
                    if let Ok(ns_window) = win.ns_window() {
                        let ns_win: *mut objc2::runtime::AnyObject = ns_window.cast();
                        unsafe {
                            let behavior: isize = (1 << 0) | (1 << 4);
                            let _: () = msg_send![&*ns_win, setCollectionBehavior: behavior];
                        }
                    }
                }

                // Size the main window to the primary monitor and create overlay
                // windows covering every secondary monitor.
                commands::sync_overlays(app.handle());

                // Small modal window used for the distraction prompt.
                commands::create_prompt_window(app.handle());
            }

            commands::show_main_window(app.handle(), false);

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
