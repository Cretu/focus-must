// Focus Must is a macOS focus companion: write down the task first, choose
// the apps it needs, and the app puts every other app away until the
// session ends.
//
// This is the MyGo edition: the whole app in Go, its interface drawn by
// MyGo's native UI on the GPU, without a webview. See README.md.
package main

import (
	"log"
	"os"

	"github.com/egoist/mygo"

	"github.com/cretu/focus-must/mygo/internal/storage"
)

// version is the app's version when it runs unpackaged (go run); packaged
// builds take mygo.json's.
const version = "3.1.1"

func main() {
	mygo.App.SetLanguages("zh-Hans", "en")

	store := storage.Open(storage.DefaultDir())
	defer store.Close()

	sysLang := ""
	if langs := mygo.App.PreferredLanguages(); len(langs) > 0 {
		sysLang = langs[0]
	}

	d := &desktop{selfPID: os.Getpid()}
	mygo.App.WhenReady(func() {
		d.app = newFocusApp(d, store, sysLang)
		d.start()
	})
	if err := mygo.App.Run(); err != nil {
		log.Fatal(err)
	}
}
