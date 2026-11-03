package brand

import (
	"bytes"
	"image/png"
	"testing"
)

func TestTrayIcon(t *testing.T) {
	img, err := png.Decode(bytes.NewReader(TrayIcon(32)))
	if err != nil {
		t.Fatal(err)
	}
	if b := img.Bounds(); b.Dx() != 32 || b.Dy() != 32 {
		t.Fatalf("size %v", b)
	}
	// The spokes are drawn; the center between them, as the web app's
	// mark, and the corners are empty.
	if _, _, _, a := img.At(16, 16).RGBA(); a != 0 {
		t.Error("center is drawn")
	}
	if _, _, _, a := img.At(16, 4).RGBA(); a == 0 {
		t.Error("top spoke is empty")
	}
	if _, _, _, a := img.At(1, 1).RGBA(); a != 0 {
		t.Error("corner is drawn")
	}
}
