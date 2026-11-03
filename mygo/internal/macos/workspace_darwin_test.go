//go:build darwin

package macos

import (
	"testing"
)

// These run on macOS only, in CI's macOS runner: they read the real
// NSWorkspace through purego.

func TestRunningAppsHaveBundleIDs(t *testing.T) {
	apps := RunningApps()
	if len(apps) == 0 {
		t.Skip("no regular apps run (a headless session)")
	}
	for _, app := range apps {
		if app.BundleID == "" || app.PID <= 0 {
			t.Errorf("app %+v", app)
		}
	}
}

func TestFrontmost(t *testing.T) {
	app, ok := Frontmost()
	if !ok {
		t.Skip("no app in front (a headless session)")
	}
	if app.PID <= 0 {
		t.Errorf("front app %+v", app)
	}
}

func TestAppPath(t *testing.T) {
	if got := AppPath("com.apple.finder"); got == "" {
		t.Error("no path for Finder")
	}
	if got := AppPath("com.example.no-such-app"); got != "" {
		t.Errorf("path %q for a missing app", got)
	}
}

func TestStringRoundTrip(t *testing.T) {
	withPool(func() {
		for _, s := range []string{"", "Focus Must", "专注 ✓"} {
			if got := goString(nsString(s)); got != s {
				t.Errorf("round trip of %q gave %q", s, got)
			}
		}
	})
}
