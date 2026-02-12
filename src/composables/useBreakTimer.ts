import { ref, computed, watch, onUnmounted, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface AppState {
    is_restricted: boolean
    default_whitelist: string[]
    session_whitelist: string[]
    focus_started_at: number | null
    free_activity_end_at: number | null
}

export function useBreakTimer(appState: Ref<AppState>) {
    const showFreeActivityOptions = ref(false)
    const customMinutes = ref('')
    const breakRemaining = ref('')
    let breakTimer: ReturnType<typeof setInterval> | null = null

    function stopBreakTimer() {
        if (breakTimer) {
            clearInterval(breakTimer)
            breakTimer = null
        }
    }

    const isOnBreak = computed(() => {
        return appState.value.free_activity_end_at !== null && appState.value.free_activity_end_at !== undefined
    })

    function updateCountdown() {
        const endAt = appState.value.free_activity_end_at
        if (!endAt) { breakRemaining.value = ''; return }
        const now = Math.floor(Date.now() / 1000)
        const remaining = endAt - now
        if (remaining <= 0) {
            breakRemaining.value = ''
        } else {
            const mins = Math.floor(remaining / 60)
            const secs = remaining % 60
            breakRemaining.value = `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`
        }
    }

    watch(isOnBreak, (v) => {
        if (v) {
            stopBreakTimer()
            updateCountdown()
            breakTimer = setInterval(updateCountdown, 1000)
        } else {
            stopBreakTimer()
            breakRemaining.value = ''
            showFreeActivityOptions.value = false
        }
    }, { immediate: true })

    onUnmounted(() => {
        stopBreakTimer()
    })

    async function startFreeActivity(minutes: number) {
        try {
            await invoke('start_free_activity', { durationMinutes: minutes })
            showFreeActivityOptions.value = false
        } catch (e) {
            console.error('Failed to start free activity:', e)
        }
    }

    return {
        showFreeActivityOptions,
        customMinutes,
        breakRemaining,
        isOnBreak,
        startFreeActivity,
    }
}
