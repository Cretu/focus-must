package main

import (
	"log"
	"os/exec"
	"slices"
	"sync"
	"time"

	"github.com/egoist/mygo"
	"github.com/egoist/mygo/ui"

	"github.com/cretu/focus-must/mygo/internal/brand"
	"github.com/cretu/focus-must/mygo/internal/core"
	"github.com/cretu/focus-must/mygo/internal/macos"
)

// pollInterval is how often the monitor looks at the app in front.
const pollInterval = 500 * time.Millisecond

// desktop is the system the app runs on: the main window over the primary
// display, an overlay over each other display, the menu bar's tray and
// AppKit's running applications. Its methods run on the main thread.
type desktop struct {
	app      *focusApp
	main     *mygo.Window
	overlays []*mygo.Window
	tray     *mygo.Tray
	quitting bool
	visible  bool

	notifier      distractionNotifier
	lastFrontmost string
	lastTick      int64
	selfPID       int
	trayMenuKey   string
}

func (d *desktop) now() time.Time { return time.Now() }

// start makes the windows and the tray, and starts monitoring. Call it once
// the app is ready.
func (d *desktop) start() {
	mygo.App.SetActivationPolicy(mygo.ActivationPolicyAccessory)
	mygo.App.OnBeforeQuit(func(*mygo.QuitEvent) { d.quitting = true })

	primary := mygo.Screen.PrimaryDisplay().Bounds
	d.main = mygo.NewWindow(d.coverOptions(primary, ui.View(d.app.mainView)))
	d.main.SetVisibleOnAllWorkspaces(true)
	d.main.OnClose(func(e *mygo.CloseEvent) {
		// Cmd+W hides the planner; only Quit closes it.
		if !d.quitting {
			e.PreventDefault()
			d.hideAll()
		}
	})
	d.syncOverlays()
	mygo.Screen.OnDisplaysChanged(d.syncOverlays)

	var err error
	d.tray, err = mygo.NewTray(mygo.TrayOptions{
		ID:             "focus-tray",
		Icon:           brand.TrayIcon(36),
		IconIsTemplate: true,
		ToolTip:        "Focus Must",
		Title:          d.app.t("tray.planning"),
	})
	if err != nil {
		log.Printf("Failed to create the tray icon: %v", err)
	}
	d.changed()
	d.showMain()

	go func() {
		for range time.Tick(pollInterval) {
			mygo.RunOnMain(d.poll)
		}
	}()
}

// coverOptions are a window covering a display, without chrome, in which
// the view tints the desktop.
func (d *desktop) coverOptions(r mygo.Rectangle, content *ui.Content) mygo.WindowOptions {
	return mygo.WindowOptions{
		Title:             "Focus Must",
		X:                 r.X,
		Y:                 r.Y,
		Width:             r.Width,
		Height:            r.Height,
		Hidden:            true,
		Frameless:         true,
		Transparent:       true,
		DisableResize:     true,
		DisableMove:       true,
		DisableMinimize:   true,
		DisableMaximize:   true,
		DisableFullScreen: true,
		DisableShadow:     true,
		SkipTaskbar:       true,
		Content:           content,
	}
}

// syncOverlays sizes the main window to the primary display and makes an
// overlay for every other display, again when displays come and go.
func (d *desktop) syncOverlays() {
	displays := mygo.Screen.Displays()
	for _, w := range d.overlays {
		w.Destroy()
	}
	d.overlays = nil
	for _, disp := range displays {
		if disp.Primary {
			d.main.SetBounds(disp.Bounds)
			continue
		}
		w := mygo.NewWindow(d.coverOptions(disp.Bounds, ui.View(d.app.overlayView)))
		w.SetVisibleOnAllWorkspaces(true)
		w.OnClose(func(e *mygo.CloseEvent) {
			if !d.quitting {
				e.PreventDefault()
			}
		})
		d.overlays = append(d.overlays, w)
		if d.visible {
			w.ShowInactive()
		}
	}
}

func (d *desktop) showMain() {
	d.visible = true
	d.main.SetAlwaysOnTop(false)
	d.main.Show()
	d.main.Focus()
	mygo.App.Focus()
	for _, w := range d.overlays {
		w.SetAlwaysOnTop(false)
		w.ShowInactive()
	}
}

func (d *desktop) hideAll() {
	d.visible = false
	d.main.Hide()
	for _, w := range d.overlays {
		w.Hide()
	}
}

// poll runs every half second on the main thread: it hides an app in front
// that the session does not allow, and once a second runs the clock.
func (d *desktop) poll() {
	a := d.app
	if front, ok := macos.Frontmost(); ok && front.BundleID != "" && front.PID != d.selfPID {
		d.lastFrontmost = front.BundleID
		if a.state.ShouldBlock(front.BundleID) {
			// Gently put the distraction away, as Cmd+H does.
			macos.Hide(front.PID)
			if d.notifier.shouldNotify(front.BundleID, time.Now()) {
				d.notify("Focus Must", a.t("notify.distraction", "name", front.Name))
			}
		}
	}
	if now := time.Now().Unix(); now != d.lastTick {
		d.lastTick = now
		a.tick()
		d.updateTrayTitle()
		d.invalidate()
	}
}

func (d *desktop) invalidate() {
	if d.main == nil {
		return
	}
	d.main.Invalidate()
	for _, w := range d.overlays {
		w.Invalidate()
	}
}

// changed brings the tray, the language of MyGo's own controls and the
// windows up to date with the state.
func (d *desktop) changed() {
	a := d.app
	if a.locale() == "en-US" {
		mygo.App.SetLocale("en")
	} else {
		mygo.App.SetLocale("zh-Hans")
	}
	d.updateTrayMenu()
	d.updateTrayTitle()
	d.invalidate()
}

func (d *desktop) updateTrayTitle() {
	if d.tray == nil {
		return
	}
	a := d.app
	now := a.unix()
	title := a.t("tray.planning")
	switch a.state.Phase() {
	case core.Focusing:
		title = a.t("tray.focusPrefix") + " " + core.Clock(a.state.Elapsed(now))
	case core.Paused:
		title = a.t("tray.paused")
	case core.OnBreak:
		title = a.t("tray.breakPrefix") + " " + core.Clock(a.state.BreakRemaining(now))
	}
	d.tray.SetTitle(title)
}

// updateTrayMenu rebuilds the tray's menu when its items change: their
// language, and which of them the phase enables.
func (d *desktop) updateTrayMenu() {
	if d.tray == nil {
		return
	}
	a := d.app
	phase := a.state.Phase()
	inFocus := phase == core.Focusing || phase == core.Paused
	lock, pause, endBreak := a.t("tray.lockInactive"), a.t("tray.pauseInactive"), a.t("tray.breakInactive")
	if inFocus {
		lock, pause = a.t("tray.lockActive"), a.t("tray.pause")
		if phase == core.Paused {
			pause = a.t("tray.resume")
		}
	}
	if phase == core.OnBreak {
		endBreak = a.t("tray.breakActive")
	}
	key := a.locale() + lock + pause + endBreak
	if key == d.trayMenuKey {
		return
	}
	d.trayMenuKey = key
	d.tray.SetMenu(mygo.NewMenu([]*mygo.MenuItem{
		{ID: "show", Label: a.t("tray.show"), Click: func(*mygo.MenuItem, *mygo.Window) {
			a.view = viewPlanning
			d.showMain()
		}},
		{ID: "lock", Label: lock, Disabled: !inFocus, Click: func(*mygo.MenuItem, *mygo.Window) { a.endFocus() }},
		{ID: "pause", Label: pause, Disabled: !inFocus, Click: func(*mygo.MenuItem, *mygo.Window) {
			if a.state.Paused {
				a.resumeFocus()
			} else {
				a.pauseFocus()
			}
		}},
		{ID: "end_break", Label: endBreak, Disabled: phase != core.OnBreak, Click: func(*mygo.MenuItem, *mygo.Window) { a.endBreak() }},
		mygo.Separator(),
		{ID: "settings", Label: a.t("tray.settings"), Click: func(*mygo.MenuItem, *mygo.Window) {
			if !inFocus || a.state.Paused {
				a.openSettings()
			}
			d.showMain()
		}},
		{ID: "quit", Label: a.t("tray.quit"), Click: func(*mygo.MenuItem, *mygo.Window) { mygo.App.Quit() }},
	}))
}

func (d *desktop) notify(title, body string) {
	n := mygo.NewNotification(mygo.NotificationOptions{Title: title, Body: body})
	go func() {
		if err := n.Show(); err != nil {
			log.Printf("Failed to show notification: %v", err)
		}
	}()
}

// playSound plays one of macOS's sounds, as "Glass".
func (d *desktop) playSound(name string) {
	if err := exec.Command("afplay", "/System/Library/Sounds/"+name+".aiff").Start(); err != nil {
		log.Printf("Failed to play sound %s: %v", name, err)
	}
}

func (d *desktop) runningApps() []appItem {
	var apps []appItem
	for _, app := range macos.RunningApps() {
		if app.PID == d.selfPID {
			continue
		}
		apps = append(apps, appItem{Name: app.Name, BundleID: app.BundleID, Path: app.Path})
	}
	return apps
}

func (d *desktop) appInfo(bundleID string) (appItem, bool) {
	path := macos.AppPath(bundleID)
	if path == "" {
		return appItem{}, false
	}
	name := path
	if i := lastSlash(path); i >= 0 {
		name = path[i+1:]
	}
	if len(name) > 4 && name[len(name)-4:] == ".app" {
		name = name[:len(name)-4]
	}
	return appItem{Name: name, BundleID: bundleID, Path: path}, true
}

func lastSlash(s string) int {
	for i := len(s) - 1; i >= 0; i-- {
		if s[i] == '/' {
			return i
		}
	}
	return -1
}

var iconMu sync.Mutex

// loadIcons draws the apps' icons off the frame being built, a few
// milliseconds each, and hands them back on the main thread.
func (d *desktop) loadIcons(apps []appItem, done func(map[string]*ui.Bitmap)) {
	apps = slices.Clone(apps)
	go func() {
		iconMu.Lock()
		defer iconMu.Unlock()
		icons := map[string]*ui.Bitmap{}
		for _, app := range apps {
			png, err := mygo.Shell.FileIcon(app.Path, 80) // 40 DIPs at 2×
			if err != nil {
				icons[app.BundleID] = nil
				continue
			}
			bmp, err := ui.DecodeBitmap(png)
			if err != nil {
				log.Printf("Failed to decode icon of %s: %v", app.BundleID, err)
			}
			icons[app.BundleID] = bmp
		}
		mygo.RunOnMain(func() {
			done(icons)
			d.invalidate()
		})
	}()
}

func (d *desktop) openAtLogin() bool { return mygo.App.OpenAtLogin() }

func (d *desktop) setOpenAtLogin(on bool) error { return mygo.App.SetOpenAtLogin(on) }

func (d *desktop) setAppearance(appearance string) {
	switch appearance {
	case "light":
		mygo.Theme.SetSource(mygo.ThemeLight)
	case "dark":
		mygo.Theme.SetSource(mygo.ThemeDark)
	default:
		mygo.Theme.SetSource(mygo.ThemeSystem)
	}
}

func (d *desktop) selfCheck() selfCheckReport {
	r := selfCheckReport{LastFrontmost: d.lastFrontmost, Version: mygo.App.Version()}
	if r.Version == "" {
		r.Version = version
	}
	for _, disp := range mygo.Screen.Displays() {
		r.Displays = append(r.Displays, displayInfo{
			Name:    disp.Label,
			Width:   int(float64(disp.Bounds.Width) * disp.ScaleFactor),
			Height:  int(float64(disp.Bounds.Height) * disp.ScaleFactor),
			Primary: disp.Primary,
		})
	}
	return r
}
