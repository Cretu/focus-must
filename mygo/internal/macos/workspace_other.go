//go:build !darwin

package macos

// Frontmost reports no app: monitoring needs macOS.
func Frontmost() (App, bool) { return App{}, false }

// RunningApps reports no apps: listing them needs macOS.
func RunningApps() []App { return nil }

// Hide does nothing outside macOS.
func Hide(pid int) bool { return false }

// AppPath reports no path outside macOS.
func AppPath(bundleID string) string { return "" }
