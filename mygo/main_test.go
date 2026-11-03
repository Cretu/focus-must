package main

import (
	"image/png"
	"os"
	"path/filepath"
	"slices"
	"testing"
	"time"

	"github.com/egoist/mygo/ui"

	"github.com/cretu/focus-must/mygo/internal/core"
	"github.com/cretu/focus-must/mygo/internal/storage"
)

// fakeSystem stands in for the desktop: no windows, a clock the test
// moves, and a fixed list of running apps.
type fakeSystem struct {
	clock    time.Time
	apps     []appItem
	visible  bool
	notified []string
	sounds   []string
	login    bool
	theme    string
}

func (f *fakeSystem) showMain()              { f.visible = true }
func (f *fakeSystem) hideAll()               { f.visible = false }
func (f *fakeSystem) changed()               {}
func (f *fakeSystem) notify(_, body string)  { f.notified = append(f.notified, body) }
func (f *fakeSystem) playSound(name string)  { f.sounds = append(f.sounds, name) }
func (f *fakeSystem) runningApps() []appItem { return slices.Clone(f.apps) }
func (f *fakeSystem) appInfo(id string) (appItem, bool) {
	return appItem{Name: "Installed " + id, BundleID: id}, true
}
func (f *fakeSystem) loadIcons(apps []appItem, done func(map[string]*ui.Bitmap)) {}
func (f *fakeSystem) openAtLogin() bool                                          { return f.login }
func (f *fakeSystem) setOpenAtLogin(on bool) error                               { f.login = on; return nil }
func (f *fakeSystem) setAppearance(a string)                                     { f.theme = a }
func (f *fakeSystem) now() time.Time                                             { return f.clock }
func (f *fakeSystem) selfCheck() selfCheckReport {
	return selfCheckReport{LastFrontmost: "com.apple.Safari", Version: version,
		Displays: []displayInfo{{Name: "Built-in Retina Display", Width: 3024, Height: 1964, Primary: true}}}
}

func (f *fakeSystem) advance(d time.Duration) { f.clock = f.clock.Add(d) }

// newTestApp makes the app over a temporary data directory, in English.
func newTestApp(t *testing.T) (*focusApp, *fakeSystem, *ui.Tester) {
	t.Helper()
	dir := t.TempDir()
	if err := os.WriteFile(filepath.Join(dir, "settings.json"),
		[]byte(`{"default_whitelist":["com.apple.finder"],"locale":"en-US","break_reminder_minutes":0}`), 0o644); err != nil {
		t.Fatal(err)
	}
	store := storage.Open(dir)
	t.Cleanup(func() { store.Close() })
	sys := &fakeSystem{
		clock: time.Date(2026, 11, 3, 9, 30, 0, 0, time.Local),
		apps: []appItem{
			{Name: "Safari", BundleID: "com.apple.Safari"},
			{Name: "Visual Studio Code", BundleID: "com.microsoft.VSCode"},
			{Name: "Slack", BundleID: "com.tinyspeck.slackmacgap"},
		},
	}
	a := newFocusApp(sys, store, "en-US")
	tt := ui.NewTester(a.mainView, 1440, 900)
	return a, sys, tt
}

func click(t *testing.T, tt *ui.Tester, label string) {
	t.Helper()
	if err := tt.Click(label); err != nil {
		t.Fatalf("click %q: %v\ntexts: %q", label, err, tt.Texts())
	}
}

func want(t *testing.T, tt *ui.Tester, texts ...string) {
	t.Helper()
	for _, s := range texts {
		if !tt.HasText(s) {
			t.Errorf("no %q among %q", s, tt.Texts())
		}
	}
}

func TestFocusSessionFlow(t *testing.T) {
	a, sys, tt := newTestApp(t)
	want(t, tt, "Focus Must: Plan your next task before you start", "Safari", "Slack", "Start Focus")

	// Starting without a task asks for one.
	click(t, tt, "Start Focus")
	want(t, tt, "Write down your task before starting a focus session")
	if a.state.Phase() != core.Planning {
		t.Fatal("started without a task")
	}

	click(t, tt, "Describe the task you are about to complete...")
	tt.Type("Write the report")
	click(t, tt, "Safari")
	want(t, tt, "1 selected")
	tt.Key(ui.Cmd, ui.KeyEnter)
	if a.state.Phase() != core.Focusing {
		t.Fatalf("phase %v after ⌘↵", a.state.Phase())
	}
	if sys.visible {
		t.Error("the planner stays up during focus")
	}
	if !slices.Equal(a.state.SessionWhitelist, []string{"com.apple.Safari", "com.apple.finder"}) || a.state.Task != "Write the report" {
		t.Errorf("session %+v", a.state)
	}
	if !a.state.ShouldBlock("com.tinyspeck.slackmacgap") || a.state.ShouldBlock("com.apple.Safari") {
		t.Error("whitelist not applied")
	}

	// A look from the tray shows the timer.
	sys.advance(65 * time.Second)
	tt.Frame()
	want(t, tt, "Stay Focused", "01:05", "Write the report", "Safari")

	// Pause, allow Slack, resume.
	click(t, tt, "Pause Focus")
	if a.state.Phase() != core.Paused || !sys.visible {
		t.Fatalf("phase %v after pausing", a.state.Phase())
	}
	want(t, tt, "Paused")
	click(t, tt, "Slack")
	sys.advance(5 * time.Minute)
	click(t, tt, "Resume Focus")
	if a.state.Phase() != core.Focusing || a.state.ShouldBlock("com.tinyspeck.slackmacgap") {
		t.Fatalf("resume: phase %v, whitelist %v", a.state.Phase(), a.state.SessionWhitelist)
	}
	tt.Frame()
	want(t, tt, "01:05") // paused time does not count

	// End, confirming.
	sys.advance(time.Minute)
	click(t, tt, "End Focus")
	want(t, tt, "Are you sure you want to end this focus session?")
	click(t, tt, "Confirm")
	if a.state.Phase() != core.Planning || !sys.visible {
		t.Fatalf("phase %v after ending", a.state.Phase())
	}
	tt.Frame()
	want(t, tt, "Recent Records", "Today", "2m5s", "Write the report")
	if got := a.recentTasks(); !slices.Equal(got, []string{"Write the report"}) {
		t.Errorf("recent tasks %q", got)
	}
}

func TestBreak(t *testing.T) {
	a, sys, tt := newTestApp(t)
	click(t, tt, "Take a Break (Free Activity)")
	click(t, tt, "10m")
	if a.state.Phase() != core.OnBreak || sys.visible {
		t.Fatalf("phase %v", a.state.Phase())
	}
	sys.advance(4 * time.Minute)
	a.tick()
	tt.Frame()
	want(t, tt, "On Break 06:00")

	sys.advance(6 * time.Minute)
	a.tick()
	if a.state.Phase() != core.Planning || !sys.visible {
		t.Fatalf("break did not end: %v", a.state.Phase())
	}
	tt.Frame()
	want(t, tt, "10m")
}

func TestCustomBreak(t *testing.T) {
	a, _, tt := newTestApp(t)
	click(t, tt, "Take a Break (Free Activity)")
	click(t, tt, "Custom")
	tt.Type("999")
	tt.Key(0, ui.KeyEnter)
	if a.state.BreakRemaining(a.unix()) != maxBreakMinutes*60 {
		t.Errorf("custom break of %d s", a.state.BreakRemaining(a.unix()))
	}
}

func TestBreakReminder(t *testing.T) {
	a, sys, tt := newTestApp(t)
	a.setBreakReminder(25)
	a.task = "Deep work"
	a.startFocus()
	sys.advance(25 * time.Minute)
	a.tick()
	tt.Frame()
	if !sys.visible || len(sys.sounds) != 1 {
		t.Fatalf("reminder: visible %v, sounds %v", sys.visible, sys.sounds)
	}
	want(t, tt, "Time for a break", "You've focused for 25 minutes — stretch, move, grab some water.")
	click(t, tt, "Break 5 min")
	if a.state.Phase() != core.OnBreak {
		t.Fatalf("phase %v", a.state.Phase())
	}
	// The focus session before the break is kept.
	if page := a.store.History(0, 10); len(page.Items) != 1 || page.Items[0].Duration != 25*60 {
		t.Errorf("history %+v", page.Items)
	}
}

func TestSettings(t *testing.T) {
	a, sys, tt := newTestApp(t)
	click(t, tt, "Settings")
	// The whitelisted Finder does not run: it is listed anyway.
	want(t, tt, "Default Allowed Apps", "Installed com.apple.finder", "Safari", "OK · com.apple.Safari", "1 detected", "v"+version)

	click(t, tt, "Safari")
	a.autostart = true // the switch beside "Start at Login"
	click(t, tt, "Save Settings")
	if a.view != viewPlanning || !sys.login {
		t.Fatalf("view %v, login %v", a.view, sys.login)
	}
	if !slices.Equal(a.state.DefaultWhitelist, []string{"com.apple.Safari", "com.apple.finder"}) {
		t.Errorf("whitelist %v", a.state.DefaultWhitelist)
	}
	if !a.selected["com.apple.Safari"] {
		t.Error("the planner does not preselect the saved whitelist")
	}
	if got := a.store.LoadSettings(); !slices.Equal(got.DefaultWhitelist, a.state.DefaultWhitelist) {
		t.Errorf("saved %v", got.DefaultWhitelist)
	}
}

func TestLanguage(t *testing.T) {
	a, _, tt := newTestApp(t)
	a.openSettings()
	a.settingsLocale = "zh-CN"
	a.saveSettings()
	tt.Frame()
	want(t, tt, "Focus Must：先想清楚要做什么，再开始", "开始专注")
}

func TestAnalytics(t *testing.T) {
	a, sys, tt := newTestApp(t)
	now := sys.clock.Unix()
	a.store.Append(core.Record{Type: "focus", StartedAt: now - 7200, EndedAt: now - 3600, Duration: 3600, Task: "a"})
	a.store.Append(core.Record{Type: "break", StartedAt: now - 3000, EndedAt: now - 2700, Duration: 300})
	click(t, tt, "Analytics")
	if a.analyticsError {
		t.Skip("history database unavailable")
	}
	want(t, tt, "Total Focus Time", "1h 0m", "5m", "2", "Focus Trend (Last 30 Days)")
	click(t, tt, "Back")
	if a.view != viewPlanning {
		t.Error("Back did not return")
	}
}

func TestOverlay(t *testing.T) {
	a, sys, _ := newTestApp(t)
	tt := ui.NewTester(a.overlayView, 1920, 1080)
	want(t, tt, "Focus Must", "Plan your next task on the main screen")
	a.task = "Ship it"
	a.startFocus()
	sys.advance(90 * time.Second)
	tt.Frame()
	want(t, tt, "Stay Focused", "01:30", "Ship it")
	a.startBreak(15)
	tt.Frame()
	want(t, tt, "Taking a Break", "15:00")
}

func TestDistractionNotifierThrottles(t *testing.T) {
	var n distractionNotifier
	start := time.Now()
	if !n.shouldNotify("a", start) || n.shouldNotify("a", start.Add(5*time.Second)) {
		t.Fatal("same app notified twice within 20 s")
	}
	if !n.shouldNotify("b", start.Add(6*time.Second)) || !n.shouldNotify("a", start.Add(7*time.Second)) {
		t.Fatal("other apps throttled")
	}
	if n.shouldNotify("a", start.Add(8*time.Second)) || !n.shouldNotify("a", start.Add(30*time.Second)) {
		t.Fatal("throttle window wrong")
	}
}

// TestScreenshots writes the screens to PNG files when FOCUS_SCREENSHOTS
// names a directory, to look at them without a Mac.
func TestScreenshots(t *testing.T) {
	dir := os.Getenv("FOCUS_SCREENSHOTS")
	if dir == "" {
		t.Skip("FOCUS_SCREENSHOTS not set")
	}
	for _, dark := range []bool{false, true} {
		a, sys, tt := newTestApp(t)
		tt.SetDark(dark)
		suffix := "-light"
		if dark {
			suffix = "-dark"
		}
		now := sys.clock.Unix()
		for i := range int64(12) {
			a.store.Append(core.Record{Type: "focus", StartedAt: now - 86400*i - 3600, EndedAt: now - 86400*i, Duration: 1200 + i*300, Task: "Draft chapter " + formatInt(i)})
		}
		a.store.Append(core.Record{Type: "break", StartedAt: now - 1800, EndedAt: now - 1500, Duration: 300})
		a.reloadHistory()
		a.selected["com.apple.Safari"] = true
		shot := func(name string) {
			tt.Frame()
			f, err := os.Create(filepath.Join(dir, name+suffix+".png"))
			if err != nil {
				t.Fatal(err)
			}
			defer f.Close()
			if err := png.Encode(f, tt.Image()); err != nil {
				t.Fatal(err)
			}
		}
		shot("planning")
		a.openSettings()
		shot("settings")
		a.openAnalytics()
		shot("analytics")
		a.view = viewPlanning
		a.task = "Write the quarterly report"
		a.startFocus()
		sys.advance(25*time.Minute + 7*time.Second)
		shot("focus")
		a.pauseFocus()
		shot("paused")
		ot := ui.NewTester(a.overlayView, 1440, 900)
		ot.SetDark(dark)
		ot.Frame()
		f, _ := os.Create(filepath.Join(dir, "overlay"+suffix+".png"))
		png.Encode(f, ot.Image())
		f.Close()
	}
}

func formatInt(n int64) string { return itoa(n) }
