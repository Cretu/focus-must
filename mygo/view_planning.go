package main

import (
	"math"
	"strings"
	"time"

	"github.com/egoist/mygo/ui"

	"github.com/cretu/focus-must/mygo/internal/core"
	"github.com/cretu/focus-must/mygo/internal/icons"
)

// planningView is the planner: the next task, the apps it needs, breaks,
// and the history beside them.
func (a *focusApp) planningView(c *ui.Context, pal *palette, w, h float32) {
	panel := card(c, pal).Width(min(1040, w*0.92)).Height(h * 0.7)
	// ⌘↵ starts focusing from anywhere in the planner. On the card, it
	// comes before the text area and the tiles, which keep Enter.
	if panel.Shortcut(ui.Cmd, ui.KeyEnter) || c.Shortcut(ui.Cmd, ui.KeyEnter) {
		a.startFocus()
	}
	panel.Children(func() {
		// Header.
		ui.Row(c).Padding(16, 20).Gap(12).AlignItems(ui.Start).BorderWidth(0, 0, 1, 0).BorderColor(pal.Border).Children(func() {
			ui.Row(c).Grow(1).Gap(10).AlignItems(ui.Center).Children(func() {
				brandMark(c, 28, pal.Primary)
				heading(c, pal, a.t("app.planningTitle"), 20).SingleLine()
			})
			ui.Row(c).Gap(8).Wrap().Children(func() {
				snowIcon, snowLabel := icons.MoonStar, a.t("app.snow")
				if a.snow.enabled {
					snowIcon, snowLabel = icons.Snowflake, a.t("app.snowing")
				}
				if iconButton(c, pal, snowIcon, snowLabel).Clicked() {
					a.setSnow(!a.snow.enabled)
				}
				if iconButton(c, pal, icons.Settings, a.t("app.settings")).Clicked() {
					a.openSettings()
				}
				if iconButton(c, pal, icons.ChartColumn, a.t("app.analytics")).Clicked() {
					a.openAnalytics()
				}
				a.appearanceSelect(c)
			})
		})

		// Body: the plan on the left, the history on the right.
		ui.Row(c).Grow(1).Padding(16, 20).Gap(16).AlignItems(ui.Stretch).Children(func() {
			ui.Column(c).Grow(1).Basis(0).Gap(12).Children(func() {
				ui.Scroll(c).Grow(1).Gap(14).Padding(0, 4, 0, 0).Children(func() {
					a.taskSection(c, pal)
					softCard(c, pal).Children(func() {
						a.appsHeader(c, pal, a.appsLoading, a.loadApps)
						a.appGrid(c.Key("planning-apps"), pal, a.running, a.selected, a.appsLoading)
					})
				})
				a.actions(c, pal)
			})
			ui.Column(c).Width(340).Border(1, pal.Border).Radius(10).Padding(14).Children(func() {
				a.historyView(c, pal)
			})
		})
	})
}

// appearanceSelect chooses the light or dark appearance, or the system's.
func (a *focusApp) appearanceSelect(c *ui.Context) {
	options := []string{a.t("appearance.system"), a.t("appearance.light"), a.t("appearance.dark")}
	values := []string{"system", "light", "dark"}
	chosen := options[0]
	for i, v := range values {
		if v == a.settings.Appearance {
			chosen = options[i]
		}
	}
	if ui.Select(c.Key("appearance"), &chosen, options).Width(120).Label(a.t("app.defaultAppearance")).Changed() {
		for i, o := range options {
			if o == chosen {
				a.setAppearance(values[i])
			}
		}
	}
}

func (a *focusApp) taskSection(c *ui.Context, pal *palette) {
	softCard(c, pal).Children(func() {
		sectionTitle(c, pal, icons.ClipboardList, a.t("app.nextTask"), "")
		input := ui.TextArea(c.Key("task"), &a.task).Placeholder(a.t("app.taskPlaceholder")).
			Label(a.t("app.taskPlaceholder")).MinHeight(72)
		if input.Changed() && strings.TrimSpace(a.task) != "" {
			a.taskInvalid = false
		}
		if a.taskInvalid {
			input.Border(1, pal.Error)
			// A short shake, as the web app's, when starting without a task.
			if a.taskShake {
				a.taskShake, a.taskShakeAt = false, c.Now()
			}
			if since := c.Now().Sub(a.taskShakeAt); since >= 0 && since < 500*time.Millisecond {
				k := float64(since) / float64(500*time.Millisecond)
				input.Left(float32(math.Sin(k*math.Pi*6) * (1 - k) * 6))
				c.AnimationFrame()
			}
			ui.Row(c).Gap(4).AlignItems(ui.Center).Children(func() {
				ui.Icon(c, icons.CircleAlert).FontSize(13).TextColor(pal.Error)
				ui.Text(c, a.t("app.taskRequired")).FontSize(12).TextColor(pal.Error)
			})
		}

		recent := a.recentTasks()
		if len(recent) == 0 {
			return
		}
		ui.Row(c).Gap(8).AlignItems(ui.Center).Children(func() {
			ui.Text(c, a.t("app.recentTasks")).FontSize(12).TextColor(pal.TextMuted).Shrink(0)
			ui.Row(c).Gap(6).Wrap().Grow(1).Children(func() {
				shown := recent[:min(3, len(recent))]
				for _, task := range shown {
					if iconButton(c.Key("recent:"+task), pal, nil, task).MaxWidth(220).Padding(2, 10).FontSize(12).Clicked() {
						a.applyRecentTask(task)
					}
				}
				if hidden := recent[len(shown):]; len(hidden) > 0 {
					ui.MenuButton(c, a.t("app.moreCount", "count", len(hidden)), func(m *ui.Menu) {
						for _, task := range hidden {
							if m.Item(task).Chosen() {
								a.applyRecentTask(task)
							}
						}
					}).FontSize(12)
				}
			})
		})
	})
}

// appsHeader is the "apps needed" section's header with its refresh button.
func (a *focusApp) appsHeader(c *ui.Context, pal *palette, loading bool, refresh func()) {
	ui.Row(c).Gap(8).AlignItems(ui.Center).Children(func() {
		sectionTitle(c, pal, icons.LayoutGrid, a.t("app.appsNeeded"), a.t("app.appsNeededHint")).Grow(1)
		if iconButton(c, pal, icons.RotateCcw, a.t("app.refresh")).Disabled(loading).Padding(2, 10).FontSize(12).Clicked() {
			refresh()
		}
	})
}

// actions are the buttons under the plan: breaks, and starting to focus.
func (a *focusApp) actions(c *ui.Context, pal *palette) {
	ui.Column(c).Gap(8).Shrink(0).Children(func() {
		switch {
		case a.state.Phase() == core.OnBreak:
			iconButton(c, pal, icons.Coffee, a.t("app.breakInProgress", "time", core.Clock(a.state.BreakRemaining(a.unix())))).
				Disabled(true).FillWidth()
		case !a.showBreakOpts:
			if iconButton(c, pal, icons.Coffee, a.t("app.takeBreakFree")).FillWidth().Clicked() {
				a.showBreakOpts = true
			}
		default:
			ui.Row(c).Gap(8).Children(func() {
				for _, m := range breakPresets {
					if iconButton(c.Key(m), pal, nil, a.t("app.minutesShort", "minutes", m)).Grow(1).Clicked() {
						a.startBreak(m)
					}
				}
				ui.Row(c).Grow(1.5).Gap(0).Children(func() {
					if ui.TextInput(c.Key("custom-break"), &a.customMinutes).Placeholder(a.t("app.custom")).
						Label(a.t("app.custom")).Grow(1).MinWidth(60).Submitted() {
						a.startCustomBreak()
					}
					if iconButton(c, pal, icons.ArrowRight, "").Label(a.t("app.startBreak")).
						Tooltip(a.t("app.startBreak")).Clicked() {
						a.startCustomBreak()
					}
				})
			})
		}
		primaryIconButton(c, icons.Rocket, a.t("app.startFocus")).FillWidth().Height(42).FontSize(15).OnClick(a.startFocus)
		ui.Row(c).Gap(4).Justify(ui.Center).AlignItems(ui.Center).Children(func() {
			for _, k := range []string{"⌘", "↵"} {
				ui.Text(c, k).FontSize(11).TextColor(pal.TextMuted).Padding(0, 5).Radius(4).
					Border(1, pal.Border).Background(pal.BgElevated)
			}
			ui.Text(c, a.t("app.shortcutStart")).FontSize(11).TextColor(pal.TextMuted)
		})
	})
}
