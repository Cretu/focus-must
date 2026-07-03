<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { useAppStore } from "../stores/appStore";
import AppGrid from "./AppGrid.vue";
import type { PreferredLocale } from "../i18n";

defineProps<{
    localeOptionsWithText: Array<{ label: string; value: PreferredLocale }>;
}>();

const store = useAppStore();
const { t } = useI18n();

const breakReminderOptions = computed(() =>
    [25, 45, 60, 90].map((m) => ({
        label: t("app.minutesShort", { minutes: m }),
        value: m,
    })),
);

onMounted(() => {
    store.runSelfCheck();
});
</script>

<template>
    <UCard class="w-[min(980px,92vw)] max-h-[88vh] overflow-hidden">
        <template #header>
            <div class="space-y-1">
                <div class="flex min-w-0 items-center gap-2">
                    <UIcon name="i-lucide-settings" class="text-3xl text-primary" />
                    <h1 class="font-serif text-xl font-semibold leading-tight tracking-tight">{{ t("app.settingsTitle") }}</h1>
                </div>
            </div>
        </template>

        <div class="space-y-4 overflow-y-auto max-h-[62vh]">
            <UCard variant="soft">
                <template #header>
                    <div class="flex items-center justify-between gap-2">
                        <div>
                            <div class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                                <UIcon name="i-lucide-layout-grid" class="text-base" />
                                <span>{{ t("app.defaultAllowedApps") }}</span>
                                <span class="text-xs font-normal">({{ t("app.defaultAllowedAppsSubtitle") }})</span>
                            </div>
                        </div>
                        <UButton
                            color="neutral"
                            variant="outline"
                            size="xs"
                            leading-icon="i-lucide-rotate-ccw"
                            :loading="store.settingsAppsLoading"
                            @click="store.refreshSettingsApps()"
                        >
                            {{ t("app.refresh") }}
                        </UButton>
                    </div>
                </template>

                <AppGrid
                    :apps="store.settingsApps"
                    :selected-apps="store.settingsWhitelist"
                    :loading="store.settingsAppsLoading"
                    @toggle-app="(id: string) => store.toggleSettingsApp(id)"
                />
            </UCard>

            <UCard variant="soft">
                <div class="flex items-center justify-between gap-3">
                    <div>
                        <p class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                            <UIcon name="i-lucide-power" class="text-base" />
                            <span>{{ t("app.autostart") }}</span>
                        </p>
                        <p class="text-xs text-muted">{{ t("app.autostartSubtitle") }}</p>
                    </div>
                    <USwitch v-model="store.autostartEnabled" :disabled="store.autostartLoading" />
                </div>
            </UCard>

            <UCard variant="soft">
                <div class="space-y-3">
                    <div class="flex items-center justify-between gap-3">
                        <div>
                            <p class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                                <UIcon name="i-lucide-bell" class="text-base" />
                                <span>{{ t("app.breakReminder") }}</span>
                            </p>
                            <p class="text-xs text-muted">{{ t("app.breakReminderSubtitle") }}</p>
                        </div>
                        <USwitch
                            :model-value="store.appState.focus_goal_minutes > 0"
                            @update:model-value="(v: boolean) => store.setBreakReminder(v ? 45 : 0)"
                        />
                    </div>
                    <div
                        v-if="store.appState.focus_goal_minutes > 0"
                        class="flex items-center justify-between gap-3"
                    >
                        <p class="text-xs text-muted">{{ t("app.breakReminderEvery") }}</p>
                        <USelect
                            :model-value="store.appState.focus_goal_minutes"
                            :items="breakReminderOptions"
                            value-key="value"
                            label-key="label"
                            size="sm"
                            class="w-32"
                            @update:model-value="(v: number) => store.setBreakReminder(Number(v))"
                        />
                    </div>
                </div>
            </UCard>

            <UCard variant="soft">
                <div class="space-y-3">
                    <div class="flex items-center justify-between gap-3">
                        <p class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                            <UIcon name="i-lucide-languages" class="text-base" />
                            <span>{{ t("app.defaultLanguage") }}</span>
                        </p>
                        <USelect
                            v-model="store.settingsLocale"
                            :items="localeOptionsWithText"
                            value-key="value"
                            label-key="label"
                            size="sm"
                            class="w-32"
                        />
                    </div>

                    <div class="flex items-center justify-between gap-3">
                        <p class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                            <UIcon name="i-lucide-sun-moon" class="text-base" />
                            <span>{{ t("app.defaultAppearance") }}</span>
                        </p>
                        <UColorModeSelect size="sm" class="w-28" />
                    </div>
                </div>
            </UCard>

            <UCard variant="soft">
                <template #header>
                    <div class="flex items-center justify-between gap-2">
                        <div class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                            <UIcon name="i-lucide-stethoscope" class="text-base" />
                            <span>{{ t("app.selfCheck") }}</span>
                            <span class="text-xs font-normal">({{ t("app.selfCheckSubtitle") }})</span>
                        </div>
                        <UButton
                            color="neutral"
                            variant="outline"
                            size="xs"
                            :loading="store.selfCheckLoading"
                            leading-icon="i-lucide-refresh-cw"
                            @click="store.runSelfCheck()"
                        >
                            {{ t("app.recheck") }}
                        </UButton>
                    </div>
                </template>

                <div class="space-y-3 text-sm">
                    <!-- Foreground monitoring -->
                    <div class="flex items-center justify-between gap-3">
                        <span class="flex items-center gap-1.5 text-muted">
                            <UIcon name="i-lucide-eye" class="text-base" />
                            {{ t("app.checkMonitoring") }}
                        </span>
                        <UBadge
                            v-if="store.selfCheck?.last_frontmost"
                            color="success"
                            variant="soft"
                        >
                            {{ t("app.checkOk") }} · {{ store.selfCheck.last_frontmost }}
                        </UBadge>
                        <UBadge v-else color="warning" variant="soft">
                            {{ t("app.checkMonitoringWait") }}
                        </UBadge>
                    </div>

                    <!-- Displays -->
                    <div class="space-y-1.5">
                        <div class="flex items-center justify-between gap-3">
                            <span class="flex items-center gap-1.5 text-muted">
                                <UIcon name="i-lucide-monitor" class="text-base" />
                                {{ t("app.checkDisplays") }}
                            </span>
                            <UBadge color="neutral" variant="soft">
                                {{ t("app.displayCount", { count: store.selfCheck?.monitors.length ?? 0 }) }}
                            </UBadge>
                        </div>
                        <div
                            v-for="(mon, i) in store.selfCheck?.monitors ?? []"
                            :key="i"
                            class="flex items-center justify-between gap-2 rounded-md bg-elevated/50 px-2 py-1 text-xs text-muted"
                        >
                            <span class="truncate">{{ mon.name || `#${i + 1}` }}</span>
                            <span class="flex items-center gap-1.5">
                                {{ mon.width }}×{{ mon.height }}
                                <UBadge v-if="mon.is_primary" color="primary" variant="soft" size="sm">
                                    {{ t("app.primaryDisplay") }}
                                </UBadge>
                            </span>
                        </div>
                    </div>

                    <!-- Version -->
                    <div class="flex items-center justify-between gap-3">
                        <span class="flex items-center gap-1.5 text-muted">
                            <UIcon name="i-lucide-tag" class="text-base" />
                            {{ t("app.checkVersion") }}
                        </span>
                        <UBadge color="neutral" variant="soft">v{{ store.selfCheck?.version ?? "—" }}</UBadge>
                    </div>

                    <!-- Interactive tests -->
                    <div class="flex flex-wrap gap-2 pt-1">
                        <UButton
                            color="neutral"
                            variant="outline"
                            size="sm"
                            leading-icon="i-lucide-volume-2"
                            @click="store.testSound()"
                        >
                            {{ t("app.testSound") }}
                        </UButton>
                        <UButton
                            color="neutral"
                            variant="outline"
                            size="sm"
                            leading-icon="i-lucide-bell"
                            @click="store.testNotification()"
                        >
                            {{ t("app.testNotification") }}
                        </UButton>
                    </div>
                </div>
            </UCard>
        </div>

        <template #footer>
            <div class="flex gap-2">
                <UButton
                    color="neutral"
                    variant="outline"
                    class="flex-1 justify-center text-center"
                    @click="store.currentView = 'planning'"
                >
                    <UIcon name="i-lucide-arrow-left" class="text-base" />
                    {{ t("app.back") }}
                </UButton>
                <UButton
                    color="primary"
                    variant="solid"
                    class="flex-1 justify-center text-center"
                    @click="store.saveSettings()"
                >
                    <UIcon name="i-lucide-save" class="text-base" />
                    {{ t("app.saveSettings") }}
                </UButton>
            </div>
        </template>
    </UCard>
</template>
