use serde::{Deserialize, Serialize};
use std::sync::Mutex;
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
// Locale Helpers
// ---------------------------------------------------------------------------

pub fn detect_system_locale() -> &'static str {
    let lang = std::env::var("LANG")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if lang.starts_with("en") {
        "en-US"
    } else {
        "zh-CN"
    }
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
