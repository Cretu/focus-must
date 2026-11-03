package storage

import (
	"os"
	"path/filepath"
	"slices"
	"testing"
	"time"

	"github.com/egoist/mygo/plugins/sqlite"

	"github.com/cretu/focus-must/mygo/internal/core"
)

func needSQLite(t *testing.T) {
	t.Helper()
	if err := sqlite.Load(); err != nil {
		t.Skipf("SQLite library unavailable: %v", err)
	}
}

func TestSettingsRoundTrip(t *testing.T) {
	dir := t.TempDir()
	s := &Store{dir: dir}

	got := s.LoadSettings()
	if !slices.Equal(got.DefaultWhitelist, core.DefaultWhitelist) || got.Locale != "system" || got.Appearance != "system" {
		t.Fatalf("defaults %+v", got)
	}
	if _, err := os.Stat(filepath.Join(dir, settingsFile)); err != nil {
		t.Fatal("defaults were not written")
	}

	got.Locale, got.BreakReminderMinutes, got.Snow = "en-US", 45, true
	s.SaveSettings(got)
	if again := s.LoadSettings(); again.Locale != "en-US" || again.BreakReminderMinutes != 45 || !again.Snow {
		t.Errorf("after saving %+v", again)
	}
}

// The Tauri app's settings have no appearance or snow fields.
func TestReadsTauriSettings(t *testing.T) {
	dir := t.TempDir()
	tauri := `{"default_whitelist":["com.apple.Safari"],"locale":"zh-CN","break_reminder_minutes":25}`
	if err := os.WriteFile(filepath.Join(dir, settingsFile), []byte(tauri), 0o644); err != nil {
		t.Fatal(err)
	}
	got := (&Store{dir: dir}).LoadSettings()
	if !slices.Equal(got.DefaultWhitelist, []string{"com.apple.Safari"}) || got.Locale != "zh-CN" ||
		got.BreakReminderMinutes != 25 || got.Appearance != "system" || got.Snow {
		t.Errorf("read %+v", got)
	}
}

func TestJSONLFallback(t *testing.T) {
	s := &Store{dir: t.TempDir()} // no database
	for i := range int64(3) {
		s.Append(core.Record{Type: "focus", StartedAt: 100 + i, EndedAt: 200 + i, Duration: 100, Task: "t"})
	}
	page := s.History(0, 2)
	if len(page.Items) != 2 || !page.HasMore || page.Items[0].StartedAt != 102 {
		t.Fatalf("first page %+v", page)
	}
	page = s.History(2, 2)
	if len(page.Items) != 1 || page.HasMore || page.Items[0].StartedAt != 100 {
		t.Fatalf("second page %+v", page)
	}
}

func TestHistoryAndAnalytics(t *testing.T) {
	needSQLite(t)
	dir := t.TempDir()

	// A session left in JSONL by an earlier run is imported once.
	legacy := &Store{dir: dir}
	now := time.Now().Unix()
	legacy.Append(core.Record{Type: "break", StartedAt: now - 7200, EndedAt: now - 6900, Duration: 300})

	s := Open(dir)
	defer s.Close()
	if !s.HasDatabase() {
		t.Fatal("no database")
	}
	if _, err := os.Stat(filepath.Join(dir, sessionsMigrated)); err != nil {
		t.Error("JSONL was not renamed after migrating")
	}

	s.Append(core.Record{Type: "focus", StartedAt: now - 3600, EndedAt: now - 1800, Duration: 1800, Task: "write", Whitelist: []string{"a"}})
	s.Append(core.Record{Type: "focus", StartedAt: now - 600, EndedAt: now, Duration: 600})

	page := s.History(0, 2)
	if len(page.Items) != 2 || !page.HasMore {
		t.Fatalf("page %+v", page)
	}
	if first := page.Items[0]; first.StartedAt != now-600 || first.Task != "" || first.Whitelist == nil {
		t.Errorf("newest %+v", first)
	}
	if second := page.Items[1]; second.Task != "write" || !slices.Equal(second.Whitelist, []string{"a"}) {
		t.Errorf("second %+v", second)
	}
	if rest := s.History(2, 10); len(rest.Items) != 1 || rest.HasMore || rest.Items[0].Type != "break" {
		t.Errorf("rest %+v", rest)
	}

	a, err := s.Analytics()
	if err != nil {
		t.Fatal(err)
	}
	want := Summary{TotalFocusSecs: 2400, TotalBreakSecs: 300, TotalSessions: 3, FocusSessions: 2, BreakSessions: 1}
	if a.Summary != want {
		t.Errorf("summary %+v, want %+v", a.Summary, want)
	}
	var focus, sessions int64
	for _, h := range a.Hours {
		focus += h.FocusSecs
		sessions += h.Sessions
	}
	if focus != 2400 || sessions != 2 {
		t.Errorf("hours total %d s over %d sessions", focus, sessions)
	}
	var daily int64
	for _, d := range a.Daily {
		daily += d.FocusSecs + d.BreakSecs
	}
	if daily != 2700 {
		t.Errorf("daily total %d", daily)
	}

	// Reopening does not import the JSONL again.
	s.Close()
	s = Open(dir)
	defer s.Close()
	if a, _ := s.Analytics(); a.Summary.TotalSessions != 3 {
		t.Errorf("%d sessions after reopening", a.Summary.TotalSessions)
	}
}
