package main

import (
	"cmp"
	"log"
	"slices"
	"strings"
	"time"

	"github.com/egoist/mygo/ui"

	"github.com/cretu/focus-must/mygo/internal/core"
	"github.com/cretu/focus-must/mygo/internal/i18n"
	"github.com/cretu/focus-must/mygo/internal/storage"
)

// system is what the app asks of the desktop. desktop implements it with
// windows, a tray and AppKit; tests use a fake.
type system interface {
	// showMain shows the main window and the overlays on the other
	// displays, in front.
	showMain()
	// hideAll hides the main window and the overlays.
	hideAll()
	// changed tells that the state changed: the tray's title and menu,
	// and the windows, follow it.
	changed()
	notify(title, body string)
	playSound(name string)
	runningApps() []appItem
	// appInfo describes an app that may not be running, by its bundle
	// identifier.
	appInfo(bundleID string) (appItem, bool)
	// loadIcons loads the icons of the apps without one, then calls done
	// on the main thread.
	loadIcons(apps []appItem, done func(icons map[string]*ui.Bitmap))
	openAtLogin() bool
	setOpenAtLogin(on bool) error
	setAppearance(appearance string)
	selfCheck() selfCheckReport
	now() time.Time
}

// appItem is an app shown in an app grid.
type appItem struct {
	Name     string
	BundleID string
	Path     string
}

// selfCheckReport is what the settings' self-check shows.
type selfCheckReport struct {
	LastFrontmost string
	Displays      []displayInfo
	Version       string
}

type displayInfo struct {
	Name          string
	Width, Height int
	Primary       bool
}

type viewName int

const (
	viewPlanning viewName = iota
	viewSettings
	viewAnalytics
)

const (
	historyPageSize = 20
	maxBreakMinutes = 480
)

var (
	breakPresets          = []int64{5, 10, 15, 30, 45}
	breakReminderOptions  = []int64{25, 45, 60, 90}
	defaultReminderMinute = int64(45)
)

// focusApp is the app's state and what changes it. Everything runs on the
// main thread: the views, the tray's items, and the monitor's ticks, which
// desktop sends there.
type focusApp struct {
	sys      system
	store    *storage.Store
	settings storage.Settings
	state    *core.State
	reminder core.Reminder
	sysLang  string

	view viewName

	// Planning.
	task          string
	taskInvalid   bool
	taskShake     bool      // a shake is due: the view starts it
	taskShakeAt   time.Time // on the frames' clock
	running       []appItem
	selected      map[string]bool
	appsLoading   bool
	icons         map[string]*ui.Bitmap
	showBreakOpts bool
	customMinutes string

	// Focus session.
	allowedApps       []appItem
	showEndConfirm    bool
	showBreakReminder bool
	reminderMinutes   int64
	pausedSelected    map[string]bool

	// History.
	history        []storage.Session
	historyHasMore bool
	historyTab     int
	historyScroll  ui.ScrollState

	// Analytics.
	analytics      *storage.Analytics
	analyticsError bool
	trendHover     int

	// Settings form.
	settingsApps       []appItem
	settingsWhitelist  map[string]bool
	settingsLocale     string
	settingsAppearance string
	autostart          bool
	reminderOn         bool
	reminderChoice     string
	selfCheck          selfCheckReport

	// Snow.
	snow   snowfall
	quotes quoteRotation
}

func newFocusApp(sys system, store *storage.Store, sysLang string) *focusApp {
	a := &focusApp{
		sys:            sys,
		store:          store,
		state:          core.NewState(),
		sysLang:        sysLang,
		selected:       map[string]bool{},
		pausedSelected: map[string]bool{},
		icons:          map[string]*ui.Bitmap{},
		historyHasMore: true,
		trendHover:     -1,
	}
	a.settings = store.LoadSettings()
	if len(a.settings.DefaultWhitelist) > 0 {
		a.state.DefaultWhitelist = slices.Clone(a.settings.DefaultWhitelist)
	}
	a.state.Locale = i18n.Normalize(a.settings.Locale)
	a.state.BreakReminderMinutes = a.settings.BreakReminderMinutes
	a.snow.enabled = a.settings.Snow
	a.quotes.start(sys.now())
	sys.setAppearance(a.settings.Appearance)
	a.selectDefaults()
	a.reloadHistory()
	a.loadApps()
	return a
}

// locale is the language the app shows.
func (a *focusApp) locale() string { return i18n.Resolve(a.state.Locale, a.sysLang) }

// t returns a string in the app's language.
func (a *focusApp) t(key string, args ...any) string { return i18n.T(a.locale(), key, args...) }

func (a *focusApp) unix() int64 { return a.sys.now().Unix() }

// ---------------------------------------------------------------------------
// Apps

func (a *focusApp) selectDefaults() {
	if len(a.state.DefaultWhitelist) == 0 {
		return
	}
	a.selected = map[string]bool{}
	for _, id := range a.state.DefaultWhitelist {
		a.selected[id] = true
	}
}

// loadApps lists the running apps for the planning and paused views.
func (a *focusApp) loadApps() {
	a.appsLoading = true
	a.running = a.sys.runningApps()
	a.appsLoading = false
	a.hydrateIcons(a.running)
}

func (a *focusApp) hydrateIcons(apps []appItem) {
	var missing []appItem
	for _, app := range apps {
		if _, ok := a.icons[app.BundleID]; !ok && app.Path != "" {
			missing = append(missing, app)
		}
	}
	if len(missing) == 0 {
		return
	}
	a.sys.loadIcons(missing, func(icons map[string]*ui.Bitmap) {
		for id, icon := range icons {
			a.icons[id] = icon
		}
	})
}

func toggle(set map[string]bool, id string) {
	if set[id] {
		delete(set, id)
	} else {
		set[id] = true
	}
}

func keys(set map[string]bool) []string {
	out := make([]string, 0, len(set))
	for id := range set {
		out = append(out, id)
	}
	slices.Sort(out)
	return out
}

// ---------------------------------------------------------------------------
// Focus sessions

// recentTasks are the tasks of the latest focus sessions, newest first,
// each once.
func (a *focusApp) recentTasks() []string {
	sessions := slices.Clone(a.history)
	slices.SortStableFunc(sessions, func(x, y storage.Session) int { return cmp.Compare(y.StartedAt, x.StartedAt) })
	var tasks []string
	seen := map[string]bool{}
	for _, s := range sessions {
		task := strings.TrimSpace(s.Task)
		if s.Type != "focus" || task == "" || seen[task] {
			continue
		}
		seen[task] = true
		tasks = append(tasks, task)
		if len(tasks) == 6 {
			break
		}
	}
	return tasks
}

func (a *focusApp) applyRecentTask(task string) {
	a.task = task
	a.taskInvalid = false
}

// currentTask is the task of the running session.
func (a *focusApp) currentTask() string {
	if t := strings.TrimSpace(a.state.Task); t != "" {
		return t
	}
	return strings.TrimSpace(a.task)
}

func (a *focusApp) startFocus() {
	task := strings.TrimSpace(a.task)
	if task == "" {
		a.taskInvalid = true
		a.taskShake = true
		return
	}
	a.taskInvalid = false
	whitelist := keys(a.selected)
	a.allowedApps = nil
	for _, app := range a.running {
		if a.selected[app.BundleID] {
			a.allowedApps = append(a.allowedApps, app)
		}
	}
	if rec := a.state.StartFocus(whitelist, task, a.unix()); rec != nil {
		a.record(*rec)
	}
	a.showBreakOpts = false
	a.sys.changed()
	a.sys.hideAll()
}

func (a *focusApp) pauseFocus() {
	if !a.state.Pause(a.unix()) {
		return
	}
	a.pausedSelected = map[string]bool{}
	for _, id := range a.state.SessionWhitelist {
		a.pausedSelected[id] = true
	}
	a.loadApps()
	a.sys.changed()
	a.sys.showMain()
}

func (a *focusApp) resumeFocus() {
	if !a.state.Resume(keys(a.pausedSelected), a.unix()) {
		return
	}
	a.allowedApps = nil
	for _, app := range a.running {
		if a.pausedSelected[app.BundleID] {
			a.allowedApps = append(a.allowedApps, app)
		}
	}
	a.sys.changed()
	a.sys.hideAll()
}

// endFocus ends the focus session, from the focus card, the paused view or
// the tray, and shows the planner.
func (a *focusApp) endFocus() {
	a.showEndConfirm = false
	a.showBreakReminder = false
	if rec := a.state.EndFocus(a.unix()); rec != nil {
		a.record(*rec)
	}
	a.selected = map[string]bool{}
	a.task = ""
	a.allowedApps = nil
	a.view = viewPlanning
	a.loadApps()
	a.selectDefaults()
	a.sys.changed()
	a.sys.showMain()
}

// continueFocus hides the window and keeps the session going, after a look
// at it from the tray.
func (a *focusApp) continueFocus() {
	a.showBreakReminder = false
	a.sys.hideAll()
}

// ---------------------------------------------------------------------------
// Breaks

func (a *focusApp) startBreak(minutes int64) {
	minutes = min(max(minutes, 1), maxBreakMinutes)
	if a.state.FocusStartedAt != 0 {
		// A break ends the focus session, as the Tauri app's did, and
		// keeps it in the history.
		if rec := a.state.EndFocus(a.unix()); rec != nil {
			a.record(*rec)
		}
	}
	a.showBreakReminder = false
	a.state.StartBreak(minutes, a.unix())
	a.showBreakOpts = false
	a.customMinutes = ""
	a.sys.changed()
	a.sys.hideAll()
}

// startCustomBreak starts a break of the minutes typed, when valid.
func (a *focusApp) startCustomBreak() {
	var minutes int64
	s := strings.TrimSpace(a.customMinutes)
	for _, r := range s {
		if r < '0' || r > '9' {
			return
		}
		minutes = minutes*10 + int64(r-'0')
		if minutes > maxBreakMinutes {
			minutes = maxBreakMinutes
			break
		}
	}
	if minutes < 1 {
		return
	}
	a.startBreak(minutes)
}

// endBreak ends the break, early from the tray or when its time is up.
func (a *focusApp) endBreak() {
	if rec := a.state.EndBreak(a.unix()); rec != nil {
		a.record(*rec)
	}
	a.view = viewPlanning
	a.sys.changed()
	a.sys.showMain()
}

// ---------------------------------------------------------------------------
// The clock

// tick runs once a second: it ends a break whose time is up and fires the
// break reminder.
func (a *focusApp) tick() {
	now := a.unix()
	if a.state.BreakEndAt != 0 && now >= a.state.BreakEndAt {
		a.endBreak()
		return
	}
	if minutes, due := a.reminder.Due(a.state, now); due {
		a.reminderMinutes = minutes
		a.showBreakReminder = true
		a.view = viewPlanning
		a.sys.playSound("Glass")
		a.sys.showMain()
	}
	a.quotes.advance(a.sys.now())
}

// ---------------------------------------------------------------------------
// History and analytics

func (a *focusApp) record(rec core.Record) {
	a.store.Append(rec)
	a.reloadHistory()
}

func (a *focusApp) reloadHistory() {
	page := a.store.History(0, historyPageSize)
	a.history = page.Items
	a.historyHasMore = page.HasMore
}

func (a *focusApp) loadMoreHistory() {
	if !a.historyHasMore {
		return
	}
	page := a.store.History(len(a.history), historyPageSize)
	a.history = append(a.history, page.Items...)
	a.historyHasMore = page.HasMore
}

func (a *focusApp) openAnalytics() {
	a.view = viewAnalytics
	a.trendHover = -1
	an, err := a.store.Analytics()
	if err != nil {
		log.Printf("Failed to load analytics: %v", err)
	}
	a.analytics = &an
	a.analyticsError = err != nil
}

// ---------------------------------------------------------------------------
// Settings

func (a *focusApp) openSettings() {
	a.view = viewSettings
	a.settingsWhitelist = map[string]bool{}
	for _, id := range a.state.DefaultWhitelist {
		a.settingsWhitelist[id] = true
	}
	a.settingsLocale = a.state.Locale
	a.settingsAppearance = cmp.Or(a.settings.Appearance, "system")
	a.autostart = a.sys.openAtLogin()
	a.reminderOn = a.state.BreakReminderMinutes > 0
	a.reminderChoice = a.reminderLabel(cmp.Or(a.state.BreakReminderMinutes, defaultReminderMinute))
	a.refreshSettingsApps()
	a.selfCheck = a.sys.selfCheck()
}

// refreshSettingsApps lists the running apps, with the whitelisted ones
// that do not run first, so they stay selectable. It keeps the unsaved
// choices.
func (a *focusApp) refreshSettingsApps() {
	apps := a.sys.runningApps()
	have := map[string]bool{}
	for _, app := range apps {
		have[app.BundleID] = true
	}
	var missing []appItem
	for _, id := range keys(a.settingsWhitelist) {
		if have[id] {
			continue
		}
		app, ok := a.sys.appInfo(id)
		if !ok {
			app = appItem{Name: id, BundleID: id}
		}
		missing = append(missing, app)
	}
	a.settingsApps = append(missing, apps...)
	a.hydrateIcons(a.settingsApps)
}

func (a *focusApp) reminderLabel(minutes int64) string {
	return a.t("app.minutesShort", "minutes", minutes)
}

// setBreakReminder sets the reminder interval right away, as the web app's
// switch and select did, without waiting for Save.
func (a *focusApp) setBreakReminder(minutes int64) {
	a.state.BreakReminderMinutes = minutes
	a.settings.BreakReminderMinutes = minutes
	a.store.SaveSettings(a.settings)
	a.sys.changed()
}

func (a *focusApp) saveSettings() {
	a.state.DefaultWhitelist = keys(a.settingsWhitelist)
	a.settings.DefaultWhitelist = slices.Clone(a.state.DefaultWhitelist)
	if err := a.sys.setOpenAtLogin(a.autostart); err != nil {
		log.Printf("Failed to update autostart: %v", err)
	}
	a.state.Locale = i18n.Normalize(a.settingsLocale)
	a.settings.Locale = a.state.Locale
	a.settings.Appearance = a.settingsAppearance
	a.sys.setAppearance(a.settingsAppearance)
	a.store.SaveSettings(a.settings)
	a.view = viewPlanning
	a.selectDefaults()
	a.sys.changed()
}

func (a *focusApp) setSnow(on bool) {
	a.snow.enabled = on
	a.settings.Snow = on
	a.store.SaveSettings(a.settings)
}

func (a *focusApp) setAppearance(appearance string) {
	a.settings.Appearance = appearance
	a.settingsAppearance = appearance
	a.sys.setAppearance(appearance)
	a.store.SaveSettings(a.settings)
}

// testNotification sends the notification a blocked app sends.
func (a *focusApp) testNotification() {
	a.sys.notify("Focus Must", a.t("notify.distraction", "name", a.t("notify.selfCheck")))
}

// ---------------------------------------------------------------------------
// The monitor

// blocked handles the app in front being off the whitelist: the desktop
// has hidden it; tell the user, at most every 20 s for the same app.
type distractionNotifier struct {
	lastID string
	lastAt time.Time
}

const notifyThrottle = 20 * time.Second

func (n *distractionNotifier) shouldNotify(bundleID string, now time.Time) bool {
	if bundleID == n.lastID && now.Sub(n.lastAt) < notifyThrottle {
		return false
	}
	n.lastID, n.lastAt = bundleID, now
	return true
}
