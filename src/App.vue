<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useI18n } from "vue-i18n";
import { en, zh_cn } from "@nuxt/ui/locale";
import {
    isEnabled as isAutostartEnabled,
    enable as enableAutostart,
    disable as disableAutostart,
} from "@tauri-apps/plugin-autostart";
import { useSnowEffect } from "./composables/useSnowEffect";
import { useBreakTimer } from "./composables/useBreakTimer";
import HistoryList from "./components/HistoryList.vue";
import FocusSessionCard from "./components/FocusSessionCard.vue";
import SettingsView from "./components/SettingsView.vue";
import AnalyticsView from "./components/AnalyticsView.vue";
import {
    isSupportedLocale,
    localeOptionsWithText,
    type LocaleCode,
    type PreferredLocale,
} from "./i18n";
import type {
    AnalyticsData,
    AppInfo,
    AppState,
    BlockedAppEvent,
    HistoryPage,
    SessionRecord,
} from "./types/contracts";

const HISTORY_PAGE_SIZE = 100;

const { t, locale } = useI18n();

const selectedLocale = ref<PreferredLocale>("system");

const effectiveLocale = computed<LocaleCode>(() => {
    if (selectedLocale.value === "system") {
        return navigator.language.toLowerCase().startsWith("en")
            ? "en-US"
            : "zh-CN";
    }

    return selectedLocale.value;
});

watch(
    effectiveLocale,
    (nextLocale) => {
        locale.value = nextLocale;
    },
    { immediate: true },
);

const nuxtUiLocale = computed(() =>
    effectiveLocale.value === "en-US" ? en : zh_cn,
);

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
    locale: "system",
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
const settingsLocale = ref<PreferredLocale>("system");
const autostartEnabled = ref(false);
const autostartLoading = ref(false);

// History state
const sessionHistory = ref<SessionRecord[]>([]);
const historyOffset = ref(0);
const historyHasMore = ref(true);
const historyLoading = ref(false);
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

watch(
    () => appState.value.locale,
    (nextLocale) => {
        if (isSupportedLocale(nextLocale)) {
            selectedLocale.value = nextLocale;
        }
    },
);

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
        settingsLocale.value = appState.value.locale;

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

    try {
        await invoke("set_locale", { locale: settingsLocale.value });
    } catch (e) {
        console.error("Failed to update locale:", e);
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
    if (isSupportedLocale(appState.value.locale)) {
        selectedLocale.value = appState.value.locale;
    }
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
    <UApp :locale="nuxtUiLocale">
    <div class="overlay-container">
        <canvas ref="snowCanvas" class="snow-canvas" v-show="snowEnabled"></canvas>

        <UCard v-if="isBooting" class="w-[min(420px,86vw)]">
            <div class="space-y-3 text-center">
                <div class="startup-spinner" aria-hidden="true"></div>
                <h1 class="text-xl font-semibold">Focus Must</h1>
                <UProgress :model-value="null" size="sm" />
                <p class="text-sm text-muted">{{ t("app.startupLoadingApps") }}</p>
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
                        <h1 class="text-xl font-semibold leading-tight">{{ t("app.planningTitle") }}</h1>
                    </div>
                    <div class="flex flex-wrap gap-2">
                        <UButton
                            color="neutral"
                            variant="outline"
                            size="sm"
                            @click="snowEnabled = !snowEnabled"
                            :leading-icon="snowEnabled ? 'i-lucide-snowflake' : 'i-lucide-moon-star'"
                        >
                            {{ snowEnabled ? t("app.snowing") : t("app.snow") }}
                        </UButton>
                        <UButton
                            color="neutral"
                            variant="outline"
                            size="sm"
                            leading-icon="i-lucide-settings"
                            @click="openSettings"
                        >
                            {{ t("app.settings") }}
                        </UButton>
                        <UButton
                            color="neutral"
                            variant="outline"
                            size="sm"
                            leading-icon="i-lucide-chart-column"
                            @click="openAnalytics"
                        >
                            {{ t("app.analytics") }}
                        </UButton>
                        <UColorModeSelect
                            size="sm"
                            class="w-28"
                        />
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
                                    <span>{{ t("app.nextTask") }}</span>
                                </div>
                            </template>
                            <div class="space-y-3">
                                <UTextarea
                                    v-model="taskDescription"
                                    :rows="3"
                                    autoresize
                                    :color="isTaskInputInvalid ? 'error' : 'success'"
                                    :highlight="isTaskInputInvalid"
                                    :placeholder="t('app.taskPlaceholder')"
                                    @focus="isTaskInputInvalid = false"
                                    :class="['w-full', isTaskInputShaking ? 'shake' : '']"
                                />

                                <div v-if="recentTaskSuggestions.length > 0" class="flex items-center gap-2">
                                    <div class="shrink-0 text-xs text-muted">{{ t("app.recentTasks") }}</div>
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
                                                {{ t("app.moreCount", { count: hiddenRecentTasks.length }) }}
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
                                        <span>{{ t("app.appsNeeded") }}</span>
                                    </div>
                                    <UButton color="neutral" variant="outline" size="xs" @click="loadApps()">
                                        {{ t("app.refresh") }}
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
                                    :title="t('app.noRunningApps')"
                                    class="col-span-full"
                                />
                            </div>
                        </UCard>
                    </div>

                    <div class="mt-3 shrink-0 space-y-2">
                        <template v-if="isOnBreak">
                            <UButton color="neutral" variant="soft" block disabled leading-icon="i-lucide-coffee">
                                {{ t("app.breakInProgress", { time: breakRemaining }) }}
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
                                {{ t("app.takeBreakFree") }}
                            </UButton>

                            <div v-else class="grid grid-cols-3 gap-2 sm:grid-cols-6">
                                <UButton color="neutral" variant="outline" size="sm" @click="startFreeActivity(5)">
                                    {{ t("app.minutesShort", { minutes: 5 }) }}
                                </UButton>
                                <UButton color="neutral" variant="outline" size="sm" @click="startFreeActivity(10)">
                                    {{ t("app.minutesShort", { minutes: 10 }) }}
                                </UButton>
                                <UButton color="neutral" variant="outline" size="sm" @click="startFreeActivity(15)">
                                    {{ t("app.minutesShort", { minutes: 15 }) }}
                                </UButton>
                                <UButton color="neutral" variant="outline" size="sm" @click="startFreeActivity(30)">
                                    {{ t("app.minutesShort", { minutes: 30 }) }}
                                </UButton>
                                <UButton color="neutral" variant="outline" size="sm" @click="startFreeActivity(45)">
                                    {{ t("app.minutesShort", { minutes: 45 }) }}
                                </UButton>
                                <UInput
                                    v-model="customMinutes"
                                    type="number"
                                    min="1"
                                    max="480"
                                    :placeholder="t('app.custom')"
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
                            {{ t("app.startFocus") }}
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

        <SettingsView
            v-else-if="!isFocusing && currentView === 'settings'"
            v-model:autostart-enabled="autostartEnabled"
            v-model:settings-locale="settingsLocale"
            :settings-apps="settingsApps"
            :settings-whitelist="settingsWhitelist"
            :autostart-loading="autostartLoading"
            :locale-options-with-text="localeOptionsWithText"
            @refresh="openSettings"
            @toggle-settings-app="toggleSettingsApp"
            @back="currentView = 'planning'"
            @save="saveSettings"
        />

        <AnalyticsView
            v-else-if="!isFocusing && currentView === 'analytics'"
            :analytics-loading="analyticsLoading"
            :analytics-data="analyticsData"
            @back="currentView = 'planning'"
        />

        <FocusSessionCard
            v-else-if="isFocusing"
            v-model:show-end-confirm="showEndConfirm"
            :blocked-app-state="blockedAppState"
            :return-countdown="returnCountdown"
            :formatted-time="formattedTime"
            :task-description="taskDescription"
            :allowed-app-names="allowedAppNames"
            @confirm-end="confirmEndFocus"
        />
    </div>
    </UApp>
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
