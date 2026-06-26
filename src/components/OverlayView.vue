<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import type { AppState } from "../types/contracts";

const { t, locale } = useI18n();

// --- Motivational quotes (bilingual) ---
const quotes = {
    "zh-CN": [
        { text: "专注是心灵的阳光。", author: "—— 托马斯·卡莱尔" },
        { text: "伟大的作品不是靠力量，而是靠坚持来完成的。", author: "—— 塞缪尔·约翰逊" },
        { text: "只有全神贯注，才能触碰卓越。", author: "—— 阿尔伯特·爱因斯坦" },
        { text: "成功不是偶然的，它是努力、坚持、学习和牺牲的结果。", author: "—— 贝利" },
        { text: "不要让昨天的失败消耗今天的精力。", author: "—— 威廉·莎士比亚" },
        { text: "种一棵树最好的时间是十年前，其次是现在。", author: "—— 中国谚语" },
        { text: "路漫漫其修远兮，吾将上下而求索。", author: "—— 屈原" },
        { text: "耐心是一切聪明才智的基础。", author: "—— 柏拉图" },
        { text: "所谓天才，不过是每天比别人多努力一个小时。", author: "—— 鲁迅" },
        { text: "静心方能致远。", author: "—— 诸葛亮" },
    ],
    "en-US": [
        { text: "Concentrate all your thoughts upon the work at hand.", author: "— Alexander Graham Bell" },
        { text: "The secret of getting ahead is getting started.", author: "— Mark Twain" },
        { text: "It is during our darkest moments that we must focus to see the light.", author: "— Aristotle" },
        { text: "Do what you can, with what you have, where you are.", author: "— Theodore Roosevelt" },
        { text: "The only way to do great work is to love what you do.", author: "— Steve Jobs" },
        { text: "Where focus goes, energy flows.", author: "— Tony Robbins" },
        { text: "You don't have to be great to start, but you have to start to be great.", author: "— Zig Ziglar" },
        { text: "Simplicity is the ultimate sophistication.", author: "— Leonardo da Vinci" },
        { text: "Success is the sum of small efforts repeated day in and day out.", author: "— Robert Collier" },
        { text: "Stay hungry, stay foolish.", author: "— Steve Jobs" },
    ],
};

// --- State ---
const appState = ref<AppState>({
    is_restricted: true,
    default_whitelist: [],
    session_whitelist: [],
    task_description: null,
    focus_started_at: null,
    free_activity_started_at: null,
    free_activity_end_at: null,
    locale: "system",
    focus_goal_minutes: 0,
    paused: false,
});

const elapsedSeconds = ref(0);
const nowSeconds = ref(Math.floor(Date.now() / 1000));
const currentQuoteIndex = ref(0);
let unlistenState: UnlistenFn | null = null;
let timerInterval: ReturnType<typeof setInterval> | null = null;
let quoteInterval: ReturnType<typeof setInterval> | null = null;

// --- Computed ---
const isFocusing = computed(() => appState.value.focus_started_at !== null);
const isOnBreak = computed(() => appState.value.free_activity_end_at !== null);

const currentQuotes = computed(() => {
    const lang = locale.value.startsWith("en") ? "en-US" : "zh-CN";
    return quotes[lang];
});

const currentQuote = computed(() => {
    const q = currentQuotes.value;
    return q[currentQuoteIndex.value % q.length];
});

const formattedTime = computed(() => {
    const total = elapsedSeconds.value;
    const hours = Math.floor(total / 3600);
    const mins = Math.floor((total % 3600) / 60);
    const secs = total % 60;
    if (hours > 0) {
        return `${String(hours).padStart(2, "0")}:${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
    }
    return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
});

const breakRemaining = computed(() => {
    const endAt = appState.value.free_activity_end_at;
    if (!endAt) return "";
    const remaining = endAt - nowSeconds.value;
    if (remaining <= 0) return "";
    const mins = Math.floor(remaining / 60);
    const secs = remaining % 60;
    return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
});

// --- Timer ---
function updateTimer() {
    nowSeconds.value = Math.floor(Date.now() / 1000);
    if (appState.value.focus_started_at) {
        elapsedSeconds.value = Math.max(0, nowSeconds.value - appState.value.focus_started_at);
    }
}

function rotateQuote() {
    currentQuoteIndex.value = (currentQuoteIndex.value + 1) % currentQuotes.value.length;
}

// Keep this (secondary-display) window's language in sync with the app setting,
// resolving "system" the same way the main window does.
function applyLocale(loc: string) {
    locale.value = (
        loc === "system"
            ? navigator.language.toLowerCase().startsWith("en")
            : loc.toLowerCase().startsWith("en")
    )
        ? "en-US"
        : "zh-CN";
}

// --- Lifecycle ---
onMounted(async () => {
    // Random starting quote
    currentQuoteIndex.value = Math.floor(Math.random() * currentQuotes.value.length);

    // Load initial state
    try {
        appState.value = await invoke<AppState>("get_state");
        applyLocale(appState.value.locale);
    } catch (e) {
        console.error("Overlay: failed to load state", e);
    }

    // Listen for state changes
    unlistenState = await listen<AppState>("state-changed", (event) => {
        appState.value = event.payload;
        applyLocale(event.payload.locale);
    });

    // Focus timer
    updateTimer();
    timerInterval = setInterval(updateTimer, 1000);

    // Rotate quote every 30 seconds
    const QUOTE_ROTATION_INTERVAL_MS = 30_000;
    quoteInterval = setInterval(rotateQuote, QUOTE_ROTATION_INTERVAL_MS);
});

onUnmounted(() => {
    if (unlistenState) unlistenState();
    if (timerInterval) clearInterval(timerInterval);
    if (quoteInterval) clearInterval(quoteInterval);
});
</script>

<template>
    <div class="overlay-screen">
        <!-- Focus Mode -->
        <div v-if="isFocusing" class="overlay-content">
            <UIcon name="i-lucide-brain" class="overlay-icon" />
            <h1 class="overlay-title">{{ t("app.keepFocus") }}</h1>
            <div class="overlay-timer">{{ formattedTime }}</div>
            <p v-if="appState.task_description" class="overlay-task">
                {{ appState.task_description }}
            </p>
        </div>

        <!-- Break Mode -->
        <div v-else-if="isOnBreak" class="overlay-content">
            <UIcon name="i-lucide-coffee" class="overlay-icon icon-break" />
            <h1 class="overlay-title">{{ t("overlay.breakTitle") }}</h1>
            <div class="overlay-timer timer-break">{{ breakRemaining }}</div>
        </div>

        <!-- Planning Mode (restricted) -->
        <div v-else class="overlay-content">
            <UIcon name="i-lucide-shield-check" class="overlay-icon icon-planning" />
            <h1 class="overlay-title">Focus Must</h1>
            <p class="overlay-subtitle">{{ t("overlay.planningHint") }}</p>
        </div>

        <!-- Motivational Quote -->
        <div class="quote-container">
            <Transition name="quote-fade" mode="out-in">
                <div :key="currentQuoteIndex" class="quote-content">
                    <p class="quote-text">{{ currentQuote.text }}</p>
                    <p class="quote-author">{{ currentQuote.author }}</p>
                </div>
            </Transition>
        </div>
    </div>
</template>

<style scoped>
.overlay-screen {
    position: fixed;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 48px;
    padding: 48px;
    background:
        radial-gradient(circle at 20% 20%, rgba(59, 130, 246, 0.06), transparent 50%),
        radial-gradient(circle at 80% 80%, rgba(16, 185, 129, 0.05), transparent 50%),
        rgba(10, 14, 24, 0.88);
    backdrop-filter: blur(32px) saturate(120%);
    -webkit-backdrop-filter: blur(32px) saturate(120%);
    color: rgba(255, 255, 255, 0.92);
    user-select: none;
}

:global(html.light) .overlay-screen {
    background:
        radial-gradient(circle at 20% 20%, rgba(59, 130, 246, 0.04), transparent 50%),
        radial-gradient(circle at 80% 80%, rgba(16, 185, 129, 0.03), transparent 50%),
        rgba(236, 245, 255, 0.92);
    color: rgba(15, 23, 42, 0.88);
}

.overlay-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    text-align: center;
}

.overlay-icon {
    font-size: 72px;
    color: rgba(99, 102, 241, 0.85);
    animation: pulse-glow 3s ease-in-out infinite;
}

.icon-break {
    color: rgba(251, 191, 36, 0.85);
}

.icon-planning {
    color: rgba(16, 185, 129, 0.85);
}

.overlay-title {
    font-size: 32px;
    font-weight: 700;
    letter-spacing: 1px;
    opacity: 0.95;
}

.overlay-timer {
    font-size: 80px;
    font-weight: 700;
    font-feature-settings: "tnum";
    font-variant-numeric: tabular-nums;
    letter-spacing: 4px;
    background: linear-gradient(135deg, #818cf8, #6366f1);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    line-height: 1;
}

.timer-break {
    background: linear-gradient(135deg, #fbbf24, #f59e0b);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
}

.overlay-task {
    font-size: 18px;
    opacity: 0.65;
    max-width: 600px;
    line-height: 1.6;
}

.overlay-subtitle {
    font-size: 18px;
    opacity: 0.55;
}

/* Quote */
.quote-container {
    position: absolute;
    bottom: 64px;
    left: 50%;
    transform: translateX(-50%);
    text-align: center;
    max-width: 700px;
    width: 90%;
}

.quote-content {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.quote-text {
    font-size: 18px;
    font-style: italic;
    opacity: 0.5;
    line-height: 1.7;
}

.quote-author {
    font-size: 14px;
    opacity: 0.35;
}

/* Animations */
@keyframes pulse-glow {
    0%, 100% { opacity: 0.85; transform: scale(1); }
    50% { opacity: 1; transform: scale(1.05); }
}

.quote-fade-enter-active,
.quote-fade-leave-active {
    transition: all 0.8s ease;
}

.quote-fade-enter-from {
    opacity: 0;
    transform: translateY(8px);
}

.quote-fade-leave-to {
    opacity: 0;
    transform: translateY(-8px);
}
</style>
