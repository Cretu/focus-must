<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import AppGrid from "./AppGrid.vue";
import type { PreferredLocale } from "../i18n";
import type { AppInfo } from "../types/contracts";

const props = defineProps<{
    settingsApps: AppInfo[];
    settingsWhitelist: Set<string>;
    autostartEnabled: boolean;
    autostartLoading: boolean;
    settingsLocale: PreferredLocale;
    localeOptionsWithText: Array<{ label: string; value: PreferredLocale }>;
}>();

const emit = defineEmits<{
    (event: "refresh"): void;
    (event: "toggle-settings-app", bundleId: string): void;
    (event: "back"): void;
    (event: "save"): void;
    (event: "update:autostartEnabled", value: boolean): void;
    (event: "update:settingsLocale", value: PreferredLocale): void;
}>();

const { t } = useI18n();

const autostartEnabledModel = computed({
    get: () => props.autostartEnabled,
    set: (value: boolean) => emit("update:autostartEnabled", value),
});

const settingsLocaleModel = computed({
    get: () => props.settingsLocale,
    set: (value: PreferredLocale) => emit("update:settingsLocale", value),
});
</script>

<template>
    <UCard class="w-[min(980px,92vw)] max-h-[88vh] overflow-hidden">
        <template #header>
            <div class="space-y-1">
                <div class="flex min-w-0 items-center gap-2">
                    <UIcon name="i-lucide-settings" class="text-3xl text-primary" />
                    <h1 class="text-xl font-semibold leading-tight">{{ t("app.settingsTitle") }}</h1>
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
                        <UButton color="neutral" variant="outline" size="xs" @click="emit('refresh')">
                            {{ t("app.refresh") }}
                        </UButton>
                    </div>
                </template>

                <AppGrid
                    :apps="settingsApps"
                    :selected-apps="settingsWhitelist"
                    @toggle-app="(id: string) => emit('toggle-settings-app', id)"
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
                    <USwitch v-model="autostartEnabledModel" :disabled="autostartLoading" />
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
                            v-model="settingsLocaleModel"
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
        </div>

        <template #footer>
            <div class="flex gap-2">
                <UButton
                    color="neutral"
                    variant="outline"
                    class="flex-1 justify-center text-center"
                    @click="emit('back')"
                >
                    <UIcon name="i-lucide-arrow-left" class="text-base" />
                    {{ t("app.back") }}
                </UButton>
                <UButton
                    color="success"
                    variant="solid"
                    class="flex-1 justify-center text-center"
                    @click="emit('save')"
                >
                    <UIcon name="i-lucide-save" class="text-base" />
                    {{ t("app.saveSettings") }}
                </UButton>
            </div>
        </template>
    </UCard>
</template>
