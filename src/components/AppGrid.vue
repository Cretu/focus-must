<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { AppInfo } from "../types/contracts";

const props = defineProps<{
    apps: AppInfo[];
    selectedApps: Set<string>;
    emptyMessage?: string;
    loading?: boolean;
}>();

const emit = defineEmits<{
    (event: "toggle-app", bundleId: string): void;
}>();

const { t } = useI18n();

// Filter box appears once the grid is big enough for scanning to hurt.
const SEARCH_THRESHOLD = 9;

const query = ref("");
const normalizedQuery = computed(() => query.value.trim().toLowerCase());

const filteredApps = computed(() => {
    if (!normalizedQuery.value) {
        return props.apps;
    }
    return props.apps.filter(
        (app) =>
            app.name.toLowerCase().includes(normalizedQuery.value) ||
            app.bundle_id.toLowerCase().includes(normalizedQuery.value),
    );
});

const showSearch = computed(
    () => props.apps.length >= SEARCH_THRESHOLD || normalizedQuery.value.length > 0,
);

const selectedCount = computed(
    () => props.apps.filter((app) => props.selectedApps.has(app.bundle_id)).length,
);

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
    <div class="app-grid-container">
        <div v-if="showSearch || selectedCount > 0" class="app-grid-toolbar">
            <UInput
                v-if="showSearch"
                v-model="query"
                icon="i-lucide-search"
                size="xs"
                :placeholder="t('app.searchApps')"
                class="min-w-0 flex-1"
            />
            <UBadge v-if="selectedCount > 0" color="primary" variant="soft" size="sm">
                {{ t("app.selectedCount", { count: selectedCount }) }}
            </UBadge>
        </div>

        <!-- Skeleton grid while the running-app list is being fetched -->
        <div v-if="loading && apps.length === 0" class="app-grid" aria-hidden="true">
            <USkeleton v-for="i in 10" :key="i" class="app-skeleton" />
        </div>

        <div v-else class="app-grid">
            <UCard
                v-for="app in filteredApps"
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
                class="app-grid-span"
            />
            <p v-else-if="filteredApps.length === 0" class="app-grid-span app-grid-no-match">
                {{ t("app.noMatchedApps") }}
            </p>
        </div>
    </div>
</template>

<style scoped>
.app-grid-container {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
}

.app-grid-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 6px;
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

.app-grid-span {
    grid-column: 1 / -1;
}

.app-grid-no-match {
    padding: 18px 0;
    text-align: center;
    font-size: 12px;
    color: color-mix(in oklab, currentColor 60%, transparent);
}

.app-skeleton {
    width: 100%;
    aspect-ratio: 1 / 1;
    border-radius: var(--ui-radius);
}

.app-item {
    width: 100%;
    min-width: 0;
    aspect-ratio: 1 / 1;
    background: rgba(255, 255, 255, 0.03);
    cursor: pointer;
    user-select: none;
    transition:
        transform 0.18s ease,
        box-shadow 0.18s ease,
        border-color 0.18s ease,
        background 0.18s ease;
}

.app-item:hover {
    transform: translateY(-2px);
    border-color: rgba(217, 119, 87, 0.55);
    box-shadow: 0 8px 18px -10px rgba(30, 29, 27, 0.45);
}

:global(html.light) .app-item {
    background: rgba(30, 29, 27, 0.025);
}

@media (prefers-reduced-motion: reduce) {
    .app-item {
        transition: none;
    }
    .app-item:hover {
        transform: none;
    }
}

.app-item.selected {
    border-color: rgba(217, 119, 87, 0.95);
    background: rgba(217, 119, 87, 0.16);
    box-shadow:
        0 0 0 1px rgba(217, 119, 87, 0.42),
        0 8px 16px rgba(217, 119, 87, 0.18);
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
    color: rgba(168, 162, 158, 1);
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

:global(html.light) .app-item.selected {
    border-color: rgba(165, 74, 50, 1);
    background: rgba(217, 119, 87, 0.2);
    box-shadow:
        0 0 0 1px rgba(165, 74, 50, 0.45),
        0 10px 18px rgba(165, 74, 50, 0.18);
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
