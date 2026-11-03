package main

import (
	"time"

	"github.com/egoist/mygo/ui"

	"github.com/cretu/focus-must/mygo/internal/core"
	"github.com/cretu/focus-must/mygo/internal/icons"
)

// focusView is the card a running session shows when the user looks at
// it from the tray: the timer, the task and the apps allowed.
func (a *focusApp) focusView(c *ui.Context, pal *palette, w float32) {
	c.After(time.Second) // the timer's next second
	card(c, pal).Width(min(760, w*0.9)).Padding(28).Gap(20).AlignItems(ui.Center).Children(func() {
		brain := ui.Icon(c, icons.Brain).FontSize(60).TextColor(pal.Primary)
		k := 1 + 0.04*brain.Loop("breathe", 3*time.Second, ui.Bounce(ui.EaseInOut))
		brain.Scale(k, k)
		heading(c, pal, a.t("app.keepFocus"), 24)
		ui.Text(c, core.Clock(a.state.Elapsed(a.unix()))).FontSize(56).FontWeight(300).
			FontFeatures("tnum").TextColor(pal.TextHighlighted)

		task := a.currentTask()
		if task == "" {
			task = a.t("app.focusTaskFallback")
		}
		ui.Text(c, task).FontSize(14).TextAlign(ui.Center).FillWidth().Padding(12, 16).Radius(10).Background(pal.BgElevated)

		if len(a.allowedApps) > 0 {
			ui.Row(c).Gap(8).Wrap().Justify(ui.Center).Children(func() {
				for _, app := range a.allowedApps {
					pill(c.Key(app.BundleID), app.Name, pal.Primary, pal.PrimarySoft)
				}
			})
		}

		ui.Text(c, a.t("app.focusHint")).FontSize(13).TextColor(pal.TextMuted).TextAlign(ui.Center).
			FillWidth().Padding(12, 16).Radius(10).Border(1, pal.Border)

		ui.Row(c).Gap(8).Wrap().Justify(ui.Center).Children(func() {
			primaryIconButton(c, icons.ArrowRight, a.t("app.continueFocus")).OnClick(a.continueFocus)
			if iconButton(c, pal, icons.CirclePause, a.t("app.pauseFocus")).Clicked() {
				a.pauseFocus()
			}
			if iconButton(c, pal, icons.Square, a.t("app.endFocus")).Clicked() {
				a.showEndConfirm = true
			}
		})
	})
}

// pausedView lets the user change the apps a paused session allows before
// resuming it.
func (a *focusApp) pausedView(c *ui.Context, pal *palette, w, h float32) {
	card(c, pal).Width(min(820, w*0.92)).MaxHeight(h * 0.8).Children(func() {
		ui.Row(c).Padding(16, 20).Gap(8).AlignItems(ui.Center).BorderWidth(0, 0, 1, 0).BorderColor(pal.Border).Children(func() {
			ui.Icon(c, icons.CirclePause).FontSize(24).TextColor(pal.Primary)
			heading(c, pal, a.t("app.pausedTitle"), 20)
		})
		ui.Scroll(c).Grow(1).Shrink(1).Padding(16, 20).Gap(14).Children(func() {
			ui.Text(c, a.t("app.pausedHint")).FontSize(13).Padding(12, 16).Radius(10).Background(pal.BgElevated)
			softCard(c, pal).Children(func() {
				a.appsHeader(c, pal, a.appsLoading, a.loadApps)
				a.appGrid(c.Key("paused-apps"), pal, a.running, a.pausedSelected, a.appsLoading)
			})
		})
		ui.Row(c).Padding(14, 20).Gap(8).BorderWidth(1, 0, 0, 0).BorderColor(pal.Border).Children(func() {
			if iconButton(c, pal, icons.Square, a.t("app.endFocus")).Clicked() {
				a.endFocus()
			}
			primaryIconButton(c, icons.Play, a.t("app.resumeFocus")).Grow(1).OnClick(a.resumeFocus)
		})
	})
}
