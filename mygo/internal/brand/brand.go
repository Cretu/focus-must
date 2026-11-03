// Package brand draws Focus Must's starburst mark: eight rounded spokes
// around a center, the same shape as the web app's BrandMark.vue and
// splash screen. It gives the mark as polygons, which the UI fills with a
// painter and TrayIcon rasterizes into the menu bar's template image.
package brand

import (
	"bytes"
	"image"
	"image/color"
	"image/png"
	"math"

	"golang.org/x/image/vector"
)

// ViewBox is the size of the mark's coordinate space, as the SVG's.
const ViewBox = 48

type spoke struct {
	y, h, angle float64 // a 5-wide capsule from y to y+h, turned around the center
}

var spokes = []spoke{
	{3, 15, 0}, {3, 15, 90}, {3, 15, 180}, {3, 15, 270},
	{6.5, 11.5, 45}, {6.5, 11.5, 135}, {6.5, 11.5, 225}, {6.5, 11.5, 315},
}

// Polygons returns the spokes as closed polygons in the 48×48 view box.
func Polygons() [][][2]float64 {
	const (
		cx, cy = 24.0, 24.0
		r      = 2.5 // half the spoke's width, its ends' radius
		steps  = 10  // segments per rounded end
	)
	out := make([][][2]float64, 0, len(spokes))
	for _, s := range spokes {
		top, bottom := s.y+r, s.y+s.h-r
		var pts [][2]float64
		// Upper end: a half circle from the left side to the right, over the top.
		for i := 0; i <= steps; i++ {
			a := math.Pi + math.Pi*float64(i)/steps
			pts = append(pts, [2]float64{cx + r*math.Cos(a), top + r*math.Sin(a)})
		}
		// Lower end: from the right side to the left, under the bottom.
		for i := 0; i <= steps; i++ {
			a := math.Pi * float64(i) / steps
			pts = append(pts, [2]float64{cx + r*math.Cos(a), bottom + r*math.Sin(a)})
		}
		sin, cos := math.Sincos(s.angle * math.Pi / 180)
		for i, p := range pts {
			x, y := p[0]-cx, p[1]-cy
			pts[i] = [2]float64{cx + x*cos - y*sin, cy + x*sin + y*cos}
		}
		out = append(out, pts)
	}
	return out
}

// TrayIcon returns the mark as a black-on-transparent PNG of size×size
// pixels, for a template image that macOS tints to suit the menu bar.
func TrayIcon(size int) []byte {
	z := vector.NewRasterizer(size, size)
	scale := float32(size) / ViewBox
	for _, poly := range Polygons() {
		for i, p := range poly {
			x, y := float32(p[0])*scale, float32(p[1])*scale
			if i == 0 {
				z.MoveTo(x, y)
			} else {
				z.LineTo(x, y)
			}
		}
		z.ClosePath()
	}
	img := image.NewNRGBA(image.Rect(0, 0, size, size))
	z.Draw(img, img.Bounds(), image.NewUniform(color.Black), image.Point{})
	var buf bytes.Buffer
	if err := png.Encode(&buf, img); err != nil {
		panic(err) // encoding an in-memory image does not fail
	}
	return buf.Bytes()
}
