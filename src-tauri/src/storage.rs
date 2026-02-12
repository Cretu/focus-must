use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const SETTINGS_FILE: &str = "settings.json";
const SESSIONS_FILE: &str = "sessions.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    #[serde(default)]
    pub default_whitelist: Vec<String>,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            default_whitelist: vec![
                "com.apple.finder".to_string(),
                "com.apple.systempreferences".to_string(),
                "com.focus-must".to_string(),
            ],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_type: String, // "focus" | "break"
    pub started_at: u64,
    pub ended_at: u64,
    pub duration_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(default)]
    pub whitelist: Vec<String>,
}

fn data_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME env var not set");
    let path = Path::new(&home).join(".focusmust");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path
}

pub fn load_settings() -> UserSettings {
    let path = data_dir().join(SETTINGS_FILE);
    if path.exists() {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(settings) = serde_json::from_str(&content) {
                return settings;
            }
        }
    }
    UserSettings::default()
}

pub fn save_settings(settings: &UserSettings) {
    let path = data_dir().join(SETTINGS_FILE);
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        let _ = fs::write(path, json);
    }
}

pub fn append_session(record: &SessionRecord) {
    let path = data_dir().join(SESSIONS_FILE);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        if let Ok(json) = serde_json::to_string(record) {
            let _ = writeln!(file, "{}", json);
        }
    }
}

pub fn load_sessions() -> Vec<SessionRecord> {
    let path = data_dir().join(SESSIONS_FILE);
    let mut sessions = Vec::new();
    if path.exists() {
        if let Ok(file) = fs::File::open(path) {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(line) = line {
                    if let Ok(record) = serde_json::from_str::<SessionRecord>(&line) {
                        sessions.push(record);
                    }
                }
            }
        }
    }
    // Newest first
    sessions.reverse();
    sessions
}
