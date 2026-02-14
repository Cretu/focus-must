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
const showMoreRecentTasks = ref(false);

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
const analyticsData = ref<AnalyticsData | null>(null);
const analyticsLoading = ref(false);

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
    return `${String(hour).padStart(2, "0")}:00`;
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
            loadHistory();
        });

        unlistenBlocked = await listen<BlockedAppEvent>(
            "blocked-app",
            (event) => {
                blockedAppState.value = event.payload;
                startReturnCountdown();
            },
        );

        unlistenShowView = await listen<string>("show-view", (event) => {
            if (event.payload === "settings") {
                openSettings();
            } else {
                currentView.value = "planning";
            }
        });

        // Auto-select default whitelist apps
        initSelectedFromWhitelist();
        loadHistory();
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
            blockedAppState.value = null;
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
    loadHistory();
}

async function loadHistory() {
    try {
        sessionHistory.value = await invoke<SessionRecord[]>("get_history");
    } catch (e) {
        console.error("Failed to load history:", e);
    }
}
</script>

<template>
    <div class="overlay-container">
        <!-- Snow Canvas -->
        <canvas
            ref="snowCanvas"
            class="snow-canvas"
            v-show="snowEnabled"
        ></canvas>

        <div v-if="isBooting" class="overlay-card startup-card">
            <div class="startup-spinner" aria-hidden="true"></div>
            <h1 class="overlay-title">Focus Must</h1>
            <p class="startup-subtitle">正在加载应用列表...</p>
        </div>

        <!-- PLANNING MODE -->
        <div
            v-else-if="!isFocusing && currentView === 'planning'"
            :class="[
                'overlay-card',
                'planning-card-layout',
                'relative-position',
            ]"
        >
            <div class="planning-content">
                <!-- Top-right toggle group -->
                <div class="settings-toggle-group">
                    <label
                        class="snow-toggle"
                        @click.prevent="snowEnabled = !snowEnabled"
                    >
                        <span class="snow-toggle-icon">{{
                            snowEnabled ? "❄️" : "🌙"
                        }}</span>
                        <span class="snow-toggle-label">{{
                            snowEnabled ? "下雪中" : "下雪"
                        }}</span>
                    </label>
                    <label class="snow-toggle" @click.prevent="openSettings">
                        <span class="snow-toggle-icon">⚙️</span>
                        <span class="snow-toggle-label">设置</span>
                    </label>
                    <label class="snow-toggle" @click.prevent="openAnalytics">
                        <span class="snow-toggle-icon">📊</span>
                        <span class="snow-toggle-label">统计</span>
                    </label>
                </div>
                <div class="lock-icon">🔒</div>
                <h1 class="overlay-title">
                    Focus Must：先想清楚要做什么，再开始
                </h1>

                <!-- Task Input -->
                <div class="section">
                    <div class="section-label">📝 接下来做什么</div>
                    <textarea
                        class="task-input"
                        :class="{
                            shake: isTaskInputShaking,
                            'is-invalid': isTaskInputInvalid,
                            'task-input-main': true,
                        }"
                        v-model="taskDescription"
                        placeholder="描述你接下来要完成的任务..."
                    ></textarea>

                    <div
                        v-if="recentTaskSuggestions.length > 0"
                        class="recent-task-block"
                    >
                        <div class="recent-task-label">最近任务</div>
                        <div class="recent-task-list">
                            <button
                                v-for="task in visibleRecentTasks"
                                :key="task"
                                type="button"
                                class="recent-task-chip"
                                @click="applyRecentTask(task)"
                            >
                                {{ task }}
                            </button>
                            <div
                                v-if="hiddenRecentTasks.length > 0"
                                class="recent-task-more-wrapper"
                            >
                                <button
                                    type="button"
                                    class="recent-task-chip recent-task-more-btn"
                                    @click="
                                        showMoreRecentTasks =
                                            !showMoreRecentTasks
                                    "
                                >
                                    更多 ({{ hiddenRecentTasks.length }})
                                </button>
                                <div
                                    v-if="showMoreRecentTasks"
                                    class="recent-task-dropdown"
                                >
                                    <button
                                        v-for="task in hiddenRecentTasks"
                                        :key="task"
                                        type="button"
                                        class="recent-task-dropdown-item"
                                        @click="
                                            applyRecentTask(task);
                                            showMoreRecentTasks = false;
                                        "
                                    >
                                        {{ task }}
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- App Whitelist -->
                <div class="section">
                    <div class="section-header">
                        <span class="section-label">📱 需要用到的 APP</span>
                        <button
                            class="btn btn-ghost btn-refresh"
                            @click="loadApps()"
                        >
                            刷新
                        </button>
                    </div>
                    <div class="app-list">
                        <div
                            v-for="app in runningApps"
                            :key="app.bundle_id"
                            class="app-item"
                            :class="{
                                selected: selectedApps.has(app.bundle_id),
                            }"
                            @click="toggleApp(app.bundle_id)"
                        >
                            <div
                                class="app-item-icon-placeholder"
                                :class="{ 'has-image': !!app.icon_data_url }"
                            >
                                <img
                                    v-if="app.icon_data_url"
                                    :src="app.icon_data_url"
                                    :alt="app.name"
                                    class="app-item-icon-image"
                                />
                                <span v-else>{{
                                    app.name ? app.name[0].toUpperCase() : "?"
                                }}</span>
                            </div>
                            <div :class="appNameClass(app.name)">{{ app.name }}</div>
                        </div>

                        <p
                            v-if="runningApps.length === 0"
                            class="empty-list-message"
                        >
                            没有检测到其他运行中的应用
                        </p>
                    </div>
                </div>

                <div class="planning-actions">
                    <div class="break-options-container">
                        <!-- On break: show countdown -->
                        <button
                            v-if="isOnBreak"
                            class="btn btn-ghost btn-full-width btn-disabled"
                            disabled
                        >
                            ☕️ 休息中 {{ breakRemaining }}
                        </button>

                        <!-- Not on break: normal toggle -->
                        <template v-else>
                            <button
                                v-if="!showFreeActivityOptions"
                                class="btn btn-ghost btn-full-width"
                                @click="showFreeActivityOptions = true"
                            >
                                ☕️ 休息一下 (自由活动)
                            </button>

                            <div v-else class="duration-options-grid">
                                <button
                                    class="btn btn-ghost btn-duration"
                                    @click="startFreeActivity(5)"
                                >
                                    5分
                                </button>
                                <button
                                    class="btn btn-ghost btn-duration"
                                    @click="startFreeActivity(10)"
                                >
                                    10分
                                </button>
                                <button
                                    class="btn btn-ghost btn-duration"
                                    @click="startFreeActivity(15)"
                                >
                                    15分
                                </button>
                                <button
                                    class="btn btn-ghost btn-duration"
                                    @click="startFreeActivity(30)"
                                >
                                    30分
                                </button>
                                <button
                                    class="btn btn-ghost btn-duration"
                                    @click="startFreeActivity(45)"
                                >
                                    45分
                                </button>
                                <input
                                    v-model="customMinutes"
                                    type="number"
                                    min="1"
                                    max="480"
                                    placeholder="自定义"
                                    class="task-input custom-duration-input"
                                    @keyup.enter="
                                        customMinutes &&
                                        startFreeActivity(Number(customMinutes))
                                    "
                                />
                            </div>
                        </template>
                    </div>

                    <button
                        class="btn btn-success btn-full-width"
                        @click="startFocus"
                    >
                        🚀 开始专注
                    </button>
                </div>
            </div>

            <div class="history-side-panel">
                <HistoryList :sessions="sessionHistory" />
            </div>
        </div>

        <!-- SETTINGS MODE -->
        <div
            v-else-if="!isFocusing && currentView === 'settings'"
            class="overlay-card"
        >
            <div class="lock-icon">⚙️</div>
            <h1 class="overlay-title">设置</h1>
            <p class="overlay-subtitle">
                配置默认白名单 APP，每次专注时自动选择
            </p>

            <div class="section">
                <div class="section-header">
                    <span class="section-label">📱 默认允许的 APP</span>
                    <button
                        class="btn btn-ghost btn-refresh"
                        @click="openSettings"
                    >
                        刷新
                    </button>
                </div>
                <div class="app-list">
                    <div
                        v-for="app in settingsApps"
                        :key="app.bundle_id"
                        class="app-item"
                        :class="{
                            selected: settingsWhitelist.has(app.bundle_id),
                        }"
                        @click="toggleSettingsApp(app.bundle_id)"
                    >
                        <div
                            class="app-item-icon-placeholder"
                            :class="{ 'has-image': !!app.icon_data_url }"
                        >
                            <img
                                v-if="app.icon_data_url"
                                :src="app.icon_data_url"
                                :alt="app.name"
                                class="app-item-icon-image"
                            />
                            <span v-else>{{
                                app.name ? app.name[0].toUpperCase() : "?"
                            }}</span>
                        </div>
                        <div :class="appNameClass(app.name)">{{ app.name }}</div>
                    </div>

                    <p
                        v-if="settingsApps.length === 0"
                        class="empty-list-message"
                    >
                        没有检测到其他运行中的应用
                    </p>
                </div>
            </div>

            <div class="section">
                <div class="section-label">🚀 开机启动</div>
                <label class="autostart-row">
                    <USwitch
                        v-model="autostartEnabled"
                        :disabled="autostartLoading"
                    />
                    <span>开机自动启动 Focus Must</span>
                </label>
            </div>

            <div class="settings-actions">
                <button
                    class="btn btn-ghost flex-1"
                    @click="currentView = 'planning'"
                >
                    返回
                </button>
                <button class="btn btn-success flex-1" @click="saveSettings">
                    保存设置
                </button>
            </div>
        </div>

        <div
            v-else-if="!isFocusing && currentView === 'analytics'"
            class="overlay-card analytics-card"
        >
            <div class="lock-icon">📊</div>
            <h1 class="overlay-title">统计分析</h1>
            <p class="overlay-subtitle">专注与休息数据总览</p>

            <div v-if="analyticsLoading" class="analytics-loading">正在计算统计数据...</div>

            <div v-else-if="analyticsData" class="analytics-content">
                <div class="analytics-summary-grid">
                    <div class="analytics-summary-card">
                        <div class="analytics-summary-label">总专注时长</div>
                        <div class="analytics-summary-value">
                            {{ formatDurationLabel(analyticsData.summary.total_focus_secs) }}
                        </div>
                    </div>
                    <div class="analytics-summary-card">
                        <div class="analytics-summary-label">总休息时长</div>
                        <div class="analytics-summary-value">
                            {{ formatDurationLabel(analyticsData.summary.total_break_secs) }}
                        </div>
                    </div>
                    <div class="analytics-summary-card">
                        <div class="analytics-summary-label">总会话数</div>
                        <div class="analytics-summary-value">
                            {{ analyticsData.summary.total_sessions }}
                        </div>
                    </div>
                </div>

                <div class="analytics-section">
                    <div class="section-label">📈 日趋势（近 30 天）</div>
                    <div class="trend-bars">
                        <div
                            v-for="point in analyticsData.daily_trend"
                            :key="point.day"
                            class="trend-bar-item"
                        >
                            <div class="trend-bar-track">
                                <div
                                    class="trend-bar-fill"
                                    :style="{
                                        height: `${Math.max(4, (point.focus_secs / maxDailyFocusSecs) * 100)}%`,
                                    }"
                                ></div>
                            </div>
                            <div class="trend-bar-label">{{ point.day.slice(5) }}</div>
                        </div>
                    </div>
                </div>

                <div class="analytics-section">
                    <div class="section-label">🕒 专注时段分布</div>
                    <div class="hour-distribution-list">
                        <div
                            v-for="bucket in analyticsData.focus_hour_distribution"
                            :key="bucket.hour"
                            class="hour-distribution-item"
                        >
                            <div class="hour-label">{{ formatHourLabel(bucket.hour) }}</div>
                            <div class="hour-track">
                                <div
                                    class="hour-fill"
                                    :style="{
                                        width: `${(bucket.focus_secs / maxHourFocusSecs) * 100}%`,
                                    }"
                                ></div>
                            </div>
                            <div class="hour-value">{{ formatDurationCompact(bucket.focus_secs) }}</div>
                        </div>
                    </div>
                </div>
            </div>

            <div class="settings-actions">
                <button class="btn btn-ghost flex-1" @click="currentView = 'planning'">
                    返回
                </button>
            </div>
        </div>

        <!-- FOCUS MODE (shown when blocking window reappears) -->
        <div v-else-if="isFocusing" class="overlay-card focus-card">
            <!-- Blocked App Alert -->
            <div v-if="blockedAppState" class="blocked-alert">
                <div class="lock-icon blocked-icon-lg">🚫</div>
                <h2 class="overlay-title text-accent">检测到分心</h2>
                <p class="overlay-subtitle mb-lg">
                    你打开了 <strong>{{ blockedAppState.name }}</strong>
                </p>

                <!-- Has return target: show countdown -->
                <template v-if="blockedAppState.return_to_bundle_id">
                    <div class="countdown-circle">
                        <div class="countdown-number">
                            {{ returnCountdown }}
                        </div>
                        <div class="countdown-label">秒后返回</div>
                    </div>
                    <p class="focus-hint mt-lg">
                        正在带你回到
                        <strong>{{ blockedAppState.return_to_name }}</strong>
                        ...
                    </p>
                </template>

                <!-- No return target: prompt manual switch -->
                <template v-else>
                    <p class="focus-hint mt-lg">
                        ⚠️ 请使用 <strong>⌘+Tab</strong> 手动切换回工作 App
                    </p>
                </template>
            </div>

            <!-- Normal Focus State (Timer) -->
            <div v-else>
                <div class="focus-icon">🧘</div>
                <h1 class="overlay-title">保持专注</h1>

                <div class="timer-display">{{ formattedTime }}</div>

                <div class="focus-task">
                    {{ taskDescription || "正在完成一项重要的任务..." }}
                </div>

                <div class="allowed-apps" v-if="allowedAppNames.length > 0">
                    <span
                        v-for="app in allowedAppNames"
                        :key="app.bundle_id"
                        class="allowed-app-item"
                    >
                        {{ app.name }}
                    </span>
                </div>

                <div class="focus-hint">
                    试图打开其他应用时，窗口会再次出现提醒你。
                </div>

                <div class="btn-group">
                    <button class="btn btn-ghost" @click="requestEndFocus">
                        结束专注
                    </button>
                </div>
            </div>

            <!-- End Focus Confirmation -->
            <div v-if="showEndConfirm" class="confirm-overlay">
                <p class="confirm-text">确定要结束本次专注吗？</p>
                <div class="confirm-actions">
                    <button class="btn btn-ghost" @click="cancelEndFocus">
                        取消
                    </button>
                    <button class="btn btn-primary" @click="confirmEndFocus">
                        确定结束
                    </button>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.blocked-alert {
    animation: fadeIn 0.3s ease;
}
.countdown-circle {
    width: 80px;
    height: 80px;
    border-radius: 50%;
    border: 4px solid var(--accent);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    margin: 0 auto;
    box-shadow: 0 0 20px rgba(233, 69, 96, 0.4);
}
.countdown-number {
    font-size: 32px;
    font-weight: 700;
    color: var(--accent);
    line-height: 1;
}
.countdown-label {
    font-size: 10px;
    color: var(--text-secondary);
    text-transform: uppercase;
    margin-top: 2px;
}
@keyframes fadeIn {
    from {
        opacity: 0;
        transform: translateY(10px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
}

/* Snow */
.snow-canvas {
    position: fixed;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 100000;
}
.settings-toggle-group {
    position: absolute;
    top: 16px;
    right: 16px;
    z-index: 10;
    display: flex;
    gap: 8px;
}
.snow-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 20px;
    cursor: pointer;
    transition: all 0.2s ease;
    user-select: none;
}
.snow-toggle:hover {
    background: rgba(255, 255, 255, 0.15);
}
.snow-toggle-icon {
    font-size: 14px;
}
.snow-toggle-label {
    font-size: 12px;
    color: var(--text-secondary);
    font-weight: 500;
}

.autostart-row {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-primary);
    font-size: 14px;
    cursor: pointer;
}

/* Confirm overlay */
.confirm-overlay {
    position: absolute;
    inset: 0;
    backdrop-filter: blur(10px);
    display: flex;
    flex-direction: column;
    justify-content: center;
    z-index: 10;
    background: rgba(0, 0, 0, 0.5);
    border-radius: 24px;
}
</style>
