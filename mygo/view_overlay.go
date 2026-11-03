package main

import (
	"time"

	"github.com/egoist/mygo/ui"

	"github.com/cretu/focus-must/mygo/internal/core"
	"github.com/cretu/focus-must/mygo/internal/icons"
)

// overlayView covers each display other than the main window's: the timer
// of the session or the break, or a hint to plan on the main display,
// over a rotating quote.
func (a *focusApp) overlayView(c *ui.Context) {
	pal := &darkPalette // the overlays are always the warm charcoal
	t := *ui.DarkTheme()
	t.Text, t.TextMuted, t.Accent = pal.Text, pal.TextMuted, pal.Primary
	c.SetTheme(&t)
	c.Root().Background(ui.Transparent)
	c.After(time.Second)

	q := a.quotes.current(a.locale())
	ui.Column(c).Fill().Center().Gap(16).Padding(32).
		RadialGradient(ui.RadialGradient{Stops: []ui.GradientStop{
			{Color: ui.Hex("#262624").Alpha(0.92), At: 0},
			{Color: ui.Hex("#1a1918").Alpha(0.96), At: 1},
		}}).
		Children(func() {
			now := a.unix()
			switch a.state.Phase() {
			case core.Focusing, core.Paused:
				ui.Icon(c, icons.Brain).FontSize(72).TextColor(pal.Primary)
				ui.Text(c, a.t("app.keepFocus")).Font(serif).FontSize(32).FontWeight(600).TextColor(pal.TextHighlighted)
				ui.Text(c, core.Clock(a.state.Elapsed(now))).FontSize(72).FontWeight(300).FontFeatures("tnum").TextColor(pal.TextHighlighted)
				if task := a.state.Task; task != "" {
					ui.Text(c, task).FontSize(18).TextColor(pal.TextMuted).TextAlign(ui.Center).MaxWidth(720)
				}
			case core.OnBreak:
				ui.Icon(c, icons.Coffee).FontSize(72).TextColor(pal.Success)
				ui.Text(c, a.t("overlay.breakTitle")).Font(serif).FontSize(32).FontWeight(600).TextColor(pal.TextHighlighted)
				ui.Text(c, core.Clock(a.state.BreakRemaining(now))).FontSize(72).FontWeight(300).FontFeatures("tnum").TextColor(pal.Success)
			default:
				ui.Icon(c, icons.ShieldCheck).FontSize(72).TextColor(pal.Primary)
				ui.Text(c, "Focus Must").Font(serif).FontSize(32).FontWeight(600).TextColor(pal.TextHighlighted)
				ui.Text(c, a.t("overlay.planningHint")).FontSize(16).TextColor(pal.TextMuted)
			}

			quoteBox := ui.Column(c.Key(q.text)).Gap(6).AlignItems(ui.Center).MaxWidth(640).Margin(48, 0, 0, 0)
			quoteBox.Opacity(quoteBox.Animate("fade-in", 1, 600*time.Millisecond))
			quoteBox.Children(func() {
				ui.Text(c, q.text).Font(serif).FontSize(18).Italic().TextAlign(ui.Center).TextColor(pal.Text)
				ui.Text(c, q.author).FontSize(13).TextColor(pal.TextMuted)
			})
		})
}
