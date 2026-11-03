// Package i18n holds Focus Must's strings in Simplified Chinese and
// English. The locale files are the web frontend's (src/locales), with the
// strings the tray and notifications need added.
package i18n

import (
	"embed"
	"encoding/json"
	"fmt"
	"strings"
)

// Supported locales.
const (
	ZhCN   = "zh-CN"
	EnUS   = "en-US"
	System = "system"
)

//go:embed locales/*.json
var files embed.FS

var catalogs = map[string]map[string]string{}

func init() {
	for _, loc := range []string{ZhCN, EnUS} {
		data, err := files.ReadFile("locales/" + loc + ".json")
		if err != nil {
			panic(err)
		}
		var tree map[string]any
		if err := json.Unmarshal(data, &tree); err != nil {
			panic(fmt.Sprintf("i18n: %s: %v", loc, err))
		}
		flat := map[string]string{}
		flatten("", tree, flat)
		catalogs[loc] = flat
	}
}

func flatten(prefix string, tree map[string]any, out map[string]string) {
	for k, v := range tree {
		key := k
		if prefix != "" {
			key = prefix + "." + k
		}
		switch v := v.(type) {
		case string:
			out[key] = v
		case map[string]any:
			flatten(key, v, out)
		}
	}
}

// Normalize maps a preference to "system", "zh-CN" or "en-US".
func Normalize(pref string) string {
	switch {
	case strings.EqualFold(pref, System):
		return System
	case strings.HasPrefix(strings.ToLower(pref), "en"):
		return EnUS
	}
	return ZhCN
}

// Resolve maps a preference to the locale to show, resolving "system"
// with the system's language (a BCP 47 tag such as "en-US" or
// "zh-Hans-CN"). Only English and Chinese are supported; other languages
// get Chinese, as the Tauri app did.
func Resolve(pref, systemLanguage string) string {
	if n := Normalize(pref); n != System {
		return n
	}
	if strings.HasPrefix(strings.ToLower(systemLanguage), "en") {
		return EnUS
	}
	return ZhCN
}

// T returns the string of the key in the locale, with each {name} replaced
// by the value after "name" in args: T(EnUS, "app.openedApp", "name", "Mail").
// A missing key falls back to English, then to the key itself.
func T(locale, key string, args ...any) string {
	s, ok := catalogs[locale][key]
	if !ok {
		if s, ok = catalogs[EnUS][key]; !ok {
			s = key
		}
	}
	for i := 0; i+1 < len(args); i += 2 {
		s = strings.ReplaceAll(s, "{"+fmt.Sprint(args[i])+"}", fmt.Sprint(args[i+1]))
	}
	return s
}

// Has reports whether the locale defines the key.
func Has(locale, key string) bool {
	_, ok := catalogs[locale][key]
	return ok
}

// Keys returns the keys a locale defines.
func Keys(locale string) []string {
	keys := make([]string, 0, len(catalogs[locale]))
	for k := range catalogs[locale] {
		keys = append(keys, k)
	}
	return keys
}
