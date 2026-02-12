import { ref, watch, onUnmounted } from 'vue'

interface Snowflake {
    x: number; y: number; r: number; speed: number; wind: number; opacity: number
}

export function useSnowEffect() {
    const snowEnabled = ref(false)
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

        const flakes: Snowflake[] = Array.from({ length: 120 }, () => ({
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
            for (const f of flakes) {
                ctx.beginPath()
                ctx.arc(f.x, f.y, f.r, 0, Math.PI * 2)
                ctx.fillStyle = `rgba(255, 255, 255, ${f.opacity})`
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

    watch(snowEnabled, (v) => { v ? start() : stop() })
    onUnmounted(() => {
        stop()
    })

    return { snowEnabled, snowCanvas }
}
