<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
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
                    <div class="flex items-start justify-between gap-2">
                        <div>
                            <div class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                                <UIcon name="i-lucide-layout-grid" class="text-base" />
                                <span>{{ t("app.defaultAllowedApps") }}</span>
                            </div>
                            <p class="text-xs text-muted">{{ t("app.defaultAllowedAppsSubtitle") }}</p>
                        </div>
                        <UButton color="neutral" variant="outline" size="xs" @click="emit('refresh')">
                            {{ t("app.refresh") }}
                        </UButton>
                    </div>
                </template>

                <div class="app-grid">
                    <UCard
                        v-for="app in settingsApps"
                        :key="app.bundle_id"
                        variant="outline"
                        :class="['app-item', { selected: settingsWhitelist.has(app.bundle_id) }]"
                        @click="emit('toggle-settings-app', app.bundle_id)"
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
                        :title="t('app.noRunningApps')"
                        class="col-span-full"
                    />
                </div>
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

<style scoped>
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
</style>
