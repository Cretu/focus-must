//go:build darwin

package macos

import (
	"runtime"
	"sync"
	"unsafe"

	"github.com/ebitengine/purego"
	"github.com/ebitengine/purego/objc"
)

const (
	nsUTF8StringEncoding = 4
	// NSApplicationActivationPolicyRegular: apps in the Dock, which the user
	// works in, as opposed to agents and background processes.
	activationPolicyRegular = 0
)

var (
	loadOnce sync.Once
	poolPush func() uintptr
	poolPop  func(uintptr)
)

func load() {
	loadOnce.Do(func() {
		// AppKit is already loaded in a MyGo app; opening it again makes
		// NSWorkspace available to tests and tools too.
		if _, err := purego.Dlopen("/System/Library/Frameworks/AppKit.framework/AppKit", purego.RTLD_GLOBAL|purego.RTLD_NOW); err != nil {
			panic(err)
		}
		lib, err := purego.Dlopen("/usr/lib/libobjc.A.dylib", purego.RTLD_GLOBAL|purego.RTLD_NOW)
		if err != nil {
			panic(err)
		}
		purego.RegisterLibFunc(&poolPush, lib, "objc_autoreleasePoolPush")
		purego.RegisterLibFunc(&poolPop, lib, "objc_autoreleasePoolPop")
	})
}

// withPool runs fn in an autorelease pool on a locked thread, so the
// objects AppKit autoreleases are released on the thread that made them.
func withPool(fn func()) {
	load()
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()
	pool := poolPush()
	defer poolPop(pool)
	fn()
}

var sels sync.Map // string → objc.SEL

func sel(name string) objc.SEL {
	if s, ok := sels.Load(name); ok {
		return s.(objc.SEL)
	}
	s := objc.RegisterName(name)
	sels.Store(name, s)
	return s
}

func send(obj objc.ID, name string, args ...any) objc.ID {
	if obj == 0 {
		return 0
	}
	return objc.Send[objc.ID](obj, sel(name), args...)
}

func workspace() objc.ID {
	return send(objc.ID(objc.GetClass("NSWorkspace")), "sharedWorkspace")
}

// goString copies an NSString.
func goString(str objc.ID) string {
	if str == 0 {
		return ""
	}
	p := objc.Send[*byte](str, sel("UTF8String"))
	if p == nil {
		return ""
	}
	n := objc.Send[uint](str, sel("lengthOfBytesUsingEncoding:"), uint(nsUTF8StringEncoding))
	return string(unsafe.Slice(p, n))
}

// nsString returns an autoreleased NSString.
func nsString(s string) objc.ID {
	b := []byte(s)
	var p *byte
	if len(b) > 0 {
		p = &b[0]
	}
	str := send(send(objc.ID(objc.GetClass("NSString")), "alloc"), "initWithBytes:length:encoding:",
		p, uint(len(b)), uint(nsUTF8StringEncoding))
	runtime.KeepAlive(b)
	return send(str, "autorelease")
}

func appInfo(app objc.ID) App {
	return App{
		Name:     goString(send(app, "localizedName")),
		BundleID: goString(send(app, "bundleIdentifier")),
		Path:     goString(send(send(app, "bundleURL"), "path")),
		PID:      int(objc.Send[int32](app, sel("processIdentifier"))),
	}
}

// Frontmost returns the app in front.
func Frontmost() (app App, ok bool) {
	withPool(func() {
		front := send(workspace(), "frontmostApplication")
		if front != 0 {
			app, ok = appInfo(front), true
		}
	})
	return app, ok
}

// RunningApps returns the running apps that have a bundle identifier and
// show in the Dock.
func RunningApps() []App {
	var apps []App
	withPool(func() {
		list := send(workspace(), "runningApplications")
		n := objc.Send[uint](list, sel("count"))
		for i := range n {
			app := objc.Send[objc.ID](list, sel("objectAtIndex:"), i)
			if objc.Send[int](app, sel("activationPolicy")) != activationPolicyRegular {
				continue
			}
			info := appInfo(app)
			if info.BundleID != "" {
				apps = append(apps, info)
			}
		}
	})
	return apps
}

// Hide hides the running app of the process, as Cmd+H does.
func Hide(pid int) bool {
	hidden := false
	withPool(func() {
		app := send(objc.ID(objc.GetClass("NSRunningApplication")), "runningApplicationWithProcessIdentifier:", int32(pid))
		if app != 0 {
			hidden = objc.Send[bool](app, sel("hide"))
		}
	})
	return hidden
}

// AppPath returns the path of the app with the bundle identifier, running
// or not, "" when none is installed.
func AppPath(bundleID string) string {
	path := ""
	withPool(func() {
		url := send(workspace(), "URLForApplicationWithBundleIdentifier:", nsString(bundleID))
		path = goString(send(url, "path"))
	})
	return path
}
