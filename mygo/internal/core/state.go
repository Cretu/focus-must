// Package core holds Focus Must's session state and the rules that change
// it: starting, pausing, resuming and ending a focus session, breaks, and
// which apps a session allows. It has no UI and no platform code, so it is
// tested as plain Go.
//
// It mirrors the Tauri backend's AppState (src-tauri/src/state.rs) and the
// transitions of its commands (src-tauri/src/commands.rs).
package core

import "slices"

// MinSessionDuration is the shortest session, in seconds, worth recording.
const MinSessionDuration = 10

// SelfBundleID is the app's own bundle identifier.
const SelfBundleID = "com.focus-must"

// DefaultWhitelist are the apps always allowed, whatever the user chose.
var DefaultWhitelist = []string{
	"com.apple.finder",
	"com.apple.systempreferences",
	SelfBundleID,
}

// Phase is what the app is doing.
type Phase int

const (
	// Planning: no focus session and no break.
	Planning Phase = iota
	// Focusing: a focus session runs and blocks what it does not allow.
	Focusing
	// Paused: a focus session is paused; nothing is blocked.
	Paused
	// OnBreak: a timed break (free activity) runs.
	OnBreak
)

// State is the session state, owned by the main thread.
type State struct {
	IsRestricted     bool
	DefaultWhitelist []string
	SessionWhitelist []string
	Task             string
	// FocusStartedAt is the start of the focus session (unix seconds), 0
	// without one. Resuming moves it forward by the time spent paused.
	FocusStartedAt int64
	BreakStartedAt int64
	// BreakEndAt is when the break ends (unix seconds), 0 without one.
	BreakEndAt int64
	// Locale is "system", "zh-CN" or "en-US".
	Locale string
	// BreakReminderMinutes repeats a break reminder that often during a
	// focus session; 0 turns it off.
	BreakReminderMinutes int64
	Paused               bool
	PausedAt             int64
	// Session counts the focus sessions started, to tell them apart.
	Session int64
}

// NewState returns the state at launch.
func NewState() *State {
	return &State{
		IsRestricted:     true,
		DefaultWhitelist: slices.Clone(DefaultWhitelist),
		Locale:           "system",
	}
}

// Phase reports what the app is doing.
func (s *State) Phase() Phase {
	switch {
	case s.FocusStartedAt != 0 && s.Paused:
		return Paused
	case s.FocusStartedAt != 0:
		return Focusing
	case s.BreakEndAt != 0:
		return OnBreak
	}
	return Planning
}

// IsAppAllowed reports whether the app may be in front.
func (s *State) IsAppAllowed(bundleID string) bool {
	if s.Paused || s.BreakEndAt != 0 || !s.IsRestricted {
		return true
	}
	return slices.Contains(s.DefaultWhitelist, bundleID) || slices.Contains(s.SessionWhitelist, bundleID)
}

// ShouldBlock reports whether the app in front should be hidden: a focus
// session runs, unpaused, and does not allow it.
func (s *State) ShouldBlock(bundleID string) bool {
	if bundleID == "" || s.FocusStartedAt == 0 || !s.IsRestricted || s.BreakEndAt != 0 {
		return false
	}
	return !s.IsAppAllowed(bundleID)
}

// Elapsed is how long the focus session has run, in seconds, without the
// time it was paused.
func (s *State) Elapsed(now int64) int64 {
	if s.FocusStartedAt == 0 {
		return 0
	}
	if s.Paused && s.PausedAt != 0 {
		return max(0, s.PausedAt-s.FocusStartedAt)
	}
	return max(0, now-s.FocusStartedAt)
}

// BreakRemaining is how long the break has left, in seconds.
func (s *State) BreakRemaining(now int64) int64 {
	if s.BreakEndAt == 0 {
		return 0
	}
	return max(0, s.BreakEndAt-now)
}

// Record is a finished session, as the history keeps it.
type Record struct {
	Type      string // "focus" or "break"
	StartedAt int64
	EndedAt   int64
	Duration  int64
	Task      string
	Whitelist []string
}

// StartFocus starts a focus session for the task, allowing the apps of the
// whitelist. A break that runs ends, and is returned to be recorded.
func (s *State) StartFocus(whitelist []string, task string, now int64) *Record {
	rec := s.endBreak(now)
	s.Session++
	s.SessionWhitelist = slices.Clone(whitelist)
	s.Task = task
	s.FocusStartedAt = now
	s.BreakStartedAt, s.BreakEndAt = 0, 0
	s.Paused, s.PausedAt = false, 0
	return rec
}

// Pause pauses the focus session. It reports false when there is none to
// pause.
func (s *State) Pause(now int64) bool {
	if s.FocusStartedAt == 0 || s.Paused {
		return false
	}
	s.Paused, s.PausedAt = true, now
	return true
}

// Resume resumes a paused session with the whitelist, when not nil, and
// leaves the time paused out of its duration.
func (s *State) Resume(whitelist []string, now int64) bool {
	if !s.Paused {
		return false
	}
	if whitelist != nil {
		s.SessionWhitelist = slices.Clone(whitelist)
	}
	if s.FocusStartedAt != 0 && s.PausedAt != 0 {
		s.FocusStartedAt += max(0, now-s.PausedAt)
	}
	s.Paused, s.PausedAt = false, 0
	return true
}

// EndFocus ends the focus session and returns its record, nil when it was
// too short to keep or there was none.
func (s *State) EndFocus(now int64) *Record {
	var rec *Record
	if s.FocusStartedAt != 0 {
		end := now
		if s.Paused && s.PausedAt != 0 {
			// Time spent paused is not focus time.
			end = s.PausedAt
		}
		if d := end - s.FocusStartedAt; d >= MinSessionDuration {
			rec = &Record{
				Type:      "focus",
				StartedAt: s.FocusStartedAt,
				EndedAt:   now,
				Duration:  d,
				Task:      s.Task,
				Whitelist: slices.Clone(s.SessionWhitelist),
			}
		}
	}
	s.SessionWhitelist = nil
	s.Task = ""
	s.IsRestricted = true
	s.FocusStartedAt = 0
	s.Paused, s.PausedAt = false, 0
	return rec
}

// StartBreak starts a break of the given minutes, ending a focus session.
func (s *State) StartBreak(minutes int64, now int64) {
	s.BreakStartedAt = now
	s.BreakEndAt = now + minutes*60
	s.FocusStartedAt = 0
	s.Paused, s.PausedAt = false, 0
}

// EndBreak ends the break and returns its record, nil when it was too
// short to keep or there was none.
func (s *State) EndBreak(now int64) *Record { return s.endBreak(now) }

func (s *State) endBreak(now int64) *Record {
	var rec *Record
	if s.BreakStartedAt != 0 && s.BreakEndAt != 0 {
		if d := now - s.BreakStartedAt; d >= MinSessionDuration {
			rec = &Record{Type: "break", StartedAt: s.BreakStartedAt, EndedAt: now, Duration: d}
		}
	}
	s.BreakStartedAt, s.BreakEndAt = 0, 0
	return rec
}

// Reminder fires the break reminder every BreakReminderMinutes of a focus
// session.
type Reminder struct {
	session int64
	fired   int64
}

// Due reports whether a reminder is due now and, when one is, the minutes
// focused so far. Call it once a second.
func (r *Reminder) Due(s *State, now int64) (int64, bool) {
	if s.FocusStartedAt == 0 || s.Paused {
		return 0, false
	}
	if r.session != s.Session {
		r.session, r.fired = s.Session, 0
	}
	if s.BreakReminderMinutes <= 0 {
		return 0, false
	}
	goal := s.BreakReminderMinutes * 60
	if s.Elapsed(now) >= goal*(r.fired+1) {
		r.fired++
		return s.BreakReminderMinutes * r.fired, true
	}
	return 0, false
}
