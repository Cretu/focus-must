use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const SETTINGS_FILE: &str = "settings.json";
const SESSIONS_FILE: &str = "sessions.jsonl";
const SESSIONS_FILE_MIGRATED: &str = "sessions.jsonl.migrated";
const DB_FILE: &str = "history.db";

// ---------------------------------------------------------------------------
// Singleton DB connection
// ---------------------------------------------------------------------------
static DB: std::sync::OnceLock<Mutex<Connection>> = std::sync::OnceLock::new();

fn init_db() -> Option<&'static Mutex<Connection>> {
    Some(DB.get_or_init(|| {
        let conn = Connection::open(db_path()).expect("Failed to open history database");
        initialize_schema(&conn).expect("Failed to initialize database schema");
        migrate_jsonl_if_needed(&conn);
        Mutex::new(conn)
    }))
}

fn with_db<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&Connection) -> Option<R>,
{
    let db = init_db()?;
    let conn = crate::lock_mutex(db);
    f(&conn)
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    #[serde(default)]
    pub default_whitelist: Vec<String>,
    #[serde(default = "default_locale")]
    pub locale: String,
}

fn default_locale() -> String {
    "system".to_string()
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            default_whitelist: crate::DEFAULT_WHITELIST
                .iter()
                .map(|s| s.to_string())
                .collect(),
            locale: default_locale(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_type: String,
    pub started_at: u64,
    pub ended_at: u64,
    pub duration_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(default)]
    pub whitelist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HistoryPage {
    pub items: Vec<SessionRecord>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalyticsSummary {
    pub total_focus_secs: u64,
    pub total_break_secs: u64,
    pub total_sessions: u64,
    pub focus_sessions: u64,
    pub break_sessions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyTrendPoint {
    pub day: String,
    pub focus_secs: u64,
    pub break_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusHourBucket {
    pub hour: u8,
    pub focus_secs: u64,
    pub sessions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalyticsData {
    pub summary: AnalyticsSummary,
    pub daily_trend: Vec<DailyTrendPoint>,
    pub focus_hour_distribution: Vec<FocusHourBucket>,
}

// ---------------------------------------------------------------------------
// Paths & schema
// ---------------------------------------------------------------------------

fn data_dir() -> PathBuf {
    let base = std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| Path::new(".").to_path_buf());

    let path = base.join(".focusmust");
    if let Err(error) = fs::create_dir_all(&path) {
        eprintln!("Failed to create data dir at {}: {error}", path.display());
    }

    path
}

fn db_path() -> PathBuf {
    data_dir().join(DB_FILE)
}

fn initialize_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_type TEXT NOT NULL,
            started_at INTEGER NOT NULL,
            ended_at INTEGER NOT NULL,
            duration_secs INTEGER NOT NULL,
            task TEXT,
            whitelist_json TEXT NOT NULL DEFAULT '[]'
        );
        CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at DESC);
        CREATE INDEX IF NOT EXISTS idx_sessions_type_started_at ON sessions(session_type, started_at DESC);
        ",
    )
}

// ---------------------------------------------------------------------------
// JSONL migration (runs once, then renames the file)
// ---------------------------------------------------------------------------

fn migrate_jsonl_if_needed(conn: &Connection) {
    let jsonl_path = data_dir().join(SESSIONS_FILE);
    if !jsonl_path.exists() {
        return;
    }

    let migrated_path = data_dir().join(SESSIONS_FILE_MIGRATED);
    if migrated_path.exists() {
        // Already migrated in a previous run but JSONL was re-created as fallback
        // Try to import any new records that appeared since last migration
    }

    let file = match fs::File::open(&jsonl_path) {
        Ok(file) => file,
        Err(_) => return,
    };

    use std::io::{BufRead, BufReader};
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    for line in reader.lines().map_while(Result::ok) {
        if let Ok(record) = serde_json::from_str::<SessionRecord>(&line) {
            records.push(record);
        }
    }

    if records.is_empty() {
        // Nothing to migrate — rename the empty file
        let _ = fs::rename(&jsonl_path, &migrated_path);
        return;
    }

    // Insert records that don't already exist (dedup by started_at + session_type)
    if let Ok(tx) = conn.unchecked_transaction() {
        {
            let mut stmt = match tx.prepare(
                "
            INSERT INTO sessions (
                session_type,
                started_at,
                ended_at,
                duration_secs,
                task,
                whitelist_json
            )
            SELECT ?1, ?2, ?3, ?4, ?5, ?6
            WHERE NOT EXISTS (
                SELECT 1 FROM sessions
                WHERE session_type = ?1 AND started_at = ?2
            )
            ",
            ) {
                Ok(stmt) => stmt,
                Err(error) => {
                    eprintln!("Failed to prepare migration statement: {error}");
                    return;
                }
            };

            for record in records {
                let whitelist_json =
                    serde_json::to_string(&record.whitelist).unwrap_or_else(|_| "[]".to_string());
                if let Err(error) = stmt.execute(params![
                    record.session_type,
                    record.started_at as i64,
                    record.ended_at as i64,
                    record.duration_secs as i64,
                    record.task,
                    whitelist_json,
                ]) {
                    eprintln!("Failed to migrate a legacy session record: {error}");
                }
            }
        }

        if let Err(error) = tx.commit() {
            eprintln!("Failed to commit migration transaction: {error}");
            return;
        }
    }

    // Rename JSONL file so it's not re-processed, but kept for recovery
    let _ = fs::rename(&jsonl_path, &migrated_path);
}

// ---------------------------------------------------------------------------
// JSONL fallback (only used when SQLite unavailable)
// ---------------------------------------------------------------------------

fn append_session_jsonl(record: &SessionRecord) {
    let path = data_dir().join(SESSIONS_FILE);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        if let Ok(json) = serde_json::to_string(record) {
            if let Err(error) = writeln!(file, "{}", json) {
                eprintln!("Failed to append session JSONL: {error}");
            }
        }
    }
}

fn load_sessions_from_jsonl() -> Vec<SessionRecord> {
    let path = data_dir().join(SESSIONS_FILE);
    let mut sessions = Vec::new();
    if path.exists() {
        if let Ok(file) = fs::File::open(path) {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(file);
            for line in reader.lines().map_while(Result::ok) {
                if let Ok(record) = serde_json::from_str::<SessionRecord>(&line) {
                    sessions.push(record);
                }
            }
        }
    }
    sessions.reverse();
    sessions
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn load_settings() -> UserSettings {
    let path = data_dir().join(SETTINGS_FILE);
    if path.exists() {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(settings) = serde_json::from_str(&content) {
                return settings;
            }
        }
    }

    let settings = UserSettings::default();
    save_settings(&settings);
    settings
}

pub fn save_settings(settings: &UserSettings) {
    let path = data_dir().join(SETTINGS_FILE);
    if let Ok(json) = serde_json::to_string_pretty(settings) {
        if let Err(error) = fs::write(&path, json) {
            eprintln!("Failed to save settings at {}: {error}", path.display());
        }
    }
}

pub fn append_session(record: &SessionRecord) {
    let inserted = with_db(|conn| {
        let whitelist_json =
            serde_json::to_string(&record.whitelist).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "
            INSERT INTO sessions (
                session_type,
                started_at,
                ended_at,
                duration_secs,
                task,
                whitelist_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
            params![
                record.session_type,
                record.started_at as i64,
                record.ended_at as i64,
                record.duration_secs as i64,
                record.task,
                whitelist_json,
            ],
        )
        .ok()
    });

    if inserted.is_none() {
        append_session_jsonl(record);
    }
}

pub fn load_sessions() -> Vec<SessionRecord> {
    with_db(|conn| {
        let mut stmt = conn
            .prepare(
                "
            SELECT
                session_type,
                started_at,
                ended_at,
                duration_secs,
                task,
                whitelist_json
            FROM sessions
            ORDER BY started_at DESC
            ",
            )
            .ok()?;

        let rows = stmt
            .query_map([], |row| {
                let whitelist_json: String = row.get(5)?;
                let whitelist =
                    serde_json::from_str::<Vec<String>>(&whitelist_json).unwrap_or_else(|_| vec![]);

                Ok(SessionRecord {
                    session_type: row.get(0)?,
                    started_at: row.get::<_, i64>(1)? as u64,
                    ended_at: row.get::<_, i64>(2)? as u64,
                    duration_secs: row.get::<_, i64>(3)? as u64,
                    task: row.get(4)?,
                    whitelist,
                })
            })
            .ok()?;

        Some(rows.filter_map(Result::ok).collect())
    })
    .unwrap_or_else(load_sessions_from_jsonl)
}

pub fn load_sessions_page(offset: u64, limit: u64) -> HistoryPage {
    let limit = limit.clamp(1, 500) as usize;
    let offset = offset.min(i64::MAX as u64) as i64;

    with_db(|conn| {
        let mut stmt = conn
            .prepare(
                "
            SELECT
                session_type,
                started_at,
                ended_at,
                duration_secs,
                task,
                whitelist_json
            FROM sessions
            ORDER BY started_at DESC
            LIMIT ?1 OFFSET ?2
            ",
            )
            .ok()?;

        let rows = stmt
            .query_map(params![(limit + 1) as i64, offset], |row| {
                let whitelist_json: String = row.get(5)?;
                let whitelist =
                    serde_json::from_str::<Vec<String>>(&whitelist_json).unwrap_or_else(|_| vec![]);

                Ok(SessionRecord {
                    session_type: row.get(0)?,
                    started_at: row.get::<_, i64>(1)? as u64,
                    ended_at: row.get::<_, i64>(2)? as u64,
                    duration_secs: row.get::<_, i64>(3)? as u64,
                    task: row.get(4)?,
                    whitelist,
                })
            })
            .ok()?;

        let mut items: Vec<SessionRecord> = rows.filter_map(Result::ok).collect();
        let has_more = items.len() > limit;
        if has_more {
            items.truncate(limit);
        }

        Some(HistoryPage { items, has_more })
    })
    .unwrap_or_else(|| {
        let sessions = load_sessions_from_jsonl();
        let start = offset as usize;
        if start >= sessions.len() {
            return HistoryPage::default();
        }

        let end = (start + limit).min(sessions.len());
        HistoryPage {
            items: sessions[start..end].to_vec(),
            has_more: end < sessions.len(),
        }
    })
}

pub fn load_analytics() -> AnalyticsData {
    with_db(|conn| {
        let summary = conn
            .query_row(
                "
            SELECT
                COUNT(*) AS total_sessions,
                COALESCE(SUM(CASE WHEN session_type = 'focus' THEN 1 ELSE 0 END), 0) AS focus_sessions,
                COALESCE(SUM(CASE WHEN session_type = 'break' THEN 1 ELSE 0 END), 0) AS break_sessions,
                COALESCE(SUM(CASE WHEN session_type = 'focus' THEN duration_secs ELSE 0 END), 0) AS total_focus_secs,
                COALESCE(SUM(CASE WHEN session_type = 'break' THEN duration_secs ELSE 0 END), 0) AS total_break_secs
            FROM sessions
            ",
                [],
                |row| {
                    Ok(AnalyticsSummary {
                        total_sessions: row.get::<_, i64>(0)? as u64,
                        focus_sessions: row.get::<_, i64>(1)? as u64,
                        break_sessions: row.get::<_, i64>(2)? as u64,
                        total_focus_secs: row.get::<_, i64>(3)? as u64,
                        total_break_secs: row.get::<_, i64>(4)? as u64,
                    })
                },
            )
            .unwrap_or_default();

        let mut daily_trend = Vec::new();
        if let Ok(mut stmt) = conn.prepare(
            "
            SELECT
                strftime('%Y-%m-%d', started_at, 'unixepoch', 'localtime') AS day,
                COALESCE(SUM(CASE WHEN session_type = 'focus' THEN duration_secs ELSE 0 END), 0) AS focus_secs,
                COALESCE(SUM(CASE WHEN session_type = 'break' THEN duration_secs ELSE 0 END), 0) AS break_secs
            FROM sessions
            WHERE started_at >= strftime('%s', 'now', '-29 days')
            GROUP BY day
            ORDER BY day ASC
            ",
        ) {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok(DailyTrendPoint {
                    day: row.get(0)?,
                    focus_secs: row.get::<_, i64>(1)? as u64,
                    break_secs: row.get::<_, i64>(2)? as u64,
                })
            }) {
                daily_trend = rows.filter_map(Result::ok).collect();
            }
        }

        let mut by_hour: Vec<FocusHourBucket> = (0_u8..24)
            .map(|hour| FocusHourBucket {
                hour,
                focus_secs: 0,
                sessions: 0,
            })
            .collect();

        if let Ok(mut stmt) = conn.prepare(
            "
            SELECT
                CAST(strftime('%H', started_at, 'unixepoch', 'localtime') AS INTEGER) AS hour,
                COUNT(*) AS sessions,
                COALESCE(SUM(duration_secs), 0) AS focus_secs
            FROM sessions
            WHERE session_type = 'focus'
            GROUP BY hour
            ORDER BY hour ASC
            ",
        ) {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)? as usize,
                    row.get::<_, i64>(1)? as u64,
                    row.get::<_, i64>(2)? as u64,
                ))
            }) {
                for row in rows.filter_map(Result::ok) {
                    if row.0 < 24 {
                        by_hour[row.0].sessions = row.1;
                        by_hour[row.0].focus_secs = row.2;
                    }
                }
            }
        }

        Some(AnalyticsData {
            summary,
            daily_trend,
            focus_hour_distribution: by_hour,
        })
    })
    .unwrap_or_default()
}
