// Package storage keeps Focus Must's settings and session history in
// ~/.focusmust, in the same files as the Tauri app (settings.json and the
// SQLite database history.db), so that either app reads what the other
// wrote. When SQLite cannot be opened, sessions go to sessions.jsonl, as
// the Tauri app did, and are imported once SQLite works again.
package storage

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"sync"

	"github.com/egoist/mygo/plugins/sqlite"

	"github.com/cretu/focus-must/mygo/internal/core"
)

const (
	settingsFile       = "settings.json"
	sessionsFile       = "sessions.jsonl"
	sessionsMigrated   = "sessions.jsonl.migrated"
	dbFile             = "history.db"
	maxHistoryPageSize = 500
)

// Settings are the user's preferences. The first three fields are the
// Tauri app's; the others are new and that app ignores them.
type Settings struct {
	DefaultWhitelist     []string `json:"default_whitelist"`
	Locale               string   `json:"locale"`
	BreakReminderMinutes int64    `json:"break_reminder_minutes"`
	// Appearance is "system", "light" or "dark".
	Appearance string `json:"appearance,omitempty"`
	Snow       bool   `json:"snow,omitempty"`
}

// DefaultSettings are the settings of a new user.
func DefaultSettings() Settings {
	return Settings{
		DefaultWhitelist: slices.Clone(core.DefaultWhitelist),
		Locale:           "system",
		Appearance:       "system",
	}
}

// Session is a recorded session, as the history list shows it.
type Session struct {
	Type      string   `json:"session_type"`
	StartedAt int64    `json:"started_at"`
	EndedAt   int64    `json:"ended_at"`
	Duration  int64    `json:"duration_secs"`
	Task      string   `json:"task,omitempty"`
	Whitelist []string `json:"whitelist"`
}

// HistoryPage is a page of sessions, newest first.
type HistoryPage struct {
	Items   []Session
	HasMore bool
}

// Summary totals all sessions.
type Summary struct {
	TotalFocusSecs, TotalBreakSecs              int64
	TotalSessions, FocusSessions, BreakSessions int64
}

// DayPoint is a day's focus and break time.
type DayPoint struct {
	Day                  string // YYYY-MM-DD, local time
	FocusSecs, BreakSecs int64
}

// HourBucket is the focus time of sessions started in an hour of the day.
type HourBucket struct {
	Hour      int
	FocusSecs int64
	Sessions  int64
}

// Analytics are the statistics the analytics view shows.
type Analytics struct {
	Summary Summary
	// Daily covers the last 30 days, oldest first, days without sessions
	// left out.
	Daily []DayPoint
	// Hours has the 24 hours of the day.
	Hours [24]HourBucket
}

// Store reads and writes the data directory.
type Store struct {
	dir string

	mu sync.Mutex
	db *sqlite.DB // nil when SQLite is unavailable
}

// DefaultDir is ~/.focusmust.
func DefaultDir() string {
	home, err := os.UserHomeDir()
	if err != nil {
		home = "."
	}
	return filepath.Join(home, ".focusmust")
}

// Open opens the data directory, creating it when missing. It does not
// fail when SQLite is unavailable: history then goes to a JSONL file.
func Open(dir string) *Store {
	if err := os.MkdirAll(dir, 0o755); err != nil {
		log.Printf("Failed to create data dir at %s: %v", dir, err)
	}
	s := &Store{dir: dir}
	ctx := context.Background()
	db, err := sqlite.Open(ctx, filepath.Join(dir, dbFile), sqlite.OpenOptions{})
	if err != nil {
		log.Printf("Failed to open history database, falling back to JSONL: %v", err)
		return s
	}
	if err := initSchema(ctx, db); err != nil {
		log.Printf("Failed to initialize database schema, falling back to JSONL: %v", err)
		db.Close()
		return s
	}
	s.db = db
	s.migrateJSONL(ctx)
	return s
}

// Close closes the database.
func (s *Store) Close() error {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.db == nil {
		return nil
	}
	err := s.db.Close()
	s.db = nil
	return err
}

// HasDatabase reports whether history is kept in SQLite.
func (s *Store) HasDatabase() bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.db != nil
}

func initSchema(ctx context.Context, db *sqlite.DB) error {
	// The pragma returns a row: query it rather than execute it.
	if _, err := db.Query(ctx, "PRAGMA journal_mode = WAL"); err != nil {
		return err
	}
	for _, stmt := range []string{
		`CREATE TABLE IF NOT EXISTS sessions (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			session_type TEXT NOT NULL,
			started_at INTEGER NOT NULL,
			ended_at INTEGER NOT NULL,
			duration_secs INTEGER NOT NULL,
			task TEXT,
			whitelist_json TEXT NOT NULL DEFAULT '[]'
		)`,
		"CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at DESC)",
		"CREATE INDEX IF NOT EXISTS idx_sessions_type_started_at ON sessions(session_type, started_at DESC)",
	} {
		if _, err := db.Execute(ctx, stmt); err != nil {
			return err
		}
	}
	return nil
}

// ---------------------------------------------------------------------------
// Settings

// LoadSettings reads the settings, writing the defaults when there are
// none.
func (s *Store) LoadSettings() Settings {
	path := filepath.Join(s.dir, settingsFile)
	data, err := os.ReadFile(path)
	if err == nil {
		set := DefaultSettings()
		set.DefaultWhitelist = nil
		if err := json.Unmarshal(data, &set); err == nil {
			if set.Locale == "" {
				set.Locale = "system"
			}
			if set.Appearance == "" {
				set.Appearance = "system"
			}
			if set.DefaultWhitelist == nil {
				set.DefaultWhitelist = []string{}
			}
			return set
		}
		log.Printf("Failed to parse settings at %s: %v", path, err)
	}
	set := DefaultSettings()
	s.SaveSettings(set)
	return set
}

// SaveSettings writes the settings.
func (s *Store) SaveSettings(set Settings) {
	path := filepath.Join(s.dir, settingsFile)
	data, err := json.MarshalIndent(set, "", "  ")
	if err != nil {
		log.Printf("Failed to encode settings: %v", err)
		return
	}
	if err := os.WriteFile(path, data, 0o644); err != nil {
		log.Printf("Failed to save settings at %s: %v", path, err)
	}
}

// ---------------------------------------------------------------------------
// Sessions

// Append records a finished session.
func (s *Store) Append(rec core.Record) {
	sess := Session{
		Type:      rec.Type,
		StartedAt: rec.StartedAt,
		EndedAt:   rec.EndedAt,
		Duration:  rec.Duration,
		Task:      rec.Task,
		Whitelist: rec.Whitelist,
	}
	if sess.Whitelist == nil {
		sess.Whitelist = []string{}
	}
	s.mu.Lock()
	db := s.db
	s.mu.Unlock()
	if db != nil {
		wl, _ := json.Marshal(sess.Whitelist)
		_, err := db.Execute(context.Background(),
			`INSERT INTO sessions (session_type, started_at, ended_at, duration_secs, task, whitelist_json)
			 VALUES (?, ?, ?, ?, ?, ?)`,
			sess.Type, sess.StartedAt, sess.EndedAt, sess.Duration, nullable(sess.Task), string(wl))
		if err == nil {
			return
		}
		log.Printf("Failed to insert session, falling back to JSONL: %v", err)
	}
	s.appendJSONL(sess)
}

func nullable(s string) any {
	if s == "" {
		return nil
	}
	return s
}

// History returns a page of sessions, newest first.
func (s *Store) History(offset, limit int) HistoryPage {
	limit = min(max(limit, 1), maxHistoryPageSize)
	offset = max(offset, 0)
	s.mu.Lock()
	db := s.db
	s.mu.Unlock()
	if db != nil {
		rows, err := db.Query(context.Background(),
			`SELECT session_type, started_at, ended_at, duration_secs, task, whitelist_json
			 FROM sessions ORDER BY started_at DESC LIMIT ? OFFSET ?`, limit+1, offset)
		if err == nil {
			page := HistoryPage{Items: make([]Session, 0, len(rows.Values))}
			for _, r := range rows.Values {
				sess := Session{
					Type:      str(r[0]),
					StartedAt: num(r[1]),
					EndedAt:   num(r[2]),
					Duration:  num(r[3]),
					Task:      str(r[4]),
				}
				if err := json.Unmarshal([]byte(str(r[5])), &sess.Whitelist); err != nil {
					sess.Whitelist = []string{}
				}
				page.Items = append(page.Items, sess)
			}
			if len(page.Items) > limit {
				page.Items, page.HasMore = page.Items[:limit], true
			}
			return page
		}
		log.Printf("Failed to load history: %v", err)
	}
	sessions := s.loadJSONL()
	slices.Reverse(sessions)
	if offset >= len(sessions) {
		return HistoryPage{}
	}
	end := min(offset+limit, len(sessions))
	return HistoryPage{Items: sessions[offset:end], HasMore: end < len(sessions)}
}

// Analytics computes the statistics. They need SQLite: without it they
// are empty.
func (s *Store) Analytics() (Analytics, error) {
	var a Analytics
	for h := range a.Hours {
		a.Hours[h].Hour = h
	}
	s.mu.Lock()
	db := s.db
	s.mu.Unlock()
	if db == nil {
		return a, errors.New("history database unavailable")
	}
	ctx := context.Background()
	rows, err := db.Query(ctx, `
		SELECT
			COUNT(*),
			COALESCE(SUM(CASE WHEN session_type = 'focus' THEN 1 ELSE 0 END), 0),
			COALESCE(SUM(CASE WHEN session_type = 'break' THEN 1 ELSE 0 END), 0),
			COALESCE(SUM(CASE WHEN session_type = 'focus' THEN duration_secs ELSE 0 END), 0),
			COALESCE(SUM(CASE WHEN session_type = 'break' THEN duration_secs ELSE 0 END), 0)
		FROM sessions`)
	if err != nil {
		return a, fmt.Errorf("summary: %w", err)
	}
	if len(rows.Values) == 1 {
		r := rows.Values[0]
		a.Summary = Summary{
			TotalSessions:  num(r[0]),
			FocusSessions:  num(r[1]),
			BreakSessions:  num(r[2]),
			TotalFocusSecs: num(r[3]),
			TotalBreakSecs: num(r[4]),
		}
	}

	rows, err = db.Query(ctx, `
		SELECT
			strftime('%Y-%m-%d', started_at, 'unixepoch', 'localtime') AS day,
			COALESCE(SUM(CASE WHEN session_type = 'focus' THEN duration_secs ELSE 0 END), 0),
			COALESCE(SUM(CASE WHEN session_type = 'break' THEN duration_secs ELSE 0 END), 0)
		FROM sessions
		WHERE started_at >= strftime('%s', 'now', '-29 days')
		GROUP BY day
		ORDER BY day ASC`)
	if err != nil {
		return a, fmt.Errorf("daily trend: %w", err)
	}
	for _, r := range rows.Values {
		a.Daily = append(a.Daily, DayPoint{Day: str(r[0]), FocusSecs: num(r[1]), BreakSecs: num(r[2])})
	}

	rows, err = db.Query(ctx, `
		SELECT
			CAST(strftime('%H', started_at, 'unixepoch', 'localtime') AS INTEGER) AS hour,
			COUNT(*),
			COALESCE(SUM(duration_secs), 0)
		FROM sessions
		WHERE session_type = 'focus'
		GROUP BY hour
		ORDER BY hour ASC`)
	if err != nil {
		return a, fmt.Errorf("hour distribution: %w", err)
	}
	for _, r := range rows.Values {
		if h := num(r[0]); h >= 0 && h < 24 {
			a.Hours[h].Sessions = num(r[1])
			a.Hours[h].FocusSecs = num(r[2])
		}
	}
	return a, nil
}

func num(v any) int64 {
	switch v := v.(type) {
	case int64:
		return v
	case float64:
		return int64(v)
	}
	return 0
}

func str(v any) string {
	switch v := v.(type) {
	case string:
		return v
	case []byte:
		return string(v)
	}
	return ""
}

// ---------------------------------------------------------------------------
// JSONL fallback and its migration

func (s *Store) appendJSONL(sess Session) {
	f, err := os.OpenFile(filepath.Join(s.dir, sessionsFile), os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o644)
	if err != nil {
		log.Printf("Failed to open session JSONL: %v", err)
		return
	}
	defer f.Close()
	line, _ := json.Marshal(sess)
	if _, err := f.Write(append(line, '\n')); err != nil {
		log.Printf("Failed to append session JSONL: %v", err)
	}
}

// loadJSONL returns the sessions of the JSONL file, oldest first.
func (s *Store) loadJSONL() []Session {
	f, err := os.Open(filepath.Join(s.dir, sessionsFile))
	if err != nil {
		return nil
	}
	defer f.Close()
	var sessions []Session
	sc := bufio.NewScanner(f)
	sc.Buffer(make([]byte, 64*1024), 4*1024*1024)
	for sc.Scan() {
		line := strings.TrimSpace(sc.Text())
		if line == "" {
			continue
		}
		var sess Session
		if err := json.Unmarshal([]byte(line), &sess); err == nil {
			if sess.Whitelist == nil {
				sess.Whitelist = []string{}
			}
			sessions = append(sessions, sess)
		}
	}
	return sessions
}

// migrateJSONL imports the sessions of the JSONL file into SQLite, skipping
// those already there, then renames the file so it is kept but not read
// again.
func (s *Store) migrateJSONL(ctx context.Context) {
	path := filepath.Join(s.dir, sessionsFile)
	if _, err := os.Stat(path); err != nil {
		return
	}
	sessions := s.loadJSONL()
	if len(sessions) > 0 {
		stmts := make([]sqlite.Statement, 0, len(sessions))
		for _, sess := range sessions {
			wl, _ := json.Marshal(sess.Whitelist)
			stmts = append(stmts, sqlite.Statement{
				SQL: `INSERT INTO sessions (session_type, started_at, ended_at, duration_secs, task, whitelist_json)
					SELECT ?1, ?2, ?3, ?4, ?5, ?6
					WHERE NOT EXISTS (SELECT 1 FROM sessions WHERE session_type = ?1 AND started_at = ?2)`,
				Args: []any{sess.Type, sess.StartedAt, sess.EndedAt, sess.Duration, nullable(sess.Task), string(wl)},
			})
		}
		if _, err := s.db.Transaction(ctx, stmts); err != nil {
			log.Printf("Failed to migrate legacy session records: %v", err)
			return
		}
	}
	if err := os.Rename(path, filepath.Join(s.dir, sessionsMigrated)); err != nil {
		log.Printf("Failed to rename migrated session JSONL: %v", err)
	}
}
