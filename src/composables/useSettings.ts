import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppInfo } from "../types/contracts";
import type { PreferredLocale } from "../i18n";

export function useSettings() {
    const settingsApps = ref<AppInfo[]>([]);
    const settingsWhitelist = ref<Set<string>>(new Set());
    const settingsLocale = ref<PreferredLocale>("system");
    const autostartEnabled = ref(false);
    const autostartLoading = ref(false);

    async function loadSettingsApps(hydrateIconsFn: (target: "settings") => void) {
        settingsApps.value = await invoke<AppInfo[]>("get_running_apps", {
            includeIcons: false,
        });
        hydrateIconsFn("settings");
    }

    function toggleSettingsApp(bundleId: string) {
        const next = new Set(settingsWhitelist.value);
        if (next.has(bundleId)) {
            next.delete(bundleId);
        } else {
            next.add(bundleId);
        }
        settingsWhitelist.value = next;
    }

    return {
        settingsApps,
        settingsWhitelist,
        settingsLocale,
        autostartEnabled,
        autostartLoading,
        loadSettingsApps,
        toggleSettingsApp,
    };
}
