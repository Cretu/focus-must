use crate::{AppInfo, AppState};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

/// Get list of currently running user-facing applications
pub fn get_running_apps() -> Vec<AppInfo> {
    #[cfg(target_os = "macos")]
    {
        get_running_apps_macos()
    }
    #[cfg(not(target_os = "macos"))]
    {
        vec![]
    }
}

#[cfg(target_os = "macos")]
fn get_running_apps_macos() -> Vec<AppInfo> {
    use objc2_app_kit::NSWorkspace;

    let mut apps = Vec::new();

    let workspace = NSWorkspace::sharedWorkspace();
    let running_apps = workspace.runningApplications();

    for app in running_apps.iter() {
        // Only include regular (user-facing) applications
        if app.activationPolicy()
            == objc2_app_kit::NSApplicationActivationPolicy::Regular
        {
            let name = app
                .localizedName()
                .map(|n| n.to_string())
                .unwrap_or_default();
            let bundle_id = app
                .bundleIdentifier()
                .map(|id| id.to_string())
                .unwrap_or_default();

            if !bundle_id.is_empty() {
                apps.push(AppInfo { name, bundle_id });
            }
        }
    }

    apps
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
    use std::time::Duration;

    let mut last_bundle_id = String::new();
    let mut last_valid_bundle_id = String::new();
    let mut last_valid_app_name = String::new();

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
                
                let current_app_name = front_app
                    .localizedName()
                    .map(|n| n.to_string())
                    .unwrap_or_default();

                let state = app.state::<Mutex<AppState>>();
                let s = state.lock().unwrap();
                let allowed = s.is_app_allowed(&bundle_id);
                let is_restricted = s.is_restricted;
                let is_free_activity = s.free_activity_end_at.is_some();

                drop(s);
                
                if allowed && !is_free_activity {
                    last_valid_bundle_id = bundle_id.clone();
                    last_valid_app_name = current_app_name.clone();
                }

                if is_restricted && !allowed {
                    // Show blocking window — user switched to a non-whitelisted app
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.set_always_on_top(true);
                        let _ = win.show();
                        let _ = win.set_focus();
                    }

                    let _ = app.emit(
                        "blocked-app",
                        serde_json::json!({
                            "name": current_app_name,
                            "bundle_id": bundle_id,
                            "return_to_bundle_id": last_valid_bundle_id,
                            "return_to_name": last_valid_app_name,
                        }),
                    );
                } else if is_restricted && allowed {
                    // Hide blocking window — user is on a whitelisted app
                    if let Some(win) = app.get_webview_window("main") {
                        let _ = win.hide();
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(500));
    }
}
