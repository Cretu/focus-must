package main

import (
	"github.com/egoist/mygo/ui"

	"github.com/cretu/focus-must/mygo/internal/brand"
)

// The design tokens of the web app (src/styles/main.css): a terracotta
// accent ("clay", around #d97757) over warm stone neutrals, ivory surfaces
// in the light and warm charcoal ones in the dark.
var (
	clay400 = ui.Hex("#e29a74")
	clay500 = ui.Hex("#d97757")
	clay600 = ui.Hex("#c65d3f")
)

// palette is what the views color with, beyond ui.Theme.
type palette struct {
	Bg, BgMuted, BgElevated, BgAccented ui.Color
	Border, BorderMuted                 ui.Color
	Text, TextHighlighted, TextMuted    ui.Color
	Primary, PrimarySoft                ui.Color
	Success, SuccessSoft                ui.Color
	Warning, WarningSoft                ui.Color
	Error                               ui.Color
	Scrim                               ui.Color // the tint over the desktop
	Vignette                            ui.Color
	Snow                                ui.Color
}

var (
	lightPalette = palette{
		Bg:              ui.Hex("#faf9f5"),
		BgMuted:         ui.Hex("#f5f3ec"),
		BgElevated:      ui.Hex("#f0eee6"),
		BgAccented:      ui.Hex("#e8e5da"),
		Border:          ui.Hex("#e7e5e4"),
		BorderMuted:     ui.Hex("#f0eee6"),
		Text:            ui.Hex("#44403c"),
		TextHighlighted: ui.Hex("#1c1917"),
		TextMuted:       ui.Hex("#78716c"),
		Primary:         clay500,
		PrimarySoft:     clay500.Alpha(0.12),
		Success:         ui.Hex("#16a34a"),
		SuccessSoft:     ui.Hex("#16a34a").Alpha(0.12),
		Warning:         ui.Hex("#d97706"),
		WarningSoft:     ui.Hex("#d97706").Alpha(0.12),
		Error:           ui.Hex("#dc2626"),
		Scrim:           ui.RGBA(245, 243, 236, 0.2),
		Vignette:        ui.RGBA(120, 113, 108, 0.12),
		Snow:            ui.RGB(168, 162, 158),
	}
	darkPalette = palette{
		Bg:              ui.Hex("#262624"),
		BgMuted:         ui.Hex("#21201e"),
		BgElevated:      ui.Hex("#30302e"),
		BgAccented:      ui.Hex("#3a3937"),
		Border:          ui.Hex("#3a3937"),
		BorderMuted:     ui.Hex("#343330"),
		Text:            ui.Hex("#e7e5e4"),
		TextHighlighted: ui.Hex("#fafaf9"),
		TextMuted:       ui.Hex("#a8a29e"),
		Primary:         clay400,
		PrimarySoft:     clay400.Alpha(0.16),
		Success:         ui.Hex("#4ade80"),
		SuccessSoft:     ui.Hex("#4ade80").Alpha(0.14),
		Warning:         ui.Hex("#fbbf24"),
		WarningSoft:     ui.Hex("#fbbf24").Alpha(0.14),
		Error:           ui.Hex("#f87171"),
		Scrim:           ui.RGBA(30, 29, 27, 0.2),
		Vignette:        ui.RGBA(20, 19, 17, 0.24),
		Snow:            ui.RGB(255, 255, 255),
	}
)

// serif is the headings' family: the editorial serif of the web app.
const serif = "New York, Iowan Old Style, Georgia, Songti SC, serif"

// applyTheme sets the window's theme from the system's light or dark
// appearance (or the one the user chose, through mygo.Theme.SetSource) and
// returns the palette to color with.
func applyTheme(c *ui.Context) *palette {
	base := c.Theme()
	pal := &lightPalette
	if base.Dark {
		pal = &darkPalette
	}
	t := *base
	t.Background = pal.Bg
	t.Surface = pal.Bg
	t.SurfaceHover = pal.BgElevated
	t.SurfacePressed = pal.BgAccented
	t.Border = pal.Border
	t.Text = pal.Text
	t.TextMuted = pal.TextMuted
	t.Accent = pal.Primary
	t.AccentHover = pal.Primary.Mix(clay600, 0.35)
	t.AccentPressed = clay600
	t.AccentText = ui.RGB(255, 255, 255)
	if base.Dark {
		t.AccentText = ui.Hex("#1c1917")
	}
	t.Danger = pal.Error
	t.Warning = pal.Warning
	t.Success = pal.Success
	t.Focus = pal.Primary.Alpha(0.6)
	t.Selection = pal.Primary.Alpha(0.25)
	t.Radius = 9 // --ui-radius: 0.55rem
	c.SetTheme(&t)
	return pal
}

// card is the web app's UCard: a rounded, bordered surface.
func card(c *ui.Context, pal *palette) ui.Element {
	return ui.Column(c).Background(pal.Bg).Border(1, pal.Border).Radius(14).
		Shadow(0, 20, 50, -12, ui.RGBA(0, 0, 0, 0.25)).Clip()
}

// softCard is UCard's soft variant, a section inside a card.
func softCard(c *ui.Context, pal *palette) ui.Element {
	return ui.Column(c).Background(pal.BgElevated.Alpha(0.5)).Radius(10).Padding(14).Gap(12)
}

// sectionTitle is a section's header: an icon and a muted, bold title,
// with an optional hint after it.
func sectionTitle(c *ui.Context, pal *palette, icon *ui.SVG, title, hint string) ui.Element {
	return ui.Row(c).Gap(6).Wrap().AlignItems(ui.Center).Children(func() {
		ui.Icon(c, icon).FontSize(15).TextColor(pal.TextMuted)
		ui.Text(c, title).FontSize(13).FontWeight(600).TextColor(pal.TextMuted)
		if hint != "" {
			ui.Textf(c, "(%s)", hint).FontSize(11).TextColor(pal.TextMuted)
		}
	})
}

// heading is a card's serif title.
func heading(c *ui.Context, pal *palette, s string, size float32) ui.Element {
	return ui.Text(c, s).Font(serif).FontSize(size).FontWeight(600).TextColor(pal.TextHighlighted)
}

// iconButton is a button with an icon before its label, in the outline
// style of the web app's neutral buttons.
func iconButton(c *ui.Context, pal *palette, icon *ui.SVG, label string) ui.Element {
	b := ui.Button(c, "").Label(label).Background(ui.Transparent).Border(1, pal.Border)
	b.Children(func() {
		ui.Row(c).Gap(6).AlignItems(ui.Center).Children(func() {
			if icon != nil {
				ui.Icon(c, icon).FontSize(15)
			}
			if label != "" {
				ui.Text(c, label).SingleLine()
			}
		})
	})
	if b.Hovered() {
		b.Background(pal.BgElevated)
	}
	return b
}

// primaryIconButton is a solid accent button with an icon.
func primaryIconButton(c *ui.Context, icon *ui.SVG, label string) ui.Element {
	return ui.PrimaryButton(c, "").Label(label).Children(func() {
		ui.Row(c).Gap(6).AlignItems(ui.Center).Justify(ui.Center).Children(func() {
			if icon != nil {
				ui.Icon(c, icon).FontSize(15)
			}
			ui.Text(c, label).SingleLine()
		})
	})
}

// pill is a soft badge (UBadge variant="soft").
func pill(c *ui.Context, text string, fg, bg ui.Color) ui.Element {
	return ui.Text(c, text).FontSize(11).FontWeight(500).TextColor(fg).
		Background(bg).Padding(2, 8).Radius(999).SingleLine()
}

// brandMark draws the starburst in the color, in a size×size box.
func brandMark(c *ui.Context, size float32, color ui.Color) ui.Element {
	return ui.Box(c).Size(size, size).Draw(func(p *ui.Painter, r ui.Rect) {
		scale := r.W / brand.ViewBox
		for _, poly := range brand.Polygons() {
			var path ui.Path
			for i, pt := range poly {
				x, y := r.X+float32(pt[0])*scale, r.Y+float32(pt[1])*scale
				if i == 0 {
					path.MoveTo(x, y)
				} else {
					path.LineTo(x, y)
				}
			}
			path.Close()
			p.FillPath(&path, color)
		}
	})
}
