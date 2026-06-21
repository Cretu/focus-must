import { ref, watch, onUnmounted } from 'vue'

interface Snowflake {
    x: number; y: number; r: number; speed: number; wind: number; opacity: number
}

interface SnowEffectOptions {
    flakeCount?: number
}

const DEFAULT_FLAKE_COUNT = 120
const SNOW_STORAGE_KEY = 'focus-must-snow'

function loadSnowPreference(): boolean {
    try {
        return localStorage.getItem(SNOW_STORAGE_KEY) === '1'
    } catch {
        return false
    }
}

function isLightTheme(): boolean {
    return document.documentElement.classList.contains('light')
}

export function useSnowEffect(options?: SnowEffectOptions) {
    const flakeCount = Math.max(0, options?.flakeCount ?? DEFAULT_FLAKE_COUNT)
    const snowEnabled = ref(loadSnowPreference())
    const snowCanvas = ref<HTMLCanvasElement | null>(null)
    let animId: number | null = null
    let teardownResize: (() => void) | null = null

    function start() {
        stop()

        const maybeCanvas = snowCanvas.value
        if (!maybeCanvas) return
        const canvas: HTMLCanvasElement = maybeCanvas

        const maybeCtx = canvas.getContext('2d')
        if (!maybeCtx) return
        const ctx: CanvasRenderingContext2D = maybeCtx

        const resizeCanvas = () => {
            canvas.width = window.innerWidth
            canvas.height = window.innerHeight
        }

        resizeCanvas()
        window.addEventListener('resize', resizeCanvas)
        teardownResize = () => {
            window.removeEventListener('resize', resizeCanvas)
            teardownResize = null
        }

        const flakes: Snowflake[] = Array.from({ length: flakeCount }, () => ({
            x: Math.random() * canvas.width,
            y: Math.random() * canvas.height,
            r: Math.random() * 3 + 1,
            speed: Math.random() * 1.5 + 0.5,
            wind: Math.random() * 0.8 - 0.2,
            opacity: Math.random() * 0.6 + 0.3,
        }))

        function draw() {
            if (!snowEnabled.value) return

            ctx.clearRect(0, 0, canvas.width, canvas.height)
            // Snowflakes are white on dark themes; on light themes they'd be
            // invisible, so fall back to a soft slate-blue tone.
            const light = isLightTheme()
            const flakeColor = light ? '148, 163, 184' : '255, 255, 255'
            for (const f of flakes) {
                ctx.beginPath()
                ctx.arc(f.x, f.y, f.r, 0, Math.PI * 2)
                ctx.fillStyle = `rgba(${flakeColor}, ${f.opacity})`
                ctx.fill()
                f.y += f.speed
                f.x += f.wind + Math.sin(f.y * 0.01) * 0.3
                if (f.y > canvas.height) { f.y = -f.r; f.x = Math.random() * canvas.width }
                if (f.x > canvas.width) f.x = 0
                if (f.x < 0) f.x = canvas.width
            }
            animId = requestAnimationFrame(draw)
        }
        draw()
    }

    function stop() {
        if (animId) { cancelAnimationFrame(animId); animId = null }
        if (teardownResize) teardownResize()

        const canvas = snowCanvas.value
        if (canvas) {
            const ctx = canvas.getContext('2d')
            ctx?.clearRect(0, 0, canvas.width, canvas.height)
        }
    }

    watch(snowEnabled, (v) => {
        try {
            localStorage.setItem(SNOW_STORAGE_KEY, v ? '1' : '0')
        } catch {
            // Ignore storage failures (e.g. private mode); snow still works.
        }
        v ? start() : stop()
    })
    onUnmounted(() => {
        stop()
    })

    function setSnowCanvas(el: unknown) {
        snowCanvas.value = el instanceof HTMLCanvasElement ? el : null
        // Kick off immediately if snow was restored as enabled on load: the
        // watcher only fires on change, so the first paint needs this nudge.
        if (snowCanvas.value && snowEnabled.value) start()
    }

    return { snowEnabled, setSnowCanvas }
}
