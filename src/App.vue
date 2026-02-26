<script setup lang="ts">
import {
    ref,
    onMounted,
    onUnmounted,
    computed,
    watch,
    defineAsyncComponent,
} from "vue";
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
import { useHistory } from "./composables/useHistory";
import { useSettings } from "./composables/useSettings";
import PlanningView from "./components/PlanningView.vue";
import {
    isSupportedLocale,
    localeOptionsWithText,
    type LocaleCode,
    type PreferredLocale,
} from "./i18n";
import type { AppInfo, AppState, BlockedAppEvent } from "./types/contracts";

const FocusSessionCard = defineAsyncComponent(
    () => import("./components/FocusSessionCard.vue"),
);
const SettingsView = defineAsyncComponent(
    () => import("./components/SettingsView.vue"),
);
const AnalyticsView = defineAsyncComponent(
    () => import("./components/AnalyticsView.vue"),
);

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
    task_description: null,
    focus_started_at: null,
    free_activity_started_at: null,
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

// Settings state (composable)
const {
    settingsApps,
    settingsWhitelist,
    settingsLocale,
    autostartEnabled,
    autostartLoading,
    loadSettingsApps,
    toggleSettingsApp,
} = useSettings();

// History state (composable)
const {
    sessionHistory,
    historyHasMore,
    historyLoading,
    analyticsData,
    analyticsLoading,
    loadHistory,
    loadMoreHistory,
    loadAnalytics,
} = useHistory();

// Composables
const snowEffect = useSnowEffect();
const { snowEnabled } = snowEffect;
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
        await loadSettingsApps((target) => hydrateIcons(target));
        settingsWhitelist.value = new Set(appState.value.default_whitelist);

        if (settingsWhitelist.value.size > 0) {
            const existing = new Set(
                settingsApps.value.map((app) => app.bundle_id),
            );
            const missingIds = Array.from(settingsWhitelist.value).filter(
                (bundleId) => !existing.has(bundleId),
            );

            if (missingIds.length > 0) {
                const recovered = await Promise.all(
                    missingIds.map(async (bundleId) => {
                        try {
                            const info = await invoke<AppInfo | null>(
                                "get_app_info",
                                {
                                    bundleId,
                                    includeIcon: true,
                                },
                            );

                            if (info) {
                                return info;
                            }
                        } catch (error) {
                            console.error(
                                "Failed to recover default app info:",
                                bundleId,
                                error,
                            );
                        }

                        return {
                            bundle_id: bundleId,
                            name: bundleId,
                            icon_data_url: null,
                        };
                    }),
                );

                settingsApps.value = [...recovered, ...settingsApps.value];
                void hydrateIcons("settings");
            }
        }

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
    await loadAnalytics();
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
</script>

<template>
    <UApp :locale="nuxtUiLocale">
        <div class="overlay-container">
            <canvas
                :ref="snowEffect.setSnowCanvas"
                class="snow-canvas"
                v-show="snowEnabled"
            ></canvas>

            <UCard v-if="isBooting" class="w-[min(420px,86vw)]">
                <div class="space-y-3 text-center">
                    <div class="startup-spinner" aria-hidden="true"></div>
                    <h1 class="text-xl font-semibold">Focus Must</h1>
                    <UProgress :model-value="null" size="sm" />
                    <p class="text-sm text-muted">
                        {{ t("app.startupLoadingApps") }}
                    </p>
                </div>
            </UCard>

            <PlanningView
                v-else-if="!isFocusing && currentView === 'planning'"
                v-model:snow-enabled="snowEnabled"
                v-model:task-description="taskDescription"
                v-model:show-free-activity-options="showFreeActivityOptions"
                v-model:custom-minutes="customMinutes"
                :is-task-input-invalid="isTaskInputInvalid"
                :is-task-input-shaking="isTaskInputShaking"
                :recent-task-suggestions="recentTaskSuggestions"
                :running-apps="runningApps"
                :selected-apps="selectedApps"
                :is-on-break="isOnBreak"
                :break-remaining="breakRemaining"
                :session-history="sessionHistory"
                :history-has-more="historyHasMore"
                :history-loading="historyLoading"
                @open-settings="openSettings"
                @open-analytics="openAnalytics"
                @clear-task-invalid="isTaskInputInvalid = false"
                @apply-recent-task="applyRecentTask"
                @refresh-apps="loadApps()"
                @toggle-app="toggleApp"
                @start-free-activity="startFreeActivity"
                @start-focus="startFocus"
                @load-more-history="loadMoreHistory"
            />

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
        radial-gradient(
            circle at 18% 14%,
            rgba(59, 130, 246, 0.08),
            transparent 42%
        ),
        radial-gradient(
            circle at 82% 86%,
            rgba(16, 185, 129, 0.06),
            transparent 44%
        ),
        linear-gradient(
            135deg,
            rgba(255, 255, 255, 0.05),
            rgba(255, 255, 255, 0.01)
        ),
        rgba(20, 28, 44, 0.12);
    backdrop-filter: blur(22px) saturate(130%);
    -webkit-backdrop-filter: blur(22px) saturate(130%);
}

:global(html.light) .overlay-container {
    background:
        radial-gradient(
            circle at 18% 14%,
            rgba(59, 130, 246, 0.035),
            transparent 42%
        ),
        radial-gradient(
            circle at 82% 86%,
            rgba(16, 185, 129, 0.03),
            transparent 44%
        ),
        linear-gradient(
            135deg,
            rgba(255, 255, 255, 0.08),
            rgba(255, 255, 255, 0.015)
        ),
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

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}
</style>
