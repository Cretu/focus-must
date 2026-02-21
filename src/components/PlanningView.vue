<script setup lang="ts">
import { computed, defineAsyncComponent } from "vue";
import { useI18n } from "vue-i18n";
import type { AppInfo, SessionRecord } from "../types/contracts";

const HistoryList = defineAsyncComponent(() => import("./HistoryList.vue"));

const props = defineProps<{
    snowEnabled: boolean;
    taskDescription: string;
    isTaskInputInvalid: boolean;
    isTaskInputShaking: boolean;
    recentTaskSuggestions: string[];
    runningApps: AppInfo[];
    selectedApps: Set<string>;
    isOnBreak: boolean;
    breakRemaining: string;
    showFreeActivityOptions: boolean;
    customMinutes: string;
    sessionHistory: SessionRecord[];
    historyHasMore: boolean;
    historyLoading: boolean;
}>();

const emit = defineEmits<{
    (event: "update:snowEnabled", value: boolean): void;
    (event: "open-settings"): void;
    (event: "open-analytics"): void;
    (event: "update:taskDescription", value: string): void;
    (event: "clear-task-invalid"): void;
    (event: "apply-recent-task", task: string): void;
    (event: "refresh-apps"): void;
    (event: "toggle-app", bundleId: string): void;
    (event: "update:showFreeActivityOptions", value: boolean): void;
    (event: "start-free-activity", minutes: number): void;
    (event: "update:customMinutes", value: string): void;
    (event: "start-focus"): void;
    (event: "load-more-history"): void;
}>();

const { t } = useI18n();

const snowEnabledModel = computed({
    get: () => props.snowEnabled,
    set: (value: boolean) => emit("update:snowEnabled", value),
});

const taskDescriptionModel = computed({
    get: () => props.taskDescription,
    set: (value: string) => emit("update:taskDescription", value),
});

const showFreeActivityOptionsModel = computed({
    get: () => props.showFreeActivityOptions,
    set: (value: boolean) => emit("update:showFreeActivityOptions", value),
});

const customMinutesModel = computed({
    get: () => props.customMinutes,
    set: (value: string) => emit("update:customMinutes", value),
});

const visibleRecentTasks = computed(() => props.recentTaskSuggestions.slice(0, 3));
const hiddenRecentTasks = computed(() => props.recentTaskSuggestions.slice(3));

const recentTaskMenuItems = computed(() =>
    hiddenRecentTasks.value.map((task) => ({
        label: task,
        onSelect: () => emit("apply-recent-task", task),
    })),
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

function triggerCustomBreak() {
    if (!customMinutesModel.value) {
        return;
    }

    const minutes = Number(customMinutesModel.value);
    if (!Number.isFinite(minutes)) {
        return;
    }

    emit("start-free-activity", minutes);
}
</script>

<template>
    <UCard
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
                        @click="snowEnabledModel = !snowEnabledModel"
                        :leading-icon="snowEnabledModel ? 'i-lucide-snowflake' : 'i-lucide-moon-star'"
                    >
                        {{ snowEnabledModel ? t("app.snowing") : t("app.snow") }}
                    </UButton>
                    <UButton
                        color="neutral"
                        variant="outline"
                        size="sm"
                        leading-icon="i-lucide-settings"
                        @click="emit('open-settings')"
                    >
                        {{ t("app.settings") }}
                    </UButton>
                    <UButton
                        color="neutral"
                        variant="outline"
                        size="sm"
                        leading-icon="i-lucide-chart-column"
                        @click="emit('open-analytics')"
                    >
                        {{ t("app.analytics") }}
                    </UButton>
                    <UColorModeSelect size="sm" class="w-28" />
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
                                v-model="taskDescriptionModel"
                                :rows="3"
                                autoresize
                                :color="isTaskInputInvalid ? 'error' : 'success'"
                                :highlight="isTaskInputInvalid"
                                :placeholder="t('app.taskPlaceholder')"
                                @focus="emit('clear-task-invalid')"
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
                                        @click="emit('apply-recent-task', task)"
                                    >
                                        {{ task }}
                                    </UButton>
                                    <UDropdownMenu v-if="hiddenRecentTasks.length > 0" :items="recentTaskMenuItems">
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
                                <UButton color="neutral" variant="outline" size="xs" @click="emit('refresh-apps')">
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
                            v-if="!showFreeActivityOptionsModel"
                            color="neutral"
                            variant="outline"
                            block
                            leading-icon="i-lucide-coffee"
                            @click="showFreeActivityOptionsModel = true"
                        >
                            {{ t("app.takeBreakFree") }}
                        </UButton>

                        <div v-else class="grid grid-cols-3 gap-2 sm:grid-cols-6">
                            <UButton color="neutral" variant="outline" size="sm" @click="emit('start-free-activity', 5)">
                                {{ t("app.minutesShort", { minutes: 5 }) }}
                            </UButton>
                            <UButton color="neutral" variant="outline" size="sm" @click="emit('start-free-activity', 10)">
                                {{ t("app.minutesShort", { minutes: 10 }) }}
                            </UButton>
                            <UButton color="neutral" variant="outline" size="sm" @click="emit('start-free-activity', 15)">
                                {{ t("app.minutesShort", { minutes: 15 }) }}
                            </UButton>
                            <UButton color="neutral" variant="outline" size="sm" @click="emit('start-free-activity', 30)">
                                {{ t("app.minutesShort", { minutes: 30 }) }}
                            </UButton>
                            <UButton color="neutral" variant="outline" size="sm" @click="emit('start-free-activity', 45)">
                                {{ t("app.minutesShort", { minutes: 45 }) }}
                            </UButton>
                            <UInput
                                v-model="customMinutesModel"
                                type="number"
                                min="1"
                                max="480"
                                :placeholder="t('app.custom')"
                                @keyup.enter="triggerCustomBreak"
                            />
                        </div>
                    </template>

                    <UButton
                        color="success"
                        variant="solid"
                        block
                        leading-icon="i-lucide-rocket"
                        @click="emit('start-focus')"
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
                    @load-more="emit('load-more-history')"
                />
            </UCard>
        </div>
    </UCard>
</template>

<style scoped>
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
