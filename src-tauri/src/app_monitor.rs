use crate::commands;
use crate::state::{lock_mutex, AppInfo, AppState};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::Manager;

#[cfg(target_os = "macos")]
use std::path::PathBuf;

#[cfg(target_os = "macos")]
static ICON_CACHE: std::sync::OnceLock<Mutex<HashMap<String, Option<String>>>> =
    std::sync::OnceLock::new();

#[cfg(target_os = "macos")]
fn icon_cache() -> &'static Mutex<HashMap<String, Option<String>>> {
    ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(target_os = "macos")]
fn ns_data_to_vec(data: &objc2_foundation::NSData) -> Vec<u8> {
    use std::ffi::c_void;
    use std::ptr::NonNull;

    let length = data.length();
    if length == 0 {
        return Vec::new();
    }

    let mut bytes = vec![0_u8; length];
    unsafe {
        data.getBytes_length(
            NonNull::new_unchecked(bytes.as_mut_ptr() as *mut c_void),
            length,
        );
    }
    bytes
}

#[cfg(target_os = "macos")]
fn app_icon_data_url(app: &objc2_app_kit::NSRunningApplication) -> Option<String> {
    let icon = app.icon()?;
    image_data_url(&icon)
}

#[cfg(target_os = "macos")]
fn image_data_url(icon: &objc2_app_kit::NSImage) -> Option<String> {
    use base64::Engine as _;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSBitmapImageRepPropertyKey};
    use objc2_foundation::NSDictionary;

    let tiff_data = icon.TIFFRepresentation()?;

    let bitmap = NSBitmapImageRep::imageRepWithData(&tiff_data)?;
    let properties = NSDictionary::<NSBitmapImageRepPropertyKey, AnyObject>::new();
    let png_data = unsafe {
        bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &properties)?
    };

    let bytes = ns_data_to_vec(&png_data);
    if bytes.is_empty() {
        return None;
    }

    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:image/png;base64,{encoded}"))
}

#[cfg(target_os = "macos")]
fn app_path_for_bundle_id_macos(bundle_id: &str) -> Option<PathBuf> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::NSString;

    let bundle = NSString::from_str(bundle_id);
    let app_url = NSWorkspace::sharedWorkspace().URLForApplicationWithBundleIdentifier(&bundle)?;
    app_url.to_file_path()
}

#[cfg(target_os = "macos")]
fn app_name_from_path_macos(path: &std::path::Path) -> Option<String> {
    path.file_stem()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .filter(|name| !name.is_empty())
}

#[cfg(target_os = "macos")]
fn app_icon_data_url_from_path_macos(path: &std::path::Path) -> Option<String> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::NSString;

    let ns_path = NSString::from_str(path.to_string_lossy().as_ref());
    let icon = NSWorkspace::sharedWorkspace().iconForFile(&ns_path);
    image_data_url(&icon)
}

pub fn get_app_info(bundle_id: &str, include_icon: bool) -> Option<AppInfo> {
    #[cfg(target_os = "macos")]
    {
        get_app_info_macos(bundle_id, include_icon)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (bundle_id, include_icon);
        None
    }
}

#[cfg(target_os = "macos")]
fn get_app_info_macos(bundle_id: &str, include_icon: bool) -> Option<AppInfo> {
    let app_path = app_path_for_bundle_id_macos(bundle_id)?;
    let name = app_name_from_path_macos(&app_path).unwrap_or_else(|| bundle_id.to_string());
    let icon_data_url = if include_icon {
        app_icon_data_url_from_path_macos(&app_path)
    } else {
        None
    };

    Some(AppInfo {
        name,
        bundle_id: bundle_id.to_string(),
        icon_data_url,
    })
}

/// Get list of currently running user-facing applications
pub fn get_running_apps(include_icons: bool) -> Vec<AppInfo> {
    #[cfg(target_os = "macos")]
    {
        get_running_apps_macos(include_icons)
    }
    #[cfg(not(target_os = "macos"))]
    {
        vec![]
    }
}

#[cfg(target_os = "macos")]
fn get_running_apps_macos(include_icons: bool) -> Vec<AppInfo> {
    use objc2_app_kit::NSWorkspace;

    let mut apps = Vec::new();

    let workspace = NSWorkspace::sharedWorkspace();
    let running_apps = workspace.runningApplications();

    for app in running_apps.iter() {
        // Only include regular (user-facing) applications
        if app.activationPolicy() == objc2_app_kit::NSApplicationActivationPolicy::Regular {
            let name = app
                .localizedName()
                .map(|n| n.to_string())
                .unwrap_or_default();
            let bundle_id = app
                .bundleIdentifier()
                .map(|id| id.to_string())
                .unwrap_or_default();

            if !bundle_id.is_empty() {
                let icon_data_url = if include_icons {
                    let cached = {
                        let cache = lock_mutex(icon_cache());
                        cache.get(&bundle_id).cloned()
                    };

                    if let Some(cached) = cached {
                        cached
                    } else {
                        let generated = app_icon_data_url(&app);
                        let mut cache = lock_mutex(icon_cache());
                        cache.insert(bundle_id.clone(), generated.clone());
                        generated
                    }
                } else {
                    None
                };

                apps.push(AppInfo {
                    name,
                    bundle_id,
                    icon_data_url,
                });
            }
        }
    }

    apps
}

pub fn get_app_icon(bundle_id: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        get_app_icon_macos(bundle_id)
            .or_else(|| get_app_info(bundle_id, true).and_then(|app| app.icon_data_url))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = bundle_id;
        None
    }
}

#[cfg(target_os = "macos")]
fn get_app_icon_macos(bundle_id: &str) -> Option<String> {
    use objc2_app_kit::NSWorkspace;

    let cached = {
        let cache = lock_mutex(icon_cache());
        cache.get(bundle_id).cloned()
    };
    if let Some(cached) = cached {
        return cached;
    }

    let workspace = NSWorkspace::sharedWorkspace();
    let running_apps = workspace.runningApplications();

    let generated = running_apps.iter().find_map(|app| {
        if app.activationPolicy() != objc2_app_kit::NSApplicationActivationPolicy::Regular {
            return None;
        }

        let app_bundle_id = app
            .bundleIdentifier()
            .map(|id| id.to_string())
            .unwrap_or_default();

        if app_bundle_id == bundle_id {
            return app_icon_data_url(&app);
        }

        None
    });

    let generated =
        generated.or_else(|| get_app_info_macos(bundle_id, true).and_then(|app| app.icon_data_url));

    let mut cache = lock_mutex(icon_cache());
    cache.insert(bundle_id.to_string(), generated.clone());

    generated
}

/// Start monitoring the frontmost application.
/// Runs in a loop on a background thread; shows blocking window for non-whitelisted apps.
pub fn start_monitoring(app: tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        start_monitoring_macos(app);
    }
}

/// A stable fingerprint of the current display layout (positions + sizes).
/// Used to detect monitors being connected/disconnected or resized.
/// Call on the main thread.
#[cfg(target_os = "macos")]
fn monitors_signature(app: &tauri::AppHandle) -> String {
    let Some(win) = app.get_webview_window("main") else {
        return String::new();
    };
    let mut parts: Vec<String> = win
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let p = m.position();
            let s = m.size();
            format!("{}:{}:{}:{}", p.x, p.y, s.width, s.height)
        })
        .collect();
    parts.sort();
    parts.join("|")
}

#[cfg(target_os = "macos")]
fn start_monitoring_macos(app: tauri::AppHandle) {
    use objc2_app_kit::NSWorkspace;
    use std::thread;
    use std::time::{Duration, Instant};

    let self_bundle_id = app.config().identifier.clone();

    // Throttle distraction notifications so repeatedly opening a blocked app
    // doesn't flood Notification Center.
    let mut last_notified_bundle: Option<String> = None;
    let mut last_notified_at: Option<Instant> = None;
    const NOTIFY_THROTTLE_SECS: u64 = 20;

    // Track the display layout so overlays can be rebuilt on hot-plug. All
    // monitor/window queries run on the main thread (AppKit is not thread-safe).
    let last_monitor_sig = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    {
        let app_init = app.clone();
        let sig_init = last_monitor_sig.clone();
        let _ = app.run_on_main_thread(move || {
            if let Ok(mut guard) = sig_init.lock() {
                *guard = monitors_signature(&app_init);
            }
        });
    }
    let mut display_check_tick: u32 = 0;

    loop {
        if let Some(front_app) = NSWorkspace::sharedWorkspace().frontmostApplication() {
            let bundle_id = front_app
                .bundleIdentifier()
                .map(|id| id.to_string())
                .unwrap_or_default();

            // Skip empty ids and our own windows.
            if !bundle_id.is_empty() && bundle_id != self_bundle_id {
                let current_app_name = front_app
                    .localizedName()
                    .map(|n| n.to_string())
                    .unwrap_or_default();

                let (allowed, is_restricted, is_free_activity, has_focus_session, locale) = {
                    let state = app.state::<Mutex<AppState>>();
                    let mut s = lock_mutex(&state);
                    // Diagnostics: record the live frontmost app for the self-check.
                    s.last_frontmost = Some(bundle_id.clone());
                    (
                        s.is_app_allowed(&bundle_id),
                        s.is_restricted,
                        s.free_activity_end_at.is_some(),
                        s.focus_started_at.is_some(),
                        s.locale.clone(),
                    )
                };

                // is_app_allowed already returns true while paused, so a paused
                // session never blocks.
                let should_block =
                    has_focus_session && is_restricted && !is_free_activity && !allowed;

                if should_block {
                    // Gently collect the distracting app (Cmd+H equivalent).
                    let _ = front_app.hide();

                    // Notify, throttled per app so repeats don't pile up.
                    let now = Instant::now();
                    let same_app = last_notified_bundle.as_deref() == Some(bundle_id.as_str());
                    let recently = last_notified_at
                        .map(|t| now.duration_since(t).as_secs() < NOTIFY_THROTTLE_SECS)
                        .unwrap_or(false);
                    if !(same_app && recently) {
                        commands::notify_distraction(&app, &current_app_name, &locale);
                        last_notified_bundle = Some(bundle_id.clone());
                        last_notified_at = Some(now);
                    }
                }
            }
        }

        // Roughly every 2s, rebuild overlays if the display layout changed
        // (monitor connected/disconnected or resolution change).
        display_check_tick = display_check_tick.wrapping_add(1);
        if display_check_tick % 4 == 0 {
            let app_chk = app.clone();
            let sig_ref = last_monitor_sig.clone();
            let _ = app.run_on_main_thread(move || {
                let sig = monitors_signature(&app_chk);
                let changed = match sig_ref.lock() {
                    Ok(mut guard) if *guard != sig => {
                        *guard = sig;
                        true
                    }
                    _ => false,
                };
                if !changed {
                    return;
                }

                commands::sync_overlays(&app_chk);
                commands::refresh_monitors_info(&app_chk);

                // Re-show overlays if the windows are currently meant to be visible.
                let main_visible = app_chk
                    .get_webview_window("main")
                    .and_then(|w| w.is_visible().ok())
                    .unwrap_or(false);
                if main_visible {
                    for (label, win) in app_chk.webview_windows() {
                        if label.starts_with("overlay-") {
                            let _ = win.show();
                        }
                    }
                }
            });
        }

        /// Interval between frontmost-app polling checks.
        const APP_MONITOR_POLL_INTERVAL_MS: u64 = 500;
        thread::sleep(Duration::from_millis(APP_MONITOR_POLL_INTERVAL_MS));
    }
}
