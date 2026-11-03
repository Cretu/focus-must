package main

import (
	"strings"
	"time"
	"unicode/utf8"

	"github.com/egoist/mygo/ui"

	"github.com/cretu/focus-must/mygo/internal/icons"
)

const (
	// searchThreshold is how many apps a grid shows before it offers a
	// search.
	searchThreshold = 9
	tileWidth       = 96
)

// appGridState is a grid's search, kept per grid.
type appGridState struct{ query string }

// appGrid shows the apps as tiles to toggle in the set, with a search and
// the count of those chosen. Its key keeps its search apart from other
// grids'.
func (a *focusApp) appGrid(c *ui.Context, pal *palette, apps []appItem, set map[string]bool, loading bool) {
	col := ui.Column(c).Gap(10)
	st := ui.Local(col, "grid", func() appGridState { return appGridState{} })
	col.Children(func() {
		q := strings.ToLower(strings.TrimSpace(st.query))
		showSearch := len(apps) >= searchThreshold || q != ""
		count := 0
		for _, app := range apps {
			if set[app.BundleID] {
				count++
			}
		}
		if showSearch || count > 0 {
			ui.Row(c).Gap(8).AlignItems(ui.Center).Children(func() {
				if showSearch {
					ui.SearchField(c.Key("search"), &st.query).Placeholder(a.t("app.searchApps")).Grow(1)
				} else {
					ui.Spacer(c)
				}
				if count > 0 {
					pill(c, a.t("app.selectedCount", "count", count), pal.Primary, pal.PrimarySoft)
				}
			})
		}

		ui.Row(c).Gap(8).Wrap().AlignItems(ui.Start).Children(func() {
			if loading && len(apps) == 0 {
				for i := range 10 {
					sk := ui.Box(c.Key(i)).Size(tileWidth, 92).Radius(10).Background(pal.BgAccented)
					sk.Opacity(0.45 + 0.4*sk.Loop("pulse", 1600*time.Millisecond, ui.Bounce(ui.EaseInOut)))
				}
				return
			}
			shown := 0
			for _, app := range apps {
				if q != "" && !strings.Contains(strings.ToLower(app.Name), q) && !strings.Contains(strings.ToLower(app.BundleID), q) {
					continue
				}
				shown++
				a.appTile(c.Key(app.BundleID), pal, app, set)
			}
			switch {
			case len(apps) == 0:
				ui.Text(c, a.t("app.noRunningApps")).FontSize(13).TextColor(pal.TextMuted).
					Padding(12).Radius(8).Background(pal.BgElevated).FillWidth()
			case shown == 0:
				ui.Text(c, a.t("app.noMatchedApps")).FontSize(12).TextColor(pal.TextMuted).
					TextAlign(ui.Center).Padding(12).FillWidth()
			}
		})
	})
}

// appTile is an app's tile: its icon (or initial) and name, accented when
// chosen.
func (a *focusApp) appTile(c *ui.Context, pal *palette, app appItem, set map[string]bool) {
	selected := set[app.BundleID]
	tile := ui.Column(c).Size(tileWidth, 92).Padding(10, 6).Gap(6).AlignItems(ui.Center).Justify(ui.Center).
		Radius(10).Border(1, pal.Border).Cursor(ui.CursorPointer).Label(app.Name).Checked(selected)
	switch {
	case selected:
		tile.Background(pal.PrimarySoft).Border(1.5, pal.Primary)
	case tile.Hovered():
		tile.Background(pal.BgElevated)
	}
	if tile.Clicked() {
		toggle(set, app.BundleID)
	}
	tile.Children(func() {
		if icon := a.icons[app.BundleID]; icon != nil {
			ui.Image(c, icon).Size(40, 40)
		} else {
			initial := "?"
			if r, _ := utf8.DecodeRuneInString(app.Name); r != utf8.RuneError {
				initial = strings.ToUpper(string(r))
			}
			ui.Box(c).Size(40, 40).Center().Radius(10).Background(pal.PrimarySoft).Children(func() {
				ui.Text(c, initial).FontSize(18).FontWeight(600).TextColor(pal.Primary)
			})
		}
		size := float32(12)
		switch n := utf8.RuneCountInString(strings.TrimSpace(app.Name)); {
		case n >= 18:
			size = 10
		case n >= 12:
			size = 11
		}
		ui.Text(c, app.Name).FontSize(size).TextAlign(ui.Center).MaxLines(2)
		if selected {
			ui.Icon(c, icons.CircleCheck).FontSize(14).TextColor(pal.Primary).Attach(ui.AnchorTopRight, ui.AnchorTopRight).Top(4).Right(4)
		}
	})
}
