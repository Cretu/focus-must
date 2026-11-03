package main

import (
	"math"
	"math/rand/v2"
	"time"

	"github.com/egoist/mygo/ui"
)

const flakeCount = 120

type flake struct {
	x, y, r, speed, wind, opacity float32
}

// snowfall is the planning screen's falling snow, drawn over everything
// and passing clicks through, as the web app's canvas did.
type snowfall struct {
	enabled bool
	flakes  []flake
	w, h    float32
	last    time.Time
}

func (s *snowfall) reset(w, h float32) {
	s.w, s.h = w, h
	s.flakes = make([]flake, flakeCount)
	for i := range s.flakes {
		s.flakes[i] = flake{
			x:       rand.Float32() * w,
			y:       rand.Float32() * h,
			r:       rand.Float32()*3 + 1,
			speed:   rand.Float32()*1.5 + 0.5,
			wind:    rand.Float32()*0.8 - 0.2,
			opacity: rand.Float32()*0.6 + 0.3,
		}
	}
}

// step moves the flakes by the time since the last frame. The speeds are
// the web app's, per frame at 60 frames a second.
func (s *snowfall) step(now time.Time) {
	frames := float32(1)
	if !s.last.IsZero() {
		frames = min(float32(now.Sub(s.last).Seconds()*60), 4)
	}
	s.last = now
	for i := range s.flakes {
		f := &s.flakes[i]
		f.y += f.speed * frames
		f.x += (f.wind + float32(math.Sin(float64(f.y)*0.01))*0.3) * frames
		if f.y > s.h {
			f.y, f.x = -f.r, rand.Float32()*s.w
		}
		if f.x > s.w {
			f.x = 0
		}
		if f.x < 0 {
			f.x = s.w
		}
	}
}

// view draws the snow over the window when it is on.
func (s *snowfall) view(c *ui.Context, pal *palette) {
	if !s.enabled {
		s.last = time.Time{}
		return
	}
	ui.Box(c).Absolute().Top(0).Left(0).Right(0).Bottom(0).PassThrough().Draw(func(p *ui.Painter, r ui.Rect) {
		if len(s.flakes) == 0 || s.w != r.W || s.h != r.H {
			s.reset(r.W, r.H)
		}
		s.step(p.Now())
		var path ui.Path
		for _, f := range s.flakes {
			path.Reset().Circle(r.X+f.x, r.Y+f.y, f.r)
			p.FillPath(&path, pal.Snow.Alpha(f.opacity))
		}
		p.AnimationFrame()
	})
}

// quote is a motivational line the overlays show.
type quote struct{ text, author string }

var quotes = map[string][]quote{
	"zh-CN": {
		{"专注是心灵的阳光。", "—— 托马斯·卡莱尔"},
		{"伟大的作品不是靠力量，而是靠坚持来完成的。", "—— 塞缪尔·约翰逊"},
		{"只有全神贯注，才能触碰卓越。", "—— 阿尔伯特·爱因斯坦"},
		{"成功不是偶然的，它是努力、坚持、学习和牺牲的结果。", "—— 贝利"},
		{"不要让昨天的失败消耗今天的精力。", "—— 威廉·莎士比亚"},
		{"种一棵树最好的时间是十年前，其次是现在。", "—— 中国谚语"},
		{"路漫漫其修远兮，吾将上下而求索。", "—— 屈原"},
		{"耐心是一切聪明才智的基础。", "—— 柏拉图"},
		{"所谓天才，不过是每天比别人多努力一个小时。", "—— 鲁迅"},
		{"静心方能致远。", "—— 诸葛亮"},
	},
	"en-US": {
		{"Concentrate all your thoughts upon the work at hand.", "— Alexander Graham Bell"},
		{"The secret of getting ahead is getting started.", "— Mark Twain"},
		{"It is during our darkest moments that we must focus to see the light.", "— Aristotle"},
		{"Do what you can, with what you have, where you are.", "— Theodore Roosevelt"},
		{"The only way to do great work is to love what you do.", "— Steve Jobs"},
		{"Where focus goes, energy flows.", "— Tony Robbins"},
		{"You don't have to be great to start, but you have to start to be great.", "— Zig Ziglar"},
		{"Simplicity is the ultimate sophistication.", "— Leonardo da Vinci"},
		{"Success is the sum of small efforts repeated day in and day out.", "— Robert Collier"},
		{"Stay hungry, stay foolish.", "— Steve Jobs"},
	},
}

const quoteInterval = 30 * time.Second

// quoteRotation picks a random quote, then the next every 30 s.
type quoteRotation struct {
	index int
	since time.Time
}

func (q *quoteRotation) start(now time.Time) {
	q.index = rand.IntN(len(quotes["en-US"]))
	q.since = now
}

func (q *quoteRotation) advance(now time.Time) {
	if now.Sub(q.since) >= quoteInterval {
		q.index++
		q.since = now
	}
}

func (q *quoteRotation) current(locale string) quote {
	list := quotes[locale]
	return list[q.index%len(list)]
}
