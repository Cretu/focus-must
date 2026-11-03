package icons

import "testing"

// Every icon parses: MustParseSVG panics at init otherwise, so loading the
// package is the test.
func TestIconsParse(t *testing.T) {
	if Brain == nil || Coffee == nil || Settings == nil {
		t.Fatal("icons missing")
	}
}
