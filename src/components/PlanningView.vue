<script setup lang="ts">
import { computed, defineAsyncComponent } from "vue";
import { useI18n } from "vue-i18n";
import AppGrid from "./AppGrid.vue";
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

                        <AppGrid
                            :apps="runningApps"
                            :selected-apps="selectedApps"
                            @toggle-app="(id: string) => emit('toggle-app', id)"
                        />
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
