<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { AppInfo } from "../types/contracts";

defineProps<{
    apps: AppInfo[];
    selectedApps: Set<string>;
    emptyMessage?: string;
}>();

const emit = defineEmits<{
    (event: "toggle-app", bundleId: string): void;
}>();

const { t } = useI18n();

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
    <div class="app-grid">
        <UCard
            v-for="app in apps"
            :key="app.bundle_id"
            variant="outline"
            :class="['app-item', { selected: selectedApps.has(app.bundle_id) }]"
            @click="emit('toggle-app', app.bundle_id)"
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
            v-if="apps.length === 0"
            color="neutral"
            variant="soft"
            :title="emptyMessage || t('app.noRunningApps')"
            class="col-span-full"
        />
    </div>
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
