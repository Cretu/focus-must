// Package macos reads and controls the running applications through
// AppKit's NSWorkspace: the app in front, the running apps, and hiding one.
// It calls AppKit through purego, without cgo. On other systems the
// functions report nothing, so the rest of the app still builds and runs.
package macos

// App is a running or installed application.
type App struct {
	Name     string
	BundleID string
	// Path is the app bundle's path, "" when unknown.
	Path string
	// PID is the process identifier of a running app, 0 otherwise.
	PID int
}
