import { defineStore } from "pinia";
import { ref, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useI18n } from "vue-i18n";
import {
    isEnabled as isAutostartEnabled,
    enable as enableAutostart,
    disable as disableAutostart,
} from "@tauri-apps/plugin-autostart";
import {
    isSupportedLocale,
    type LocaleCode,
    type PreferredLocale,
} from "../i18n";
import { useHistory } from "../composables/useHistory";
import type {
    AppInfo,
    AppState,
    BlockedAppEvent,
} from "../types/contracts";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useAppStore = defineStore("app", () => {
    const { locale } = useI18n();

    // --- Locale ---
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

    // --- Core State ---
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

    // --- Focus Session ---
    const elapsedSeconds = ref(0);
    const showEndConfirm = ref(false);
    const allowedAppNames = ref<AppInfo[]>([]);

    // --- Blocked App ---
    const blockedAppState = ref<BlockedAppEvent | null>(null);
    const returnCountdown = ref(3);

    // --- Settings ---
    const settingsApps = ref<AppInfo[]>([]);
    const settingsWhitelist = ref<Set<string>>(new Set());
    const settingsLocale = ref<PreferredLocale>("system");
    const autostartEnabled = ref(false);
    const autostartLoading = ref(false);

    // --- History ---
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

    // --- Break Timer ---
    const showFreeActivityOptions = ref(false);
    const customMinutes = ref("");
    const breakRemaining = ref("");
    let breakTimer: ReturnType<typeof setInterval> | null = null;

    // --- Event listeners ---
    let unlistenState: UnlistenFn | null = null;
    let unlistenBlocked: UnlistenFn | null = null;
    let unlistenBlockedCleared: UnlistenFn | null = null;
    let unlistenShowView: UnlistenFn | null = null;
    let timerInterval: ReturnType<typeof setInterval> | null = null;
    let countdownInterval: ReturnType<typeof setInterval> | null = null;

    // ---------------------------------------------------------------------------
    // Derived State
    // ---------------------------------------------------------------------------

    const isFocusing = computed(() => appState.value.focus_started_at !== null);

    const isOnBreak = computed(
        () =>
            appState.value.free_activity_end_at !== null &&
            appState.value.free_activity_end_at !== undefined,
    );

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

    // ---------------------------------------------------------------------------
    // Actions
    // ---------------------------------------------------------------------------

    function clearBlockedState() {
        if (countdownInterval) {
            clearInterval(countdownInterval);
            countdownInterval = null;
        }
        blockedAppState.value = null;
        returnCountdown.value = 3;
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

        await Promise.all(
            list.map(async (app) => {
                if (app.icon_data_url) return;
                const icon = await fetchAppIcon(app.bundle_id);
                if (!icon) return;

                if (target === "running") {
                    const matched = runningApps.value.find(
                        (item) => item.bundle_id === app.bundle_id,
                    );
                    if (matched) matched.icon_data_url = icon;
                } else {
                    const matched = settingsApps.value.find(
                        (item) => item.bundle_id === app.bundle_id,
                    );
                    if (matched) matched.icon_data_url = icon;
                }
            })
        );
    }

    function initSelectedFromWhitelist() {
        const wl = appState.value.default_whitelist;
        if (wl.length > 0) {
            selectedApps.value = new Set(wl);
        }
    }

    async function loadState() {
        try {
            appState.value = await invoke<AppState>("get_state");
            if (isSupportedLocale(appState.value.locale)) {
                selectedLocale.value = appState.value.locale;
            }
        } catch (error) {
            console.error("Failed to load app state:", error);
        }
    }

    async function loadApps(includeIcons = false) {
        try {
            runningApps.value = await invoke<AppInfo[]>("get_running_apps", {
                includeIcons,
            });
            if (!includeIcons) {
                void hydrateIcons("running");
            }
        } catch (error) {
            console.error("Failed to load running apps:", error);
            runningApps.value = [];
        }
    }

    function toggleApp(bundleId: string) {
        selectedApps.value = toggleSetItem(selectedApps.value, bundleId);
    }

    function applyRecentTask(task: string) {
        taskDescription.value = task;
        isTaskInputInvalid.value = false;
    }

    // --- Settings ---

    async function openSettings() {
        currentView.value = "settings";
        autostartLoading.value = true;
        try {
            settingsApps.value = await invoke<AppInfo[]>("get_running_apps", {
                includeIcons: false,
            });
            void hydrateIcons("settings");
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
                                    { bundleId, includeIcon: true },
                                );
                                if (info) return info;
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

    function toggleSettingsApp(bundleId: string) {
        settingsWhitelist.value = toggleSetItem(settingsWhitelist.value, bundleId);
    }

    async function saveSettings() {
        const whitelist = Array.from(settingsWhitelist.value);
        try {
            await invoke("update_settings", { defaultWhitelist: whitelist });
        } catch (error) {
            console.error("Failed to update settings:", error);
            return;
        }
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

    async function openAnalytics() {
        currentView.value = "analytics";
        await loadAnalytics();
    }

    // --- Focus Session ---

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
        try {
            await invoke("unlock_session", { whitelist, task });
        } catch (error) {
            console.error("Failed to start focus session:", error);
        }
    }

    async function confirmEndFocus() {
        showEndConfirm.value = false;
        try {
            await invoke("lock_session");
        } catch (error) {
            console.error("Failed to end focus session:", error);
            return;
        }

        selectedApps.value = new Set();
        taskDescription.value = "";
        allowedAppNames.value = [];
        await loadApps();
        initSelectedFromWhitelist();
        await loadHistory(true);
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
                invoke<void>("switch_to_app", { bundleId: returnBundleId }).catch((error) => {
                    console.error("Failed to switch back to allowed app:", error);
                });
            }
        }, 1000);
    }

    // --- Break Timer ---

    function stopBreakTimer() {
        if (breakTimer) {
            clearInterval(breakTimer);
            breakTimer = null;
        }
    }

    function updateBreakCountdown() {
        const endAt = appState.value.free_activity_end_at;
        if (!endAt) {
            breakRemaining.value = "";
            return;
        }
        const now = Math.floor(Date.now() / 1000);
        const remaining = endAt - now;
        if (remaining <= 0) {
            breakRemaining.value = "";
        } else {
            const mins = Math.floor(remaining / 60);
            const secs = remaining % 60;
            breakRemaining.value = `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
        }
    }

    async function startFreeActivity(minutes: number) {
        try {
            await invoke("start_free_activity", { durationMinutes: minutes });
            showFreeActivityOptions.value = false;
        } catch (e) {
            console.error("Failed to start free activity:", e);
        }
    }

    // ---------------------------------------------------------------------------
    // Watchers
    // ---------------------------------------------------------------------------

    // Focus timer
    watch(
        () => appState.value.focus_started_at,
        (newVal) => {
            if (newVal) {
                if (timerInterval) clearInterval(timerInterval);
                updateTimer();
                timerInterval = setInterval(updateTimer, 1000);

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

    // Task input validation
    watch(taskDescription, (value) => {
        if (value.trim()) {
            isTaskInputInvalid.value = false;
        }
    });

    // Locale sync
    watch(
        () => appState.value.locale,
        (nextLocale) => {
            if (isSupportedLocale(nextLocale)) {
                selectedLocale.value = nextLocale;
            }
        },
    );

    // Break timer
    watch(
        isOnBreak,
        (v) => {
            if (v) {
                stopBreakTimer();
                updateBreakCountdown();
                breakTimer = setInterval(updateBreakCountdown, 1000);
            } else {
                stopBreakTimer();
                breakRemaining.value = "";
                showFreeActivityOptions.value = false;
            }
        },
        { immediate: true },
    );

    // ---------------------------------------------------------------------------
    // Lifecycle
    // ---------------------------------------------------------------------------

    async function initialize() {
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
    }

    function cleanup() {
        if (unlistenState) unlistenState();
        if (unlistenBlocked) unlistenBlocked();
        if (unlistenBlockedCleared) unlistenBlockedCleared();
        if (unlistenShowView) unlistenShowView();
        if (timerInterval) clearInterval(timerInterval);
        if (countdownInterval) clearInterval(countdownInterval);
        stopBreakTimer();
    }

    return {
        // State
        currentView,
        taskDescription,
        runningApps,
        selectedApps,
        appState,
        isTaskInputShaking,
        isTaskInputInvalid,
        isBooting,
        elapsedSeconds,
        showEndConfirm,
        allowedAppNames,
        blockedAppState,
        returnCountdown,
        settingsApps,
        settingsWhitelist,
        settingsLocale,
        autostartEnabled,
        autostartLoading,
        sessionHistory,
        historyHasMore,
        historyLoading,
        analyticsData,
        analyticsLoading,
        showFreeActivityOptions,
        customMinutes,
        breakRemaining,

        // Computed
        selectedLocale,
        effectiveLocale,
        isFocusing,
        isOnBreak,
        recentTaskSuggestions,
        formattedTime,

        // Actions
        applyRecentTask,
        loadApps,
        toggleApp,
        openSettings,
        toggleSettingsApp,
        saveSettings,
        openAnalytics,
        startFocus,
        confirmEndFocus,
        startFreeActivity,
        loadMoreHistory,

        // Lifecycle
        initialize,
        cleanup,
    };
});
