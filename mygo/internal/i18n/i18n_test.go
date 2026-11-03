package i18n

import "testing"

func TestLocalesHaveTheSameKeys(t *testing.T) {
	for _, pair := range [][2]string{{ZhCN, EnUS}, {EnUS, ZhCN}} {
		for _, k := range Keys(pair[0]) {
			if !Has(pair[1], k) {
				t.Errorf("%s has %q, %s does not", pair[0], k, pair[1])
			}
		}
	}
}

func TestT(t *testing.T) {
	if got := T(EnUS, "app.openedApp", "name", "Mail"); got != "You opened Mail" {
		t.Errorf("got %q", got)
	}
	if got := T(ZhCN, "app.selectedCount", "count", 3); got != "已选 3 个" {
		t.Errorf("got %q", got)
	}
	if got := T(ZhCN, "no.such.key"); got != "no.such.key" {
		t.Errorf("missing key gave %q", got)
	}
}

func TestResolve(t *testing.T) {
	cases := []struct{ pref, sys, want string }{
		{"system", "en-GB", EnUS},
		{"system", "zh-Hans-CN", ZhCN},
		{"system", "fr-FR", ZhCN},
		{"en-US", "zh-Hans-CN", EnUS},
		{"zh-CN", "en-US", ZhCN},
		{"SYSTEM", "en", EnUS},
	}
	for _, c := range cases {
		if got := Resolve(c.pref, c.sys); got != c.want {
			t.Errorf("Resolve(%q, %q) = %q, want %q", c.pref, c.sys, got, c.want)
		}
	}
}
