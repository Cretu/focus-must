package core

import "fmt"

// Clock formats seconds as MM:SS, or HH:MM:SS from an hour on.
func Clock(secs int64) string {
	secs = max(0, secs)
	h, m, s := secs/3600, secs%3600/60, secs%60
	if h > 0 {
		return fmt.Sprintf("%02d:%02d:%02d", h, m, s)
	}
	return fmt.Sprintf("%02d:%02d", m, s)
}
