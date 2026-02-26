use crate::{lock_mutex, AppInfo, AppState};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

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

    let length = data.length() as usize;
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
    use base64::Engine as _;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSBitmapImageRepPropertyKey};
    use objc2_foundation::NSDictionary;

    let icon = app.icon()?;
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

#[cfg(target_os = "macos")]
fn start_monitoring_macos(app: tauri::AppHandle) {
    use objc2_app_kit::NSWorkspace;
    use std::thread;
    use std::time::{Duration, Instant};

    let self_bundle_id = app.config().identifier.clone();
    let mut last_bundle_id = String::new();
    let mut last_valid_bundle_id = String::new();
    let mut last_valid_app_name = String::new();
    let mut blocking_visible = false;
    let mut blocking_shown_at: Option<Instant> = None;

    loop {
        let workspace = NSWorkspace::sharedWorkspace();

        if let Some(front_app) = workspace.frontmostApplication() {
            let bundle_id = front_app
                .bundleIdentifier()
                .map(|id| id.to_string())
                .unwrap_or_default();

            // Only react when the frontmost app changes
            if !bundle_id.is_empty() && bundle_id != last_bundle_id {
                last_bundle_id = bundle_id.clone();

                if bundle_id == self_bundle_id {
                    continue;
                }

                let current_app_name = front_app
                    .localizedName()
                    .map(|n| n.to_string())
                    .unwrap_or_default();

                let state = app.state::<Mutex<AppState>>();
                let s = lock_mutex(&state);
                let allowed = s.is_app_allowed(&bundle_id);
                let is_restricted = s.is_restricted;
                let is_free_activity = s.free_activity_end_at.is_some();
                let has_focus_session = s.focus_started_at.is_some();

                drop(s);

                if allowed && has_focus_session && !is_free_activity {
                    last_valid_bundle_id = bundle_id.clone();
                    last_valid_app_name = current_app_name.clone();
                }

                if is_restricted && !allowed {
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.set_always_on_top(true);
                        let _ = win.show();
                    }

                    blocking_visible = true;
                    blocking_shown_at = Some(Instant::now());

                    let _ = app.emit(
                        "blocked-app",
                        serde_json::json!({
                            "name": current_app_name,
                            "bundle_id": bundle_id,
                            "return_to_bundle_id": if has_focus_session { Some(last_valid_bundle_id.clone()) } else { None },
                            "return_to_name": if has_focus_session { Some(last_valid_app_name.clone()) } else { None },
                        }),
                    );
                } else if is_restricted && allowed && blocking_visible {
                    let can_hide_now = blocking_shown_at
                        .map(|t| t.elapsed() >= Duration::from_millis(700))
                        .unwrap_or(true);

                    if !can_hide_now {
                        continue;
                    }

                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.hide();
                    }

                    let _ = app.emit("blocked-app-cleared", serde_json::Value::Null);

                    blocking_visible = false;
                    blocking_shown_at = None;
                }
            }
        }

        thread::sleep(Duration::from_millis(500));
    }
}
