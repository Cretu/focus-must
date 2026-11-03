package core

import (
	"slices"
	"testing"
)

func TestAllowedApps(t *testing.T) {
	s := NewState()
	s.StartFocus([]string{"com.apple.Safari"}, "write", 1000)

	for _, id := range []string{"com.apple.Safari", "com.apple.finder", SelfBundleID} {
		if s.ShouldBlock(id) {
			t.Errorf("ShouldBlock(%q) during focus, want allowed", id)
		}
	}
	if !s.ShouldBlock("com.tinyspeck.slackmacgap") {
		t.Error("an app off the whitelist is not blocked")
	}
	if s.ShouldBlock("") {
		t.Error("an app without a bundle id is blocked")
	}

	s.Pause(1100)
	if s.ShouldBlock("com.tinyspeck.slackmacgap") {
		t.Error("a paused session blocks")
	}
	s.Resume(nil, 1200)
	s.EndFocus(1300)
	if s.ShouldBlock("com.tinyspeck.slackmacgap") {
		t.Error("blocks without a session")
	}

	s.StartBreak(5, 2000)
	if s.ShouldBlock("com.tinyspeck.slackmacgap") {
		t.Error("blocks during a break")
	}
}

func TestPhases(t *testing.T) {
	s := NewState()
	if s.Phase() != Planning {
		t.Fatalf("phase at launch %v", s.Phase())
	}
	s.StartFocus(nil, "task", 10)
	if s.Phase() != Focusing {
		t.Fatalf("phase after start %v", s.Phase())
	}
	if !s.Pause(20) || s.Phase() != Paused {
		t.Fatalf("phase after pause %v", s.Phase())
	}
	if s.Pause(25) {
		t.Error("paused twice")
	}
	if !s.Resume(nil, 30) || s.Phase() != Focusing {
		t.Fatalf("phase after resume %v", s.Phase())
	}
	s.StartBreak(10, 40)
	if s.Phase() != OnBreak {
		t.Fatalf("phase after break %v", s.Phase())
	}
	if s.BreakRemaining(100) != 540 {
		t.Errorf("break remaining %d, want 540", s.BreakRemaining(100))
	}
}

func TestPausedTimeIsNotFocusTime(t *testing.T) {
	s := NewState()
	s.StartFocus([]string{"a"}, "task", 1000)
	s.Pause(1060) // 60 s focused
	if got := s.Elapsed(1500); got != 60 {
		t.Errorf("elapsed while paused %d, want 60", got)
	}
	s.Resume([]string{"a", "b"}, 1360) // paused 300 s
	if !slices.Equal(s.SessionWhitelist, []string{"a", "b"}) {
		t.Errorf("whitelist after resume %v", s.SessionWhitelist)
	}
	if got := s.Elapsed(1400); got != 100 {
		t.Errorf("elapsed after resume %d, want 100", got)
	}
	rec := s.EndFocus(1400)
	if rec == nil || rec.Duration != 100 || rec.Task != "task" || rec.Type != "focus" {
		t.Fatalf("record %+v", rec)
	}
	if s.Task != "" || s.SessionWhitelist != nil || s.FocusStartedAt != 0 {
		t.Errorf("state after end %+v", s)
	}
}

func TestShortSessionsAreNotRecorded(t *testing.T) {
	s := NewState()
	s.StartFocus(nil, "task", 100)
	if rec := s.EndFocus(105); rec != nil {
		t.Errorf("recorded a 5 s session: %+v", rec)
	}
	s.StartBreak(5, 200)
	if rec := s.StartFocus(nil, "next", 300); rec == nil || rec.Type != "break" || rec.Duration != 100 {
		t.Errorf("break ended by a focus session recorded %+v", rec)
	}
	if s.BreakEndAt != 0 {
		t.Error("break still runs during focus")
	}
}

func TestReminder(t *testing.T) {
	s := NewState()
	s.BreakReminderMinutes = 1
	var r Reminder
	s.StartFocus(nil, "task", 1000)
	if _, due := r.Due(s, 1030); due {
		t.Fatal("due after 30 s")
	}
	if m, due := r.Due(s, 1060); !due || m != 1 {
		t.Fatalf("at 60 s: %d %v", m, due)
	}
	if _, due := r.Due(s, 1061); due {
		t.Fatal("due twice")
	}
	if m, due := r.Due(s, 1120); !due || m != 2 {
		t.Fatalf("at 120 s: %d %v", m, due)
	}
	// Resuming moves the start; the reminders already given stay given.
	s.Pause(1130)
	if _, due := r.Due(s, 1200); due {
		t.Fatal("due while paused")
	}
	s.Resume(nil, 1200)
	if _, due := r.Due(s, 1201); due {
		t.Fatal("due right after resuming")
	}
	if m, due := r.Due(s, 1250); !due || m != 3 {
		t.Fatalf("at 180 s focused: %d %v", m, due)
	}
}

func TestClock(t *testing.T) {
	for secs, want := range map[int64]string{0: "00:00", 65: "01:05", 3600: "01:00:00", 3725: "01:02:05", -3: "00:00"} {
		if got := Clock(secs); got != want {
			t.Errorf("Clock(%d) = %q, want %q", secs, got, want)
		}
	}
}
