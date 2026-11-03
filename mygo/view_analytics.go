package main

import (
	"strconv"

	"github.com/egoist/mygo/ui"

	"github.com/cretu/focus-must/mygo/internal/icons"
	"github.com/cretu/focus-must/mygo/internal/storage"
)

func (a *focusApp) analyticsView(c *ui.Context, pal *palette, w, h float32) {
	card(c, pal).Width(min(1080, w*0.94)).Height(h * 0.86).Children(func() {
		ui.Column(c).Padding(16, 20).Gap(4).BorderWidth(0, 0, 1, 0).BorderColor(pal.Border).Children(func() {
			ui.Row(c).Gap(8).AlignItems(ui.Center).Children(func() {
				ui.Icon(c, icons.ChartColumn).FontSize(26).TextColor(pal.Primary)
				heading(c, pal, a.t("app.analyticsTitle"), 20)
			})
			ui.Text(c, a.t("app.analyticsSubtitle")).FontSize(13).TextColor(pal.TextMuted)
		})

		ui.Scroll(c.Key("analytics")).Grow(1).Padding(16, 20).Gap(12).Children(func() {
			an := a.analytics
			if an == nil {
				ui.Text(c, a.t("app.analyticsLoading")).Padding(12).Radius(8).Background(pal.BgElevated)
				return
			}
			ui.Row(c).Gap(10).Children(func() {
				metric(c, pal, icons.Timer, a.t("app.totalFocusDuration"), a.durationLabel(an.Summary.TotalFocusSecs))
				metric(c, pal, icons.Coffee, a.t("app.totalBreakDuration"), a.durationLabel(an.Summary.TotalBreakSecs))
				metric(c, pal, icons.List, a.t("app.totalSessions"), itoa(an.Summary.TotalSessions))
			})
			softCard(c, pal).Children(func() {
				sectionTitle(c, pal, icons.ChartLine, a.t("app.dailyTrend"), "")
				a.trendChart(c, pal, an.Daily)
			})
			softCard(c, pal).Children(func() {
				sectionTitle(c, pal, icons.Clock3, a.t("app.focusHourDistribution"), "")
				a.hourDistribution(c, pal, an)
			})
		})

		ui.Row(c).Padding(14, 20).Justify(ui.End).BorderWidth(1, 0, 0, 0).BorderColor(pal.Border).Children(func() {
			if iconButton(c, pal, icons.ArrowLeft, a.t("app.back")).Clicked() {
				a.view = viewPlanning
			}
		})
	})
}

func itoa(n int64) string { return strconv.FormatInt(n, 10) }

// metric is a summary tile: an icon beside a label and its value.
func metric(c *ui.Context, pal *palette, icon *ui.SVG, label, value string) {
	softCard(c, pal).Grow(1).Basis(0).Children(func() {
		ui.Row(c).Gap(12).AlignItems(ui.Center).Children(func() {
			ui.Box(c).Padding(8).Radius(10).Background(pal.PrimarySoft).Children(func() {
				ui.Icon(c, icon).FontSize(20).TextColor(pal.Primary)
			})
			ui.Column(c).Gap(2).Children(func() {
				ui.Text(c, label).FontSize(12).TextColor(pal.TextMuted)
				ui.Text(c, value).FontSize(20).FontWeight(600).TextColor(pal.TextHighlighted)
			})
		})
	})
}

type trendPoint struct {
	x, y float32 // 0–100 in the plot, as the web app's SVG view box
	day  string
	secs int64
}

const (
	plotTop    = 8
	plotBottom = 92
)

// trendPoints places the days in a 100×100 plot.
func trendPoints(days []storage.DayPoint) []trendPoint {
	var peak int64 = 1
	for _, d := range days {
		peak = max(peak, d.FocusSecs)
	}
	pts := make([]trendPoint, len(days))
	for i, d := range days {
		x := float32(50)
		if len(days) > 1 {
			x = float32(i) / float32(len(days)-1) * 100
		}
		k := min(max(float32(d.FocusSecs)/float32(peak), 0), 1)
		pts[i] = trendPoint{x: x, y: plotTop + (1-k)*(plotBottom-plotTop), day: d.Day, secs: d.FocusSecs}
	}
	return pts
}

// smoothPath is a Catmull-Rom curve through the points, as cubic Béziers,
// mapped by at into the drawing's space.
func smoothPath(pts []trendPoint, at func(x, y float32) (float32, float32)) *ui.Path {
	var path ui.Path
	if len(pts) == 0 {
		return &path
	}
	path.MoveTo(at(pts[0].x, pts[0].y))
	for i := 0; i+1 < len(pts); i++ {
		p0, p1, p2 := pts[max(i-1, 0)], pts[i], pts[i+1]
		p3 := pts[min(i+2, len(pts)-1)]
		c1x, c1y := at(p1.x+(p2.x-p0.x)/6, p1.y+(p2.y-p0.y)/6)
		c2x, c2y := at(p2.x-(p3.x-p1.x)/6, p2.y-(p3.y-p1.y)/6)
		x, y := at(p2.x, p2.y)
		path.CubeTo(c1x, c1y, c2x, c2y, x, y)
	}
	return &path
}

// trendChart is the last 30 days' focus time as a smooth area, with the day
// under the pointer, or the last one, marked and labeled.
func (a *focusApp) trendChart(c *ui.Context, pal *palette, days []storage.DayPoint) {
	pts := trendPoints(days)
	if len(pts) == 0 {
		ui.Text(c, a.t("app.noTrendData")).FontSize(12).TextColor(pal.TextMuted).TextAlign(ui.Center).Padding(24, 0)
		return
	}
	chart := ui.Box(c).Height(180).Label(a.t("app.dailyTrendAria"))
	active := len(pts) - 1
	if x, _, ok := chart.PointerPosition(); ok && chart.Hovered() {
		b := chart.Bounds()
		if b.W > 0 {
			nx := (x - b.X) / b.W * 100
			best := float32(1e9)
			for i, p := range pts {
				if d := abs32(p.x - nx); d < best {
					active, best = i, d
				}
			}
		}
	}
	chart.Draw(func(p *ui.Painter, r ui.Rect) {
		at := func(x, y float32) (float32, float32) { return r.X + x/100*r.W, r.Y + y/100*r.H }
		for _, pt := range pts {
			x, y0 := at(pt.x, plotTop)
			_, y1 := at(pt.x, plotBottom)
			p.Line(x, y0, x, y1, 1, pal.Border.Alpha(0.6))
		}
		area := smoothPath(pts, at)
		lx, by := at(pts[len(pts)-1].x, plotBottom)
		fx, _ := at(pts[0].x, plotBottom)
		area.LineTo(lx, by).LineTo(fx, by).Close()
		p.FillPathGradient(area, ui.LinearGradient{
			Angle: 180, // top to bottom, as CSS's
			Stops: []ui.GradientStop{{Color: clay500.Alpha(0.35), At: 0}, {Color: clay500.Alpha(0.04), At: 1}},
		})
		p.StrokePath(smoothPath(pts, at), 2, clay500)

		ap := pts[active]
		x, y := at(ap.x, ap.y)
		_, y0 := at(ap.x, plotTop)
		p.Line(x, y0, x, by, 1, pal.TextMuted.Alpha(0.5))
		var dot ui.Path
		p.FillPath(dot.Circle(x, y, 5), pal.Bg)
		var ring ui.Path
		p.StrokePath(ring.Circle(x, y, 5), 2, clay500)

		// The tooltip above the point, kept inside the chart.
		date := ui.Span{Text: ap.day, Size: 11, Color: pal.TextMuted}
		value := ui.Span{Text: a.durationCompact(ap.secs), Size: 13, Weight: 600, Color: pal.TextHighlighted}
		dw, dh := p.MeasureText(0, date)
		vw, vh := p.MeasureText(0, value)
		tw, th := max(dw, vw)+16, dh+vh+10
		tx := min(max(x-tw/2, r.X), r.X+r.W-tw)
		ty := max(y-th-12, r.Y)
		p.Shadow(ui.Rect{X: tx, Y: ty, W: tw, H: th}, 8, 0, 4, 12, 0, ui.RGBA(0, 0, 0, 0.15))
		p.Fill(ui.Rect{X: tx, Y: ty, W: tw, H: th}, pal.Bg, 8)
		p.Stroke(ui.Rect{X: tx, Y: ty, W: tw, H: th}, pal.Border, 8, 1)
		p.RichText(tx+8, ty+5, 0, date)
		p.RichText(tx+8, ty+5+dh, 0, value)
	})

	// Labels under the chart: at most about six days.
	step := 1
	if len(pts) > 6 {
		step = (len(pts) + 5) / 6
	}
	ui.Row(c).Justify(ui.SpaceBetween).Gap(8).Children(func() {
		for i, pt := range pts {
			if i%step != 0 && i != len(pts)-1 {
				continue
			}
			ui.Column(c.Key(pt.day)).AlignItems(ui.Center).Children(func() {
				day := pt.day
				if len(day) > 5 {
					day = day[5:]
				}
				ui.Text(c, day).FontSize(10).TextColor(pal.TextMuted)
				ui.Text(c, a.durationCompact(pt.secs)).FontSize(10).FontWeight(500)
			})
		}
	})
}

func abs32(v float32) float32 {
	if v < 0 {
		return -v
	}
	return v
}

// hourDistribution shows the focus time of each hour that has some, as bars
// in two columns.
func (a *focusApp) hourDistribution(c *ui.Context, pal *palette, an *storage.Analytics) {
	var peak int64 = 1
	var active []storage.HourBucket
	for _, b := range an.Hours {
		peak = max(peak, b.FocusSecs)
		if b.FocusSecs > 0 {
			active = append(active, b)
		}
	}
	if len(active) == 0 {
		ui.Text(c, a.t("app.noTrendData")).FontSize(12).TextColor(pal.TextMuted).TextAlign(ui.Center).Padding(24, 0)
		return
	}
	ui.Grid(c).ColumnTracks(ui.Fr(1), ui.Fr(1)).GapX(24).GapY(6).Children(func() {
		for _, b := range active {
			ui.Row(c.Key(b.Hour)).Gap(6).AlignItems(ui.Center).Children(func() {
				ui.Text(c, hourRange(b.Hour)).Width(100).FontSize(11).FontFeatures("tnum").
					Padding(1, 6).Radius(6).Background(pal.BgAccented).TextAlign(ui.Center)
				ui.Progress(c, float64(b.FocusSecs)/float64(peak)).Grow(1).Label(hourRange(b.Hour))
				ui.Text(c, a.durationCompact(b.FocusSecs)).Width(64).FontSize(11).TextColor(pal.TextMuted).
					TextAlign(ui.End).SingleLine()
			})
		}
	})
}
