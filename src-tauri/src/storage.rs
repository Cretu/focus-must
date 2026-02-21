use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const SETTINGS_FILE: &str = "settings.json";
const SESSIONS_FILE: &str = "sessions.jsonl";
const DB_FILE: &str = "history.db";

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
            default_whitelist: vec![
                "com.apple.finder".to_string(),
                "com.apple.systempreferences".to_string(),
                "com.focus-must".to_string(),
            ],
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

fn open_db() -> Option<Connection> {
    let conn = Connection::open(db_path()).ok()?;
    initialize_schema(&conn).ok()?;
    migrate_jsonl_if_needed(&conn).ok()?;
    Some(conn)
}

fn migrate_jsonl_if_needed(conn: &Connection) -> rusqlite::Result<()> {
    let existing_rows: i64 =
        conn.query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;
    if existing_rows > 0 {
        return Ok(());
    }

    let path = data_dir().join(SESSIONS_FILE);
    if !path.exists() {
        return Ok(());
    }

    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return Ok(()),
    };

    use std::io::{BufRead, BufReader};
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    for line in reader.lines().map_while(Result::ok) {
        if let Ok(record) = serde_json::from_str::<SessionRecord>(&line) {
            records.push(record);
        }
    }

    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare(
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
        )?;

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
    tx.commit()?;
    Ok(())
}

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
    if let Some(conn) = open_db() {
        let whitelist_json =
            serde_json::to_string(&record.whitelist).unwrap_or_else(|_| "[]".to_string());
        let inserted = conn.execute(
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
        );

        if inserted.is_ok() {
            return;
        }
    }

    append_session_jsonl(record);
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

pub fn load_sessions() -> Vec<SessionRecord> {
    let Some(conn) = open_db() else {
        return load_sessions_from_jsonl();
    };

    let mut stmt = match conn.prepare(
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
    ) {
        Ok(stmt) => stmt,
        Err(_) => return load_sessions_from_jsonl(),
    };

    let rows = match stmt.query_map([], |row| {
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
    }) {
        Ok(rows) => rows,
        Err(_) => return load_sessions_from_jsonl(),
    };

    rows.filter_map(Result::ok).collect()
}

pub fn load_sessions_page(offset: u64, limit: u64) -> HistoryPage {
    let limit = limit.clamp(1, 500) as usize;
    let offset = offset.min(i64::MAX as u64) as i64;

    let Some(conn) = open_db() else {
        let sessions = load_sessions_from_jsonl();
        let start = offset as usize;
        if start >= sessions.len() {
            return HistoryPage::default();
        }

        let end = (start + limit).min(sessions.len());
        return HistoryPage {
            items: sessions[start..end].to_vec(),
            has_more: end < sessions.len(),
        };
    };

    let mut stmt = match conn.prepare(
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
    ) {
        Ok(stmt) => stmt,
        Err(_) => {
            let sessions = load_sessions_from_jsonl();
            let start = offset as usize;
            if start >= sessions.len() {
                return HistoryPage::default();
            }

            let end = (start + limit).min(sessions.len());
            return HistoryPage {
                items: sessions[start..end].to_vec(),
                has_more: end < sessions.len(),
            };
        }
    };

    let rows = match stmt.query_map(params![(limit + 1) as i64, offset], |row| {
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
    }) {
        Ok(rows) => rows,
        Err(_) => return HistoryPage::default(),
    };

    let mut items: Vec<SessionRecord> = rows.filter_map(Result::ok).collect();
    let has_more = items.len() > limit;
    if has_more {
        items.truncate(limit);
    }

    HistoryPage { items, has_more }
}

pub fn load_analytics() -> AnalyticsData {
    let Some(conn) = open_db() else {
        return AnalyticsData::default();
    };

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

    AnalyticsData {
        summary,
        daily_trend,
        focus_hour_distribution: by_hour,
    }
}
