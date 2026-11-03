package main

import (
	"fmt"
	"strconv"

	"github.com/egoist/mygo/ui"

	"github.com/cretu/focus-must/mygo/internal/i18n"
	"github.com/cretu/focus-must/mygo/internal/icons"
)

func (a *focusApp) settingsView(c *ui.Context, pal *palette, w, h float32) {
	card(c, pal).Width(min(980, w*0.92)).MaxHeight(h * 0.88).Children(func() {
		ui.Row(c).Padding(16, 20).Gap(8).AlignItems(ui.Center).BorderWidth(0, 0, 1, 0).BorderColor(pal.Border).Children(func() {
			ui.Icon(c, icons.Settings).FontSize(26).TextColor(pal.Primary)
			heading(c, pal, a.t("app.settingsTitle"), 20)
		})

		ui.Scroll(c.Key("settings")).Grow(1).Shrink(1).Padding(16, 20).Gap(14).Children(func() {
			// Default whitelist.
			softCard(c, pal).Children(func() {
				ui.Row(c).Gap(8).AlignItems(ui.Center).Children(func() {
					sectionTitle(c, pal, icons.LayoutGrid, a.t("app.defaultAllowedApps"), a.t("app.defaultAllowedAppsSubtitle")).Grow(1)
					if iconButton(c, pal, icons.RotateCcw, a.t("app.refresh")).Padding(2, 10).FontSize(12).Clicked() {
						a.refreshSettingsApps()
					}
				})
				a.appGrid(c.Key("settings-apps"), pal, a.settingsApps, a.settingsWhitelist, false)
			})

			// Start at login.
			softCard(c, pal).Children(func() {
				settingRow(c, pal, icons.Power, a.t("app.autostart"), a.t("app.autostartSubtitle"), func() {
					ui.Switch(c, &a.autostart).Label(a.t("app.autostart"))
				})
			})

			// Break reminder.
			softCard(c, pal).Children(func() {
				settingRow(c, pal, icons.Bell, a.t("app.breakReminder"), a.t("app.breakReminderSubtitle"), func() {
					if ui.Switch(c, &a.reminderOn).Label(a.t("app.breakReminder")).Changed() {
						if a.reminderOn {
							a.setBreakReminder(defaultReminderMinute)
							a.reminderChoice = a.reminderLabel(defaultReminderMinute)
						} else {
							a.setBreakReminder(0)
						}
					}
				})
				if a.reminderOn {
					ui.Row(c).Gap(12).AlignItems(ui.Center).Children(func() {
						ui.Text(c, a.t("app.breakReminderEvery")).FontSize(12).TextColor(pal.TextMuted).Grow(1)
						labels := make([]string, len(breakReminderOptions))
						for i, m := range breakReminderOptions {
							labels[i] = a.reminderLabel(m)
						}
						if ui.Select(c.Key("reminder"), &a.reminderChoice, labels).Width(128).
							Label(a.t("app.breakReminderEvery")).Changed() {
							for i, l := range labels {
								if l == a.reminderChoice {
									a.setBreakReminder(breakReminderOptions[i])
								}
							}
						}
					})
				}
			})

			// Language and appearance.
			softCard(c, pal).Children(func() {
				settingRow(c, pal, icons.Languages, a.t("app.defaultLanguage"), "", func() {
					values := []string{i18n.System, i18n.ZhCN, i18n.EnUS}
					labels := make([]string, len(values))
					chosen := ""
					for i, v := range values {
						labels[i] = a.t("languageOption." + v)
						if v == a.settingsLocale {
							chosen = labels[i]
						}
					}
					if ui.Select(c.Key("locale"), &chosen, labels).Width(150).Label(a.t("app.defaultLanguage")).Changed() {
						for i, l := range labels {
							if l == chosen {
								a.settingsLocale = values[i]
							}
						}
					}
				})
				settingRow(c, pal, icons.SunMoon, a.t("app.defaultAppearance"), "", func() {
					values := []string{"system", "light", "dark"}
					labels := make([]string, len(values))
					chosen := ""
					for i, v := range values {
						labels[i] = a.t("appearance." + v)
						if v == a.settingsAppearance {
							chosen = labels[i]
						}
					}
					if ui.Select(c.Key("settings-appearance"), &chosen, labels).Width(150).Label(a.t("app.defaultAppearance")).Changed() {
						for i, l := range labels {
							if l == chosen {
								a.settingsAppearance = values[i]
							}
						}
					}
				})
			})

			a.selfCheckSection(c, pal)
		})

		ui.Row(c).Padding(14, 20).Gap(8).BorderWidth(1, 0, 0, 0).BorderColor(pal.Border).Children(func() {
			if iconButton(c, pal, icons.ArrowLeft, a.t("app.back")).Grow(1).Clicked() {
				a.view = viewPlanning
			}
			primaryIconButton(c, icons.Save, a.t("app.saveSettings")).Grow(1).OnClick(a.saveSettings)
		})
	})
}

// settingRow is a setting's label, with a description under it, beside its
// control.
func settingRow(c *ui.Context, pal *palette, icon *ui.SVG, title, subtitle string, control func()) {
	ui.Row(c).Gap(12).AlignItems(ui.Center).Children(func() {
		ui.Column(c).Grow(1).Gap(2).Children(func() {
			sectionTitle(c, pal, icon, title, "")
			if subtitle != "" {
				ui.Text(c, subtitle).FontSize(12).TextColor(pal.TextMuted)
			}
		})
		control()
	})
}

func (a *focusApp) selfCheckSection(c *ui.Context, pal *palette) {
	softCard(c, pal).Children(func() {
		ui.Row(c).Gap(8).AlignItems(ui.Center).Children(func() {
			sectionTitle(c, pal, icons.Stethoscope, a.t("app.selfCheck"), a.t("app.selfCheckSubtitle")).Grow(1)
			if iconButton(c, pal, icons.RefreshCw, a.t("app.recheck")).Padding(2, 10).FontSize(12).Clicked() {
				a.selfCheck = a.sys.selfCheck()
			}
		})
		check := a.selfCheck
		row := func(icon *ui.SVG, label string, value func()) {
			ui.Row(c).Gap(12).AlignItems(ui.Center).Children(func() {
				ui.Row(c).Grow(1).Gap(6).AlignItems(ui.Center).Children(func() {
					ui.Icon(c, icon).FontSize(15).TextColor(pal.TextMuted)
					ui.Text(c, label).FontSize(13).TextColor(pal.TextMuted)
				})
				value()
			})
		}
		row(icons.Eye, a.t("app.checkMonitoring"), func() {
			if check.LastFrontmost != "" {
				pill(c, a.t("app.checkOk")+" · "+check.LastFrontmost, pal.Success, pal.SuccessSoft)
			} else {
				pill(c, a.t("app.checkMonitoringWait"), pal.Warning, pal.WarningSoft)
			}
		})
		row(icons.Monitor, a.t("app.checkDisplays"), func() {
			pill(c, a.t("app.displayCount", "count", len(check.Displays)), pal.Text, pal.BgAccented)
		})
		for i, d := range check.Displays {
			ui.Row(c.Key(i)).Gap(8).Padding(4, 8).Radius(6).Background(pal.BgElevated.Alpha(0.5)).AlignItems(ui.Center).Children(func() {
				name := d.Name
				if name == "" {
					name = "#" + strconv.Itoa(i+1)
				}
				ui.Text(c, name).FontSize(12).TextColor(pal.TextMuted).SingleLine().Grow(1)
				ui.Text(c, fmt.Sprintf("%d×%d", d.Width, d.Height)).FontSize(12).TextColor(pal.TextMuted)
				if d.Primary {
					pill(c, a.t("app.primaryDisplay"), pal.Primary, pal.PrimarySoft)
				}
			})
		}
		row(icons.Tag, a.t("app.checkVersion"), func() {
			pill(c, "v"+check.Version, pal.Text, pal.BgAccented)
		})
		ui.Row(c).Gap(8).Wrap().Children(func() {
			if iconButton(c, pal, icons.Volume2, a.t("app.testSound")).Clicked() {
				a.sys.playSound("Glass")
			}
			if iconButton(c, pal, icons.Bell, a.t("app.testNotification")).Clicked() {
				a.testNotification()
			}
		})
	})
}
