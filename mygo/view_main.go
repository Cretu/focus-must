package main

import (
	"github.com/egoist/mygo/ui"

	"github.com/cretu/focus-must/mygo/internal/core"
)

// mainView is the main window: it covers the primary display, tints the
// desktop and centers the card of what the app is doing.
func (a *focusApp) mainView(c *ui.Context) {
	pal := applyTheme(c)
	c.Root().Background(ui.Transparent)
	phase := a.state.Phase()
	// A look at a running session from the tray leaves the desktop as it
	// is; the other screens tint it.
	peeking := phase == core.Focusing
	w, h := c.Size()

	backdrop := ui.Box(c).Fill().Center().Padding(16)
	if !peeking {
		backdrop.RadialGradient(ui.RadialGradient{
			Center: ui.Point{X: 0.16, Y: 0.12},
			Radius: max(w, h) * 0.44,
			Stops:  []ui.GradientStop{{Color: clay500.Alpha(0.1), At: 0}, {Color: pal.Scrim, At: 1}},
		})
	}
	backdrop.Children(func() {
		if !peeking {
			// The vignette that draws the eye to the card.
			ui.Box(c).Absolute().Top(0).Left(0).Right(0).Bottom(0).PassThrough().
				RadialGradient(ui.RadialGradient{
					Stops: []ui.GradientStop{{Color: ui.Transparent, At: 0.55}, {Color: pal.Vignette, At: 1}},
				})
		}
		switch {
		case phase == core.Paused:
			a.pausedView(c, pal, w, h)
		case phase == core.Focusing:
			a.focusView(c, pal, w)
		case a.view == viewSettings:
			a.settingsView(c, pal, w, h)
		case a.view == viewAnalytics:
			a.analyticsView(c, pal, w, h)
		default:
			a.planningView(c, pal, w, h)
		}
		if phase != core.Focusing {
			a.snow.view(c, pal)
		}
	})
	a.dialogs(c)
}

// dialogs are the confirmations and the break reminder, over any screen.
func (a *focusApp) dialogs(c *ui.Context) {
	if a.showEndConfirm {
		switch ui.AlertDialog(c, &a.showEndConfirm, a.t("app.confirmEndTitle"), a.t("app.confirmEndDescription"),
			a.t("app.cancel"), a.t("app.confirmEnd")) {
		case 1:
			a.endFocus()
		}
	}
	if a.showBreakReminder {
		open := true
		switch ui.AlertDialog(c, &open, a.t("app.breakReminderTitle"),
			a.t("app.breakReminderDesc", "minutes", a.reminderMinutes),
			a.t("app.continueFocus"), a.t("app.startBreakNow", "minutes", 5)) {
		case 0:
			a.continueFocus()
		case 1:
			a.startBreak(5)
		}
		// The reminder waits for an answer: Escape does not dismiss it.
	}
}
