<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
    isEnabled as isAutostartEnabled,
    enable as enableAutostart,
    disable as disableAutostart,
} from "@tauri-apps/plugin-autostart";
import { useSnowEffect } from "./composables/useSnowEffect";
import { useBreakTimer } from "./composables/useBreakTimer";
import HistoryList, { type SessionRecord } from "./components/HistoryList.vue";

interface AppInfo {
    name: string;
    bundle_id: string;
    icon_data_url?: string | null;
}

interface BlockedAppEvent {
    name: string;
    bundle_id: string;
    return_to_bundle_id?: string;
    return_to_name?: string;
}

interface AppState {
    is_restricted: boolean;
    default_whitelist: string[];
    session_whitelist: string[];
    focus_started_at: number | null;
    free_activity_end_at: number | null;
}

interface AnalyticsSummary {
    total_focus_secs: number;
    total_break_secs: number;
    total_sessions: number;
    focus_sessions: number;
    break_sessions: number;
}

interface DailyTrendPoint {
    day: string;
    focus_secs: number;
    break_secs: number;
}

interface FocusHourBucket {
    hour: number;
    focus_secs: number;
    sessions: number;
}

interface AnalyticsData {
    summary: AnalyticsSummary;
    daily_trend: DailyTrendPoint[];
    focus_hour_distribution: FocusHourBucket[];
}

interface HistoryPage {
    items: SessionRecord[];
    has_more: boolean;
}

const HISTORY_PAGE_SIZE = 100;

// --- Helpers ---
function toggleSetItem(set: Set<string>, item: string): Set<string> {
    const s = new Set(set);
    if (s.has(item)) {
        s.delete(item);
    } else {
        s.add(item);
    }
    return s;
}

function appNameClass(name: string): string[] {
    const trimmed = name.trim();
    if (trimmed.length >= 18) {
        return ["app-item-name", "tiny"];
    }
    if (trimmed.length >= 12) {
        return ["app-item-name", "small"];
    }
    return ["app-item-name"];
}

const appIconCache = new Map<string, string | null>();

async function fetchAppIcon(bundleId: string): Promise<string | null> {
    if (appIconCache.has(bundleId)) {
        return appIconCache.get(bundleId) ?? null;
    }

    try {
        const icon = await invoke<string | null>("get_app_icon", { bundleId });
        const value = icon ?? null;
        appIconCache.set(bundleId, value);
        return value;
    } catch {
        appIconCache.set(bundleId, null);
        return null;
    }
}

// --- State ---
const currentView = ref<"planning" | "settings" | "analytics">("planning");
const taskDescription = ref("");
const runningApps = ref<AppInfo[]>([]);
const selectedApps = ref<Set<string>>(new Set());
const appState = ref<AppState>({
    is_restricted: true,
    default_whitelist: [],
    session_whitelist: [],
    focus_started_at: null,
    free_activity_end_at: null,
});

const isTaskInputShaking = ref(false);
const isTaskInputInvalid = ref(false);
const isBooting = ref(true);

// Focus session state
const elapsedSeconds = ref(0);
const showEndConfirm = ref(false);
const allowedAppNames = ref<AppInfo[]>([]);

const blockedAppState = ref<BlockedAppEvent | null>(null);
const returnCountdown = ref(3);

// Settings state
const settingsApps = ref<AppInfo[]>([]);
const settingsWhitelist = ref<Set<string>>(new Set());
const autostartEnabled = ref(false);
const autostartLoading = ref(false);

// History state
const sessionHistory = ref<SessionRecord[]>([]);
const historyOffset = ref(0);
const historyHasMore = ref(true);
const historyLoading = ref(false);
const analyticsData = ref<AnalyticsData | null>(null);
const analyticsLoading = ref(false);
const hoveredTrendIndex = ref<number | null>(null);

// Composables
const { snowEnabled, snowCanvas } = useSnowEffect();
const {
    showFreeActivityOptions,
    customMinutes,
    breakRemaining,
    isOnBreak,
    startFreeActivity,
} = useBreakTimer(appState);

// Event listeners
let unlistenState: UnlistenFn | null = null;
let unlistenBlocked: UnlistenFn | null = null;
let unlistenBlockedCleared: UnlistenFn | null = null;
let unlistenShowView: UnlistenFn | null = null;
let timerInterval: ReturnType<typeof setInterval> | null = null;
let countdownInterval: ReturnType<typeof setInterval> | null = null;

// Derived state
const isFocusing = computed(() => appState.value.focus_started_at !== null);

const recentTaskSuggestions = computed(() => {
    const seen = new Set<string>();

    return [...sessionHistory.value]
        .filter(
            (session) =>
                session.session_type === "focus" &&
                session.task &&
                session.task.trim().length > 0,
        )
        .sort((a, b) => b.started_at - a.started_at)
        .map((session) => session.task!.trim())
        .filter((task) => {
            if (seen.has(task)) return false;
            seen.add(task);
            return true;
        })
        .slice(0, 6);
});

const visibleRecentTasks = computed(() =>
    recentTaskSuggestions.value.slice(0, 3),
);
const hiddenRecentTasks = computed(() => recentTaskSuggestions.value.slice(3));
const recentTaskMenuItems = computed(() =>
    hiddenRecentTasks.value.map((task) => ({
        label: task,
        onSelect: () => applyRecentTask(task),
    })),
);

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

const maxDailyFocusSecs = computed(() => {
    const values = analyticsData.value?.daily_trend.map((point) => point.focus_secs) ?? [];
    return Math.max(1, ...values);
});

const maxHourFocusSecs = computed(() => {
    const values =
        analyticsData.value?.focus_hour_distribution.map((point) => point.focus_secs) ?? [];
    return Math.max(1, ...values);
});

const DAILY_TREND_PLOT_TOP = 8;
const DAILY_TREND_PLOT_BOTTOM = 92;

type DailyTrendChartPoint = {
    x: number;
    y: number;
    day: string;
    focusSecs: number;
};

const dailyTrendChartPoints = computed(() => {
    const trend = analyticsData.value?.daily_trend ?? [];
    if (trend.length === 0) {
        return [] as DailyTrendChartPoint[];
    }

    return trend.map((point, index) => {
        const x = trend.length === 1 ? 50 : (index / (trend.length - 1)) * 100;
        const normalized = Math.max(
            0,
            Math.min(1, point.focus_secs / maxDailyFocusSecs.value),
        );
        const y =
            DAILY_TREND_PLOT_TOP +
            (1 - normalized) * (DAILY_TREND_PLOT_BOTTOM - DAILY_TREND_PLOT_TOP);

        return {
            x,
            y,
            day: point.day,
            focusSecs: point.focus_secs,
        };
    });
});

function buildSmoothPath(points: DailyTrendChartPoint[]): string {
    if (points.length === 0) {
        return "";
    }

    if (points.length === 1) {
        return `M ${points[0].x} ${points[0].y}`;
    }

    let path = `M ${points[0].x} ${points[0].y}`;

    for (let index = 0; index < points.length - 1; index++) {
        const p0 = points[index - 1] ?? points[index];
        const p1 = points[index];
        const p2 = points[index + 1];
        const p3 = points[index + 2] ?? p2;

        const cp1x = p1.x + (p2.x - p0.x) / 6;
        const cp1y = p1.y + (p2.y - p0.y) / 6;
        const cp2x = p2.x - (p3.x - p1.x) / 6;
        const cp2y = p2.y - (p3.y - p1.y) / 6;

        path += ` C ${cp1x} ${cp1y}, ${cp2x} ${cp2y}, ${p2.x} ${p2.y}`;
    }

    return path;
}

const dailyTrendSmoothPath = computed(() =>
    buildSmoothPath(dailyTrendChartPoints.value),
);

const dailyTrendAreaPath = computed(() => {
    const points = dailyTrendChartPoints.value;
    if (points.length === 0) {
        return "";
    }

    const smoothLinePath = buildSmoothPath(points);
    const first = points[0];
    const last = points[points.length - 1];

    return `${smoothLinePath} L ${last.x} ${DAILY_TREND_PLOT_BOTTOM} L ${first.x} ${DAILY_TREND_PLOT_BOTTOM} Z`;
});

const activeTrendIndex = computed(() => {
    const points = dailyTrendChartPoints.value;
    if (points.length === 0) {
        return null;
    }

    if (hoveredTrendIndex.value === null) {
        return points.length - 1;
    }

    return Math.max(0, Math.min(points.length - 1, hoveredTrendIndex.value));
});

const activeTrendPoint = computed(() => {
    const index = activeTrendIndex.value;
    if (index === null) {
        return null;
    }

    return dailyTrendChartPoints.value[index] ?? null;
});

const dailyTrendLabelPoints = computed(() => {
    const points = dailyTrendChartPoints.value;
    if (points.length <= 6) {
        return points;
    }

    const step = Math.ceil(points.length / 6);
    return points.filter(
        (_point, index) => index % step === 0 || index === points.length - 1,
    );
});

function updateTrendHover(event: MouseEvent) {
    const points = dailyTrendChartPoints.value;
    const target = event.currentTarget as SVGSVGElement | null;

    if (!target || points.length === 0) {
        return;
    }

    const rect = target.getBoundingClientRect();
    if (rect.width <= 0) {
        return;
    }

    const ratio = (event.clientX - rect.left) / rect.width;
    const normalizedX = Math.max(0, Math.min(100, ratio * 100));

    let nearest = 0;
    let nearestDistance = Number.POSITIVE_INFINITY;

    points.forEach((point, index) => {
        const distance = Math.abs(point.x - normalizedX);
        if (distance < nearestDistance) {
            nearest = index;
            nearestDistance = distance;
        }
    });

    hoveredTrendIndex.value = nearest;
}

function clearTrendHover() {
    hoveredTrendIndex.value = null;
}

function formatDurationLabel(totalSeconds: number): string {
    const hours = Math.floor(totalSeconds / 3600);
    const mins = Math.floor((totalSeconds % 3600) / 60);
    if (hours > 0) {
        return `${hours}h ${mins}m`;
    }
    return `${mins}m`;
}

function formatDurationCompact(totalSeconds: number): string {
    const mins = Math.floor(totalSeconds / 60);
    if (mins >= 60) {
        const h = Math.floor(mins / 60);
        const m = mins % 60;
        return `${h}h${m > 0 ? `${m}m` : ""}`;
    }
    return `${mins}m`;
}

function formatHourLabel(hour: number): string {
    const startHour = ((hour % 24) + 24) % 24;
    const endHour = (startHour + 1) % 24;
    return `${String(startHour).padStart(2, "0")}:00~${String(endHour).padStart(2, "0")}:00`;
}

function clearBlockedState() {
    if (countdownInterval) {
        clearInterval(countdownInterval);
        countdownInterval = null;
    }
    blockedAppState.value = null;
    returnCountdown.value = 3;
}

// Watch focus state to start/stop timer
watch(
    () => appState.value.focus_started_at,
    (newVal) => {
        if (newVal) {
            if (timerInterval) clearInterval(timerInterval);
            updateTimer();
            timerInterval = setInterval(updateTimer, 1000);

            // Restore allowed apps list if empty (for UI persistence)
            if (
                allowedAppNames.value.length === 0 &&
                appState.value.session_whitelist.length > 0
            ) {
                loadApps().then(() => {
                    allowedAppNames.value = runningApps.value.filter((a) =>
                        appState.value.session_whitelist.includes(a.bundle_id),
                    );
                });
            }
        } else {
            if (timerInterval) clearInterval(timerInterval);
            timerInterval = null;
            elapsedSeconds.value = 0;
            showEndConfirm.value = false;
            clearBlockedState();
        }
    },
);

watch(taskDescription, (value) => {
    if (value.trim()) {
        isTaskInputInvalid.value = false;
    }
});

function applyRecentTask(task: string) {
    taskDescription.value = task;
    isTaskInputInvalid.value = false;
}

function updateTimer() {
    if (appState.value.focus_started_at) {
        const now = Math.floor(Date.now() / 1000);
        const start = appState.value.focus_started_at;
        elapsedSeconds.value = Math.max(0, now - start);
    }
}

async function hydrateIcons(target: "running" | "settings") {
    const list = target === "running" ? runningApps.value : settingsApps.value;

    for (const app of list) {
        if (app.icon_data_url) {
            continue;
        }

        const icon = await fetchAppIcon(app.bundle_id);
        if (!icon) {
            continue;
        }

        if (target === "running") {
            const matched = runningApps.value.find(
                (item) => item.bundle_id === app.bundle_id,
            );
            if (matched) {
                matched.icon_data_url = icon;
            }
        } else {
            const matched = settingsApps.value.find(
                (item) => item.bundle_id === app.bundle_id,
            );
            if (matched) {
                matched.icon_data_url = icon;
            }
        }
    }
}

// --- Lifecycle ---
onMounted(async () => {
    const startupStartedAt = Date.now();
    const minimumStartupAnimationMs = 220;

    try {
        await loadState();

        unlistenState = await listen<AppState>("state-changed", (event) => {
            appState.value = event.payload;
            void loadHistory(true);
        });

        unlistenBlocked = await listen<BlockedAppEvent>(
            "blocked-app",
            (event) => {
                blockedAppState.value = event.payload;
                startReturnCountdown();
            },
        );

        unlistenBlockedCleared = await listen("blocked-app-cleared", () => {
            clearBlockedState();
        });

        unlistenShowView = await listen<string>("show-view", (event) => {
            if (event.payload === "settings") {
                openSettings();
            } else {
                currentView.value = "planning";
            }
        });

        // Auto-select default whitelist apps
        initSelectedFromWhitelist();
        void loadHistory(true);
        void loadApps(false);
    } catch (e) {
        console.error("Failed during startup:", e);
    } finally {
        const elapsed = Date.now() - startupStartedAt;
        if (elapsed < minimumStartupAnimationMs) {
            await new Promise((resolve) =>
                setTimeout(resolve, minimumStartupAnimationMs - elapsed),
            );
        }
        isBooting.value = false;
    }
});

onUnmounted(() => {
    if (unlistenState) unlistenState();
    if (unlistenBlocked) unlistenBlocked();
    if (unlistenBlockedCleared) unlistenBlockedCleared();
    if (unlistenShowView) unlistenShowView();
    if (timerInterval) clearInterval(timerInterval);
    if (countdownInterval) clearInterval(countdownInterval);
});

// --- Functions ---
function initSelectedFromWhitelist() {
    const wl = appState.value.default_whitelist;
    if (wl.length > 0) {
        selectedApps.value = new Set(wl);
    }
}

async function openSettings() {
    currentView.value = "settings";
    autostartLoading.value = true;
    try {
        settingsApps.value = await invoke<AppInfo[]>("get_running_apps", {
            includeIcons: false,
        });
        void hydrateIcons("settings");
        settingsWhitelist.value = new Set(appState.value.default_whitelist);

        autostartEnabled.value = await isAutostartEnabled();
    } catch (e) {
        console.error("Failed to open settings:", e);
        autostartEnabled.value = false;
    } finally {
        autostartLoading.value = false;
    }
}

async function openAnalytics() {
    currentView.value = "analytics";
    analyticsLoading.value = true;
    try {
        analyticsData.value = await invoke<AnalyticsData>("get_analytics");
    } catch (e) {
        console.error("Failed to load analytics:", e);
        analyticsData.value = null;
    } finally {
        analyticsLoading.value = false;
    }
}

function toggleSettingsApp(bundleId: string) {
    settingsWhitelist.value = toggleSetItem(settingsWhitelist.value, bundleId);
}

async function saveSettings() {
    const whitelist = Array.from(settingsWhitelist.value);
    await invoke("update_settings", { defaultWhitelist: whitelist });
    appState.value.default_whitelist = whitelist;

    autostartLoading.value = true;
    try {
        if (autostartEnabled.value) {
            await enableAutostart();
        } else {
            await disableAutostart();
        }
    } catch (e) {
        console.error("Failed to update autostart:", e);
    } finally {
        autostartLoading.value = false;
    }

    currentView.value = "planning";
    initSelectedFromWhitelist();
}

function startReturnCountdown() {
    if (countdownInterval) {
        clearInterval(countdownInterval);
        countdownInterval = null;
    }

    if (!blockedAppState.value?.return_to_bundle_id) {
        returnCountdown.value = -1;
        return;
    }

    const returnBundleId = blockedAppState.value.return_to_bundle_id;

    returnCountdown.value = 3;
    countdownInterval = setInterval(() => {
        returnCountdown.value--;
        if (returnCountdown.value <= 0) {
            if (countdownInterval) clearInterval(countdownInterval);
            countdownInterval = null;
            invoke("switch_to_app", { bundleId: returnBundleId });
        }
    }, 1000);
}

async function loadState() {
    appState.value = await invoke<AppState>("get_state");
}

async function loadApps(includeIcons = false) {
    runningApps.value = await invoke<AppInfo[]>("get_running_apps", {
        includeIcons,
    });

    if (!includeIcons) {
        void hydrateIcons("running");
    }
}

function toggleApp(bundleId: string) {
    selectedApps.value = toggleSetItem(selectedApps.value, bundleId);
}

async function startFocus() {
    const task = taskDescription.value.trim();

    if (!task) {
        isTaskInputInvalid.value = true;
        isTaskInputShaking.value = true;
        setTimeout(() => (isTaskInputShaking.value = false), 500);
        return;
    }

    isTaskInputInvalid.value = false;

    const whitelist = Array.from(selectedApps.value);
    allowedAppNames.value = runningApps.value.filter((a) =>
        whitelist.includes(a.bundle_id),
    );
    await invoke("unlock_session", { whitelist, task });
}

function requestEndFocus() {
    showEndConfirm.value = true;
}

function cancelEndFocus() {
    showEndConfirm.value = false;
}

async function confirmEndFocus() {
    showEndConfirm.value = false;
    await invoke("lock_session");

    // Reset form, then restore default whitelist selection
    selectedApps.value = new Set();
    taskDescription.value = "";
    allowedAppNames.value = [];
    await loadApps();
    initSelectedFromWhitelist();
    await loadHistory(true);
}

async function loadHistory(reset = false) {
    if (historyLoading.value) {
        return;
    }

    if (reset) {
        historyOffset.value = 0;
        historyHasMore.value = true;
        sessionHistory.value = [];
    }

    if (!historyHasMore.value) {
        return;
    }

    historyLoading.value = true;

    try {
        const page = await invoke<HistoryPage>("get_history_page", {
            offset: historyOffset.value,
            limit: HISTORY_PAGE_SIZE,
        });

        sessionHistory.value = [...sessionHistory.value, ...page.items];
        historyOffset.value += page.items.length;
        historyHasMore.value = page.has_more;
    } catch (e) {
        console.error("Failed to load history:", e);
    } finally {
        historyLoading.value = false;
    }
}

function loadMoreHistory() {
    void loadHistory(false);
}
</script>

<template>
    <div class="overlay-container">
        <canvas ref="snowCanvas" class="snow-canvas" v-show="snowEnabled"></canvas>

        <UCard v-if="isBooting" class="w-[min(420px,86vw)]">
            <div class="space-y-3 text-center">
                <div class="startup-spinner" aria-hidden="true"></div>
                <h1 class="text-xl font-semibold">Focus Must</h1>
                <UProgress :model-value="null" size="sm" />
                <p class="text-sm text-muted">正在加载应用列表...</p>
            </div>
        </UCard>

        <UCard
            v-else-if="!isFocusing && currentView === 'planning'"
            class="flex h-[70vh] w-[min(1040px,92vw)] flex-col overflow-hidden"
            :ui="{ body: 'flex-1 min-h-0 overflow-hidden' }"
        >
            <template #header>
                <div class="flex items-start justify-between gap-3">
                    <div class="flex min-w-0 items-center gap-2">
                        <UIcon name="i-lucide-lock" class="text-3xl text-primary" />
                        <h1 class="text-xl font-semibold leading-tight">Focus Must：先想清楚要做什么，再开始</h1>
                    </div>
                    <div class="flex flex-wrap gap-2">
                        <UButton
                            color="neutral"
                            variant="outline"
                            size="sm"
                            @click="snowEnabled = !snowEnabled"
                            :leading-icon="snowEnabled ? 'i-lucide-snowflake' : 'i-lucide-moon-star'"
                        >
                            {{ snowEnabled ? "下雪中" : "下雪" }}
                        </UButton>
                        <UButton
                            color="neutral"
                            variant="outline"
                            size="sm"
                            leading-icon="i-lucide-settings-2"
                            @click="openSettings"
                        >
                            设置
                        </UButton>
                        <UButton
                            color="neutral"
                            variant="outline"
                            size="sm"
                            leading-icon="i-lucide-chart-column"
                            @click="openAnalytics"
                        >
                            统计
                        </UButton>
                        <UColorModeSelect size="sm" class="min-w-28" />
                    </div>
                </div>
            </template>

            <div class="grid h-full min-h-0 flex-1 gap-4 lg:grid-cols-[minmax(0,1fr)_340px]">
                <div class="min-h-0 flex flex-col">
                    <div class="min-h-0 flex-1 space-y-4 overflow-y-auto pr-1">
                        <UCard variant="soft">
                            <template #header>
                                <div class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                                    <UIcon name="i-lucide-clipboard-list" class="text-base" />
                                    <span>接下来做什么</span>
                                </div>
                            </template>
                            <div class="space-y-3">
                                <UTextarea
                                    v-model="taskDescription"
                                    :rows="3"
                                    autoresize
                                    :color="isTaskInputInvalid ? 'error' : 'success'"
                                    :highlight="isTaskInputInvalid"
                                    placeholder="描述你接下来要完成的任务..."
                                    @focus="isTaskInputInvalid = false"
                                    :class="['w-full', isTaskInputShaking ? 'shake' : '']"
                                />

                                <div v-if="recentTaskSuggestions.length > 0" class="flex items-center gap-2">
                                    <div class="shrink-0 text-xs text-muted">最近任务</div>
                                    <div class="flex flex-wrap gap-2">
                                        <UButton
                                            v-for="task in visibleRecentTasks"
                                            :key="task"
                                            color="neutral"
                                            variant="outline"
                                            size="xs"
                                            class="max-w-full truncate"
                                            @click="applyRecentTask(task)"
                                        >
                                            {{ task }}
                                        </UButton>
                                        <UDropdownMenu
                                            v-if="hiddenRecentTasks.length > 0"
                                            :items="recentTaskMenuItems"
                                        >
                                            <UButton color="neutral" variant="soft" size="xs">
                                                更多 ({{ hiddenRecentTasks.length }})
                                            </UButton>
                                        </UDropdownMenu>
                                    </div>
                                </div>
                            </div>
                        </UCard>

                        <UCard variant="soft">
                            <template #header>
                                <div class="flex items-center justify-between gap-2">
                                    <div class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                                        <UIcon name="i-lucide-layout-grid" class="text-base" />
                                        <span>需要用到的 APP</span>
                                    </div>
                                    <UButton color="neutral" variant="outline" size="xs" @click="loadApps()">
                                        刷新
                                    </UButton>
                                </div>
                            </template>

                            <div class="app-grid">
                                <UCard
                                    v-for="app in runningApps"
                                    :key="app.bundle_id"
                                    variant="outline"
                                    :class="['app-item', { selected: selectedApps.has(app.bundle_id) }]"
                                    @click="toggleApp(app.bundle_id)"
                                >
                                    <div class="app-item-icon-placeholder" :class="{ 'has-image': !!app.icon_data_url }">
                                        <img
                                            v-if="app.icon_data_url"
                                            :src="app.icon_data_url"
                                            :alt="app.name"
                                            class="app-item-icon-image"
                                        />
                                        <span v-else>{{ app.name ? app.name[0].toUpperCase() : "?" }}</span>
                                    </div>
                                    <div :class="appNameClass(app.name)">{{ app.name }}</div>
                                </UCard>

                                <UAlert
                                    v-if="runningApps.length === 0"
                                    color="neutral"
                                    variant="soft"
                                    title="没有检测到其他运行中的应用"
                                    class="col-span-full"
                                />
                            </div>
                        </UCard>
                    </div>

                    <div class="mt-3 shrink-0 space-y-2">
                        <template v-if="isOnBreak">
                            <UButton color="neutral" variant="soft" block disabled leading-icon="i-lucide-coffee">
                                休息中 {{ breakRemaining }}
                            </UButton>
                        </template>

                        <template v-else>
                            <UButton
                                v-if="!showFreeActivityOptions"
                                color="neutral"
                                variant="outline"
                                block
                                leading-icon="i-lucide-coffee"
                                @click="showFreeActivityOptions = true"
                            >
                                休息一下 (自由活动)
                            </UButton>

                            <div v-else class="grid grid-cols-3 gap-2 sm:grid-cols-6">
                                <UButton color="neutral" variant="outline" size="sm" @click="startFreeActivity(5)">
                                    5分
                                </UButton>
                                <UButton color="neutral" variant="outline" size="sm" @click="startFreeActivity(10)">
                                    10分
                                </UButton>
                                <UButton color="neutral" variant="outline" size="sm" @click="startFreeActivity(15)">
                                    15分
                                </UButton>
                                <UButton color="neutral" variant="outline" size="sm" @click="startFreeActivity(30)">
                                    30分
                                </UButton>
                                <UButton color="neutral" variant="outline" size="sm" @click="startFreeActivity(45)">
                                    45分
                                </UButton>
                                <UInput
                                    v-model="customMinutes"
                                    type="number"
                                    min="1"
                                    max="480"
                                    placeholder="自定义"
                                    @keyup.enter="
                                        customMinutes &&
                                        startFreeActivity(Number(customMinutes))
                                    "
                                />
                            </div>
                        </template>

                        <UButton
                            color="success"
                            variant="solid"
                            block
                            leading-icon="i-lucide-rocket"
                            @click="startFocus"
                        >
                            开始专注
                        </UButton>
                    </div>
                </div>

                <UCard
                    variant="outline"
                    class="h-full min-h-0 overflow-hidden"
                    :ui="{ body: 'h-full min-h-0 overflow-hidden' }"
                >
                    <HistoryList
                        :sessions="sessionHistory"
                        :has-more="historyHasMore"
                        :is-loading="historyLoading"
                        @load-more="loadMoreHistory"
                    />
                </UCard>
            </div>
        </UCard>

        <UCard v-else-if="!isFocusing && currentView === 'settings'" class="w-[min(980px,92vw)] max-h-[88vh] overflow-hidden">
            <template #header>
                <div class="space-y-1">
                    <div class="flex min-w-0 items-center gap-2">
                        <UIcon name="i-lucide-settings-2" class="text-3xl text-primary" />
                        <h1 class="text-xl font-semibold leading-tight">设置</h1>
                    </div>
                </div>
            </template>

            <div class="space-y-4 overflow-y-auto max-h-[62vh]">
                <UCard variant="soft">
                    <template #header>
                        <div class="flex items-start justify-between gap-2">
                            <div>
                                <div class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                                    <UIcon name="i-lucide-layout-grid" class="text-base" />
                                    <span>默认允许的 APP</span>
                                </div>
                                <p class="text-xs text-muted">配置默认白名单 APP，每次专注时自动选择</p>
                            </div>
                            <UButton color="neutral" variant="outline" size="xs" @click="openSettings">
                                刷新
                            </UButton>
                        </div>
                    </template>

                    <div class="app-grid">
                        <UCard
                            v-for="app in settingsApps"
                            :key="app.bundle_id"
                            variant="outline"
                            :class="['app-item', { selected: settingsWhitelist.has(app.bundle_id) }]"
                            @click="toggleSettingsApp(app.bundle_id)"
                        >
                            <div class="app-item-icon-placeholder" :class="{ 'has-image': !!app.icon_data_url }">
                                <img
                                    v-if="app.icon_data_url"
                                    :src="app.icon_data_url"
                                    :alt="app.name"
                                    class="app-item-icon-image"
                                />
                                <span v-else>{{ app.name ? app.name[0].toUpperCase() : "?" }}</span>
                            </div>
                            <div :class="appNameClass(app.name)">{{ app.name }}</div>
                        </UCard>

                        <UAlert
                            v-if="settingsApps.length === 0"
                            color="neutral"
                            variant="soft"
                            title="没有检测到其他运行中的应用"
                            class="col-span-full"
                        />
                    </div>
                </UCard>

                <UCard variant="soft">
                    <div class="flex items-center justify-between gap-3">
                        <div>
                            <p class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                                <UIcon name="i-lucide-power" class="text-base" />
                                <span>开机启动</span>
                            </p>
                            <p class="text-xs text-muted">开机自动启动 Focus Must</p>
                        </div>
                        <USwitch v-model="autostartEnabled" :disabled="autostartLoading" />
                    </div>
                </UCard>
            </div>

            <template #footer>
                <div class="flex gap-2">
                    <UButton
                        color="neutral"
                        variant="outline"
                        class="flex-1 justify-center text-center"
                        @click="currentView = 'planning'"
                    >
                        <UIcon name="i-lucide-arrow-left" class="text-base" />
                        返回
                    </UButton>
                    <UButton
                        color="success"
                        variant="solid"
                        class="flex-1 justify-center text-center"
                        @click="saveSettings"
                    >
                        <UIcon name="i-lucide-save" class="text-base" />
                        保存设置
                    </UButton>
                </div>
            </template>
        </UCard>

        <UCard
            v-else-if="!isFocusing && currentView === 'analytics'"
            class="flex h-[86vh] w-[min(1080px,94vw)] flex-col overflow-hidden"
            :ui="{ body: 'flex-1 min-h-0 overflow-hidden' }"
        >
            <template #header>
                <div class="space-y-1">
                    <div class="flex min-w-0 items-center gap-2">
                        <UIcon name="i-lucide-chart-column" class="text-3xl text-primary" />
                        <h1 class="text-xl font-semibold leading-tight">统计分析</h1>
                    </div>
                    <p class="text-sm text-muted">专注与休息数据总览</p>
                </div>
            </template>

            <div class="h-full min-h-0 space-y-3">
                <UAlert
                    v-if="analyticsLoading"
                    color="neutral"
                    variant="soft"
                    title="正在计算统计数据..."
                />

                <template v-else-if="analyticsData">
                    <div class="grid gap-2.5 sm:grid-cols-3">
                        <UCard variant="soft">
                            <div class="analytics-metric-card">
                                <div class="analytics-metric-main">
                                    <UIcon name="i-lucide-timer" class="analytics-metric-icon" />
                                    <div class="analytics-metric-value-group">
                                        <p class="analytics-metric-label">总专注时长</p>
                                        <p class="analytics-metric-value">
                                            {{ formatDurationLabel(analyticsData.summary.total_focus_secs) }}
                                        </p>
                                    </div>
                                </div>
                            </div>
                        </UCard>
                        <UCard variant="soft">
                            <div class="analytics-metric-card">
                                <div class="analytics-metric-main">
                                    <UIcon name="i-lucide-coffee" class="analytics-metric-icon" />
                                    <div class="analytics-metric-value-group">
                                        <p class="analytics-metric-label">总休息时长</p>
                                        <p class="analytics-metric-value">
                                            {{ formatDurationLabel(analyticsData.summary.total_break_secs) }}
                                        </p>
                                    </div>
                                </div>
                            </div>
                        </UCard>
                        <UCard variant="soft">
                            <div class="analytics-metric-card">
                                <div class="analytics-metric-main">
                                    <UIcon name="i-lucide-list" class="analytics-metric-icon" />
                                    <div class="analytics-metric-value-group">
                                        <p class="analytics-metric-label">总会话数</p>
                                        <p class="analytics-metric-value">
                                            {{ analyticsData.summary.total_sessions }}
                                        </p>
                                    </div>
                                </div>
                            </div>
                        </UCard>
                    </div>

                    <UCard variant="soft">
                        <template #header>
                            <p class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                                <UIcon name="i-lucide-chart-line" class="text-base" />
                                <span>日趋势（近 30 天）</span>
                            </p>
                        </template>
                        <div v-if="dailyTrendChartPoints.length > 0" class="space-y-3">
                            <div class="daily-trend-chart-wrap">
                                <svg
                                    class="daily-trend-chart"
                                    viewBox="0 0 100 100"
                                    preserveAspectRatio="none"
                                    role="img"
                                    aria-label="近30天专注时长趋势"
                                    @mousemove="updateTrendHover"
                                    @mouseleave="clearTrendHover"
                                >
                                    <defs>
                                        <linearGradient id="dailyTrendArea" x1="0" y1="0" x2="0" y2="1">
                                            <stop offset="0%" stop-color="rgb(34 197 94 / 0.35)" />
                                            <stop offset="100%" stop-color="rgb(34 197 94 / 0.04)" />
                                        </linearGradient>
                                    </defs>
                                    <line
                                        v-for="point in dailyTrendChartPoints"
                                        :key="`grid-${point.day}`"
                                        :x1="point.x"
                                        :y1="DAILY_TREND_PLOT_TOP"
                                        :x2="point.x"
                                        :y2="DAILY_TREND_PLOT_BOTTOM"
                                        class="daily-trend-grid-line"
                                    />
                                    <path :d="dailyTrendAreaPath" fill="url(#dailyTrendArea)" />
                                    <path
                                        :d="dailyTrendSmoothPath"
                                        fill="none"
                                        stroke="rgb(34 197 94)"
                                        stroke-width="0.35"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                    />
                                    <line
                                        v-if="activeTrendPoint"
                                        :x1="activeTrendPoint.x"
                                        :y1="DAILY_TREND_PLOT_TOP"
                                        :x2="activeTrendPoint.x"
                                        :y2="DAILY_TREND_PLOT_BOTTOM"
                                        class="daily-trend-focus-line"
                                    />
                                </svg>

                                <div
                                    v-if="activeTrendPoint"
                                    class="daily-trend-dot"
                                    :style="{ left: `${activeTrendPoint.x}%`, top: `${activeTrendPoint.y}%` }"
                                ></div>

                                <div
                                    v-if="activeTrendPoint"
                                    class="daily-trend-tooltip"
                                    :style="{ left: `${activeTrendPoint.x}%` }"
                                >
                                    <p class="daily-trend-tooltip-date">{{ activeTrendPoint.day }}</p>
                                    <p class="daily-trend-tooltip-value">
                                        {{ formatDurationCompact(activeTrendPoint.focusSecs) }}
                                    </p>
                                </div>
                            </div>

                            <div class="flex items-center justify-between gap-2">
                                <div
                                    v-for="point in dailyTrendLabelPoints"
                                    :key="`label-${point.day}`"
                                    class="min-w-0 text-center"
                                >
                                    <p class="text-[10px] text-muted">{{ point.day.slice(5) }}</p>
                                    <p class="text-[10px] font-medium">{{ formatDurationCompact(point.focusSecs) }}</p>
                                </div>
                            </div>
                        </div>

                        <div v-else class="py-6 text-center text-xs text-muted">
                            暂无趋势数据
                        </div>
                    </UCard>

                    <UCard variant="soft">
                        <template #header>
                            <p class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                                <UIcon name="i-lucide-clock-3" class="text-base" />
                                <span>专注时段分布</span>
                            </p>
                        </template>
                        <div class="grid grid-cols-2 gap-x-6 gap-y-1.5">
                            <div
                                v-for="bucket in analyticsData.focus_hour_distribution"
                                :key="bucket.hour"
                                class="grid grid-cols-[100px_1fr_64px] items-center gap-1"
                            >
                                <span class="hour-range-chip">{{ formatHourLabel(bucket.hour) }}</span>
                                <UProgress :model-value="bucket.focus_secs" :max="maxHourFocusSecs" size="xs" />
                                <span class="text-[11px] text-right text-muted whitespace-nowrap tabular-nums">{{ formatDurationCompact(bucket.focus_secs) }}</span>
                            </div>
                        </div>
                    </UCard>
                </template>
            </div>

            <template #footer>
                <div class="flex justify-end">
                    <UButton
                        color="neutral"
                        variant="outline"
                        leading-icon="i-lucide-arrow-left"
                        @click="currentView = 'planning'"
                    >
                        返回
                    </UButton>
                </div>
            </template>
        </UCard>

        <UCard v-else-if="isFocusing" class="w-[min(760px,90vw)] text-center">
            <div v-if="blockedAppState" class="space-y-4">
                <UIcon name="i-lucide-circle-x" class="mx-auto block text-6xl text-error" />
                <UAlert color="error" variant="soft" title="检测到分心">
                    <template #description>
                        你打开了 <strong>{{ blockedAppState.name }}</strong>
                    </template>
                </UAlert>

                <template v-if="blockedAppState.return_to_bundle_id">
                    <UBadge color="error" variant="soft" class="px-4 py-2 text-base">
                        {{ returnCountdown }} 秒后返回
                    </UBadge>
                    <UAlert color="neutral" variant="outline">
                        <template #description>
                            正在带你回到 <strong>{{ blockedAppState.return_to_name }}</strong> ...
                        </template>
                    </UAlert>
                </template>

                <UAlert v-else color="warning" variant="soft">
                    <template #description>
                        <span class="inline-flex items-center gap-1.5">
                            <UIcon name="i-lucide-triangle-alert" class="text-base" />
                            <span>请使用 <strong>⌘+Tab</strong> 手动切换回工作 App</span>
                        </span>
                    </template>
                </UAlert>
            </div>

            <div v-else class="space-y-5">
                <UIcon name="i-lucide-brain" class="mx-auto block text-6xl text-primary" />
                <h1 class="text-2xl font-semibold">保持专注</h1>

                <div class="timer-display">{{ formattedTime }}</div>

                <UAlert color="neutral" variant="soft">
                    <template #description>
                        {{ taskDescription || "正在完成一项重要的任务..." }}
                    </template>
                </UAlert>

                <div v-if="allowedAppNames.length > 0" class="flex flex-wrap justify-center gap-2">
                    <UBadge
                        v-for="app in allowedAppNames"
                        :key="app.bundle_id"
                        color="success"
                        variant="soft"
                        :label="app.name"
                    />
                </div>

                <UAlert color="neutral" variant="outline">
                    <template #description>
                        试图打开其他应用时，窗口会再次出现提醒你。
                    </template>
                </UAlert>

                <div class="flex justify-center">
                    <UButton
                        color="neutral"
                        variant="outline"
                        leading-icon="i-lucide-square"
                        @click="requestEndFocus"
                    >
                        结束专注
                    </UButton>
                </div>
            </div>

            <UModal
                v-model:open="showEndConfirm"
                title="结束专注"
                description="确定要结束本次专注吗？"
            >
                <template #footer>
                    <div class="flex w-full justify-end gap-2">
                        <UButton color="neutral" variant="outline" @click="cancelEndFocus">
                            取消
                        </UButton>
                        <UButton color="primary" @click="confirmEndFocus">确定结束</UButton>
                    </div>
                </template>
            </UModal>
        </UCard>
    </div>
</template>

<style scoped>
.snow-canvas {
    position: fixed;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 100000;
}

.overlay-container {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    background:
        radial-gradient(circle at 18% 14%, rgba(59, 130, 246, 0.08), transparent 42%),
        radial-gradient(circle at 82% 86%, rgba(16, 185, 129, 0.06), transparent 44%),
        linear-gradient(135deg, rgba(255, 255, 255, 0.05), rgba(255, 255, 255, 0.01)),
        rgba(20, 28, 44, 0.12);
    backdrop-filter: blur(22px) saturate(130%);
    -webkit-backdrop-filter: blur(22px) saturate(130%);
}

:global(html.light) .overlay-container {
    background:
        radial-gradient(circle at 18% 14%, rgba(59, 130, 246, 0.035), transparent 42%),
        radial-gradient(circle at 82% 86%, rgba(16, 185, 129, 0.03), transparent 44%),
        linear-gradient(135deg, rgba(255, 255, 255, 0.08), rgba(255, 255, 255, 0.015)),
        rgba(236, 245, 255, 0.14);
}

.startup-spinner {
    width: 52px;
    height: 52px;
    margin: 0 auto;
    border-radius: 50%;
    border: 3px solid rgba(255, 255, 255, 0.2);
    border-top-color: rgba(16, 185, 129, 0.95);
    animation: spin 0.9s linear infinite;
}

.shake {
    animation: shake 0.5s cubic-bezier(0.36, 0.07, 0.19, 0.97) both;
}

.timer-display {
    font-size: 56px;
    font-weight: 700;
    font-feature-settings: "tnum";
    font-variant-numeric: tabular-nums;
    letter-spacing: 2px;
}

.analytics-metric-card {
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-height: 74px;
}

.analytics-metric-label {
    margin: 0;
    font-size: 11px;
    line-height: 1.1;
    color: color-mix(in oklab, currentColor 60%, transparent);
}

.analytics-metric-main {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
}

.analytics-metric-value-group {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 5px;
}

.analytics-metric-value {
    margin: 0;
    font-size: 21px;
    font-weight: 700;
    line-height: 1;
}

.analytics-metric-icon {
    font-size: 40px;
    line-height: 1;
    color: rgb(34 197 94);
}

.hour-range-chip {
    display: inline-flex;
    width: 100%;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    background: color-mix(in oklab, rgb(34 197 94) 14%, transparent);
    padding: 2px 6px;
    font-size: 11px;
    font-weight: 600;
    line-height: 1.2;
    color: color-mix(in oklab, rgb(34 197 94) 72%, currentColor);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
}

.daily-trend-chart-wrap {
    position: relative;
    height: 136px;
    width: 100%;
    border-radius: 12px;
    padding: 10px;
    border: 1px solid rgba(148, 163, 184, 0.2);
    background: rgba(148, 163, 184, 0.06);
}

.daily-trend-chart {
    width: 100%;
    height: 100%;
    display: block;
}

.daily-trend-grid-line {
    stroke: rgba(148, 163, 184, 0.2);
    stroke-width: 0.35;
}

.daily-trend-focus-line {
    stroke: rgba(16, 185, 129, 0.7);
    stroke-width: 0.45;
    stroke-dasharray: 1.4 1.4;
}

.daily-trend-dot {
    position: absolute;
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: rgb(16 185 129);
    border: 2px solid rgb(255 255 255 / 0.92);
    transform: translate(-50%, -50%);
    box-shadow: 0 0 0 1px rgb(16 185 129 / 0.2);
    pointer-events: none;
}

.daily-trend-tooltip {
    position: absolute;
    top: 8px;
    transform: translateX(-50%);
    border-radius: 10px;
    border: 1px solid rgba(148, 163, 184, 0.28);
    background: rgba(15, 23, 42, 0.86);
    padding: 6px 8px;
    pointer-events: none;
    min-width: 82px;
}

.daily-trend-tooltip-date {
    font-size: 10px;
    color: rgba(226, 232, 240, 0.82);
    line-height: 1.1;
}

.daily-trend-tooltip-value {
    margin-top: 2px;
    font-size: 11px;
    font-weight: 600;
    color: rgba(240, 253, 244, 0.96);
    line-height: 1.1;
}

.app-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(72px, 1fr));
    justify-content: stretch;
    align-content: start;
    gap: 8px;
    max-height: 320px;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 2px 6px;
}

.app-item {
    width: 100%;
    min-width: 0;
    aspect-ratio: 1 / 1;
    background: rgba(255, 255, 255, 0.03);
    cursor: pointer;
    user-select: none;
    transition: transform 0.2s ease;
}

.app-item:hover {
    transform: translateY(-1px);
}

.app-item.selected {
    border-color: rgba(16, 185, 129, 0.95);
    background: rgba(16, 185, 129, 0.16);
    box-shadow:
        0 0 0 1px rgba(16, 185, 129, 0.42),
        0 8px 16px rgba(16, 185, 129, 0.18);
}

.app-item :deep([data-slot="body"]) {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 4px;
    padding: 6px;
}

.app-item-icon-placeholder {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 600;
    color: rgba(148, 163, 184, 1);
    background: rgba(255, 255, 255, 0.05);
    overflow: hidden;
}

.app-item-icon-placeholder.has-image {
    background: transparent;
}

.app-item-icon-image {
    width: 100%;
    height: 100%;
    border-radius: 8px;
    object-fit: cover;
    display: block;
}

@media (prefers-color-scheme: light) {
    .app-item.selected {
        border-color: rgba(5, 150, 105, 1);
        background: rgba(16, 185, 129, 0.22);
        box-shadow:
            0 0 0 1px rgba(5, 150, 105, 0.48),
            0 10px 18px rgba(5, 150, 105, 0.2);
    }
}

.app-item-name {
    font-size: 10px;
    font-weight: 500;
    line-height: 1.2;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    text-overflow: ellipsis;
    word-break: break-word;
    max-width: 100%;
    text-align: center;
}

.app-item-name.small {
    font-size: 9px;
}

.app-item-name.tiny {
    font-size: 8px;
    line-height: 1.1;
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}

@keyframes shake {
    10%,
    90% {
        transform: translate3d(-1px, 0, 0);
    }

    20%,
    80% {
        transform: translate3d(2px, 0, 0);
    }

    30%,
    50%,
    70% {
        transform: translate3d(-4px, 0, 0);
    }

    40%,
    60% {
        transform: translate3d(4px, 0, 0);
    }
}
</style>
