package main

import (
	"fmt"
	"strings"
	"time"

	"github.com/egoist/mygo/ui"

	"github.com/cretu/focus-must/mygo/internal/icons"
	"github.com/cretu/focus-must/mygo/internal/storage"
)

// historyView lists the recent sessions by day, filtered by type, loading
// older pages as it scrolls near its end.
func (a *focusApp) historyView(c *ui.Context, pal *palette) {
	ui.Text(c, a.t("history.title")).FontSize(13).FontWeight(600).TextColor(pal.TextMuted)
	ui.Segmented(c.Key("history-tab"), &a.historyTab, a.t("history.all"), a.t("history.focus"), a.t("history.break")).
		Label(a.t("history.title")).Margin(10, 0)

	filter := [...]string{"", "focus", "break"}[a.historyTab]
	var sessions []storage.Session
	for _, s := range a.history {
		if filter == "" || s.Type == filter {
			sessions = append(sessions, s)
		}
	}
	if len(sessions) == 0 {
		ui.Row(c).Gap(8).Padding(12).Radius(8).Background(pal.BgElevated).AlignItems(ui.Center).Children(func() {
			ui.Icon(c, icons.NotebookPen).FontSize(16).TextColor(pal.TextMuted)
			ui.Text(c, a.t("history.empty")).FontSize(13)
		})
		// A filter may hide what the loaded pages hold: load on.
		if a.historyHasMore {
			a.loadMoreHistory()
		}
		return
	}

	// Near the end, load the next page.
	if a.historyHasMore && a.historyScroll.MaxY-a.historyScroll.Y < 64 {
		a.loadMoreHistory()
	}
	ui.Scroll(c.Key("history")).TrackScroll(&a.historyScroll).Grow(1).Gap(12).Children(func() {
		now := a.sys.now()
		var day string
		var group []storage.Session
		flush := func() {
			if len(group) == 0 {
				return
			}
			a.historyDay(c.Key(day), pal, dayLabel(a, time.Unix(group[0].StartedAt, 0), now), group)
			group = nil
		}
		for _, s := range sessions {
			d := time.Unix(s.StartedAt, 0).Format(time.DateOnly)
			if d != day {
				flush()
				day = d
			}
			group = append(group, s)
		}
		flush()
		status := a.t("history.loadedAll")
		if a.historyHasMore {
			status = a.t("history.loading")
		}
		ui.Text(c, status).FontSize(11).TextColor(pal.TextMuted).TextAlign(ui.Center)
	})
}

func (a *focusApp) historyDay(c *ui.Context, pal *palette, label string, sessions []storage.Session) {
	ui.Column(c).Gap(8).Children(func() {
		ui.Row(c).Gap(8).AlignItems(ui.Center).Children(func() {
			ui.Text(c, label).FontSize(11).FontWeight(600).TextColor(pal.TextMuted)
			ui.Box(c).Height(1).Grow(1).Background(pal.Border)
		})
		for i, s := range sessions {
			ui.Column(c.Key(i)).Gap(4).Padding(8, 10).Radius(8).Background(pal.BgElevated.Alpha(0.5)).Children(func() {
				ui.Row(c).Gap(8).AlignItems(ui.Center).Children(func() {
					icon, fg, bg, label := icons.Target, pal.Primary, pal.PrimarySoft, a.t("history.focus")
					if s.Type != "focus" {
						icon, fg, bg, label = icons.Coffee, pal.TextMuted, pal.BgAccented, a.t("history.break")
					}
					ui.Box(c).Padding(3).Radius(6).Background(bg).Tooltip(label).Label(label).Children(func() {
						ui.Icon(c, icon).FontSize(12).TextColor(fg)
					})
					ui.Textf(c, "%s - %s", clockTime(s.StartedAt), clockTime(s.EndedAt)).
						FontSize(12).TextColor(pal.TextMuted).SingleLine().Grow(1)
					ui.Text(c, a.duration(s.Duration)).FontSize(12).FontWeight(500).SingleLine()
				})
				if s.Type == "focus" && s.Task != "" {
					ui.Text(c, s.Task).FontSize(13).SingleLine().Tooltip(s.Task)
				}
			})
		}
	})
}

func clockTime(unix int64) string { return time.Unix(unix, 0).Format("15:04:05") }

// dayLabel is "Today", "Yesterday" or the date.
func dayLabel(a *focusApp, d, now time.Time) string {
	y1, m1, d1 := d.Date()
	y2, m2, d2 := now.Date()
	days := int(time.Date(y2, m2, d2, 0, 0, 0, 0, time.Local).Sub(time.Date(y1, m1, d1, 0, 0, 0, 0, time.Local)).Hours()+12) / 24
	switch days {
	case 0:
		return a.t("history.today")
	case 1:
		return a.t("history.yesterday")
	}
	if a.locale() == "en-US" {
		return d.Format("1/2/2006")
	}
	return d.Format("2006/1/2")
}

// duration is a session's length, as "1h2m3s" or "1时2分3秒".
func (a *focusApp) duration(secs int64) string {
	h, m, s := secs/3600, secs%3600/60, secs%60
	var parts []string
	if h > 0 {
		parts = append(parts, a.t("duration.hours", "count", h))
	}
	if m > 0 {
		parts = append(parts, a.t("duration.minutes", "count", m))
	}
	if s > 0 || len(parts) == 0 {
		parts = append(parts, a.t("duration.seconds", "count", s))
	}
	return strings.Join(parts, "")
}

// durationLabel is a total, as "3h 20m".
func (a *focusApp) durationLabel(secs int64) string {
	h, m := secs/3600, secs%3600/60
	if h > 0 {
		return a.t("duration.hoursLabel", "hours", h, "minutes", m)
	}
	return a.t("duration.minutesLabel", "minutes", m)
}

// durationCompact is a short total, as "1h 5m" or "45m".
func (a *focusApp) durationCompact(secs int64) string {
	mins := secs / 60
	if mins >= 60 {
		rest := ""
		if mins%60 > 0 {
			rest = a.t("duration.minutesCompact", "minutes", mins%60)
		}
		return strings.TrimSpace(a.t("duration.hoursCompact", "hours", mins/60, "minutes", rest))
	}
	return a.t("duration.minutesCompact", "minutes", mins)
}

func hourRange(h int) string { return fmt.Sprintf("%02d:00~%02d:00", h%24, (h+1)%24) }
