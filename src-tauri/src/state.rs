use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Minimum session duration (in seconds) required to persist a session record.
pub const MIN_SESSION_DURATION_SECS: u64 = 10;

/// Apps that are always allowed regardless of user whitelist.
pub const DEFAULT_WHITELIST: &[&str] = &[
    "com.apple.finder",
    "com.apple.systempreferences",
    "com.focus-must",
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn lock_mutex<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            eprintln!("Recovering from poisoned mutex");
            poisoned.into_inner()
        }
    }
}

pub fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

// ---------------------------------------------------------------------------
// Data Structures
// ---------------------------------------------------------------------------

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
    /// Break-reminder interval in minutes for the active focus session
    /// (0 = no reminder). Set when a focus session starts.
    #[serde(default)]
    pub focus_goal_minutes: u64,
    /// Per-app temporary passes: bundle id -> unix expiry. While unexpired, the
    /// app is allowed even during a restricted focus session ("use once" grace).
    #[serde(default)]
    pub temp_allowed: HashMap<String, u64>,
    /// Whether the distraction prompt window is currently shown. Shared so the
    /// monitor thread and the frontend commands can coordinate (the prompt
    /// stays up until the user chooses, instead of auto-dismissing). Not
    /// persisted or sent to the frontend.
    #[serde(skip)]
    pub prompt_active: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            is_restricted: true,
            default_whitelist: DEFAULT_WHITELIST.iter().map(|s| s.to_string()).collect(),
            session_whitelist: vec![],
            task_description: None,
            focus_started_at: None,
            free_activity_started_at: None,
            free_activity_end_at: None,
            locale: "system".to_string(),
            focus_goal_minutes: 0,
            temp_allowed: HashMap::new(),
            prompt_active: false,
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
        if let Some(&until) = self.temp_allowed.get(bundle_id) {
            if until > unix_now_secs() {
                return true;
            }
        }
        self.default_whitelist.iter().any(|id| id == bundle_id)
            || self.session_whitelist.iter().any(|id| id == bundle_id)
    }
}

// ---------------------------------------------------------------------------
// Locale Helpers
// ---------------------------------------------------------------------------

/// Read the user's preferred macOS UI language (NSGlobalDomain `AppleLanguages`).
/// This matches what the webview reports via `navigator.language`, so the tray
/// stays consistent with the in-app UI. GUI apps usually have no useful `LANG`
/// env var, which is why the old env-only detection was unreliable.
#[cfg(target_os = "macos")]
fn macos_system_language() -> Option<String> {
    let output = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLanguages"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    // Output looks like: (\n    "en-US",\n    "zh-Hans-CN"\n)
    let text = String::from_utf8_lossy(&output.stdout);
    let first = text.split('"').nth(1)?.trim().to_string();
    if first.is_empty() {
        None
    } else {
        Some(first)
    }
}

/// Detect the effective system language as one of the supported locales.
/// Cached for the process lifetime: the OS language only changes across an app
/// relaunch (matching the webview), so this stays cheap on the per-second tray
/// update path.
pub fn detect_system_locale() -> &'static str {
    static CACHE: OnceLock<&'static str> = OnceLock::new();
    *CACHE.get_or_init(|| {
        #[cfg(target_os = "macos")]
        {
            if let Some(lang) = macos_system_language() {
                return if lang.to_ascii_lowercase().starts_with("en") {
                    "en-US"
                } else {
                    "zh-CN"
                };
            }
        }

        let lang = std::env::var("LANG")
            .unwrap_or_default()
            .to_ascii_lowercase();
        if lang.starts_with("en") {
            "en-US"
        } else {
            "zh-CN"
        }
    })
}

pub fn normalize_locale(locale: &str) -> &'static str {
    if locale.eq_ignore_ascii_case("system") {
        "system"
    } else if locale.to_ascii_lowercase().starts_with("en") {
        "en-US"
    } else {
        "zh-CN"
    }
}

pub fn resolve_effective_locale(locale: &str) -> &'static str {
    let normalized = normalize_locale(locale);
    if normalized == "system" {
        detect_system_locale()
    } else {
        normalized
    }
}

pub fn locale_is_en(locale: &str) -> bool {
    resolve_effective_locale(locale) == "en-US"
}
