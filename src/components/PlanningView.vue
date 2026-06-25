<script setup lang="ts">
import { computed, defineAsyncComponent } from "vue";
import { useI18n } from "vue-i18n";
import { useAppStore } from "../stores/appStore";
import AppGrid from "./AppGrid.vue";

const HistoryList = defineAsyncComponent(() => import("./HistoryList.vue"));

const props = defineProps<{
    snowEnabled: boolean;
}>();

const emit = defineEmits<{
    (event: "update:snowEnabled", value: boolean): void;
}>();

const store = useAppStore();
const { t } = useI18n();

const snowEnabledModel = computed({
    get: () => props.snowEnabled,
    set: (value: boolean) => emit("update:snowEnabled", value),
});

const visibleRecentTasks = computed(() => store.recentTaskSuggestions.slice(0, 3));
const hiddenRecentTasks = computed(() => store.recentTaskSuggestions.slice(3));

const recentTaskMenuItems = computed(() =>
    hiddenRecentTasks.value.map((task) => ({
        label: task,
        onSelect: () => store.applyRecentTask(task),
    })),
);

const MAX_BREAK_MINUTES = 480;

function triggerCustomBreak() {
    const raw = String(store.customMinutes).trim();
    if (!raw) return;
    const minutes = Math.floor(Number(raw));
    if (!Number.isFinite(minutes) || minutes < 1) return;
    store.startFreeActivity(Math.min(minutes, MAX_BREAK_MINUTES));
}

// Allow ⌘/Ctrl + Enter to start a focus session directly from the task box.
function handleTaskKeydown(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
        event.preventDefault();
        store.startFocus();
    }
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
                        @click="store.openSettings()"
                    >
                        {{ t("app.settings") }}
                    </UButton>
                    <UButton
                        color="neutral"
                        variant="outline"
                        size="sm"
                        leading-icon="i-lucide-chart-column"
                        @click="store.openAnalytics()"
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
                                v-model="store.taskDescription"
                                :rows="3"
                                autoresize
                                :color="store.isTaskInputInvalid ? 'error' : 'success'"
                                :highlight="store.isTaskInputInvalid"
                                :placeholder="t('app.taskPlaceholder')"
                                @focus="store.isTaskInputInvalid = false"
                                @keydown="handleTaskKeydown"
                                :class="['w-full', store.isTaskInputShaking ? 'shake' : '']"
                            />

                            <div v-if="store.recentTaskSuggestions.length > 0" class="flex items-center gap-2">
                                <div class="shrink-0 text-xs text-muted">{{ t("app.recentTasks") }}</div>
                                <div class="flex flex-wrap gap-2">
                                    <UButton
                                        v-for="task in visibleRecentTasks"
                                        :key="task"
                                        color="neutral"
                                        variant="outline"
                                        size="xs"
                                        class="max-w-full truncate"
                                        @click="store.applyRecentTask(task)"
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
                                    <span class="text-xs font-normal">({{ t("app.appsNeededHint") }})</span>
                                </div>
                                <UButton color="neutral" variant="outline" size="xs" @click="store.loadApps()">
                                    {{ t("app.refresh") }}
                                </UButton>
                            </div>
                        </template>

                        <AppGrid
                            :apps="store.runningApps"
                            :selected-apps="store.selectedApps"
                            @toggle-app="(id: string) => store.toggleApp(id)"
                        />
                    </UCard>
                </div>

                <div class="mt-3 shrink-0 space-y-2">
                    <template v-if="store.isOnBreak">
                        <UButton color="neutral" variant="soft" block disabled leading-icon="i-lucide-coffee">
                            {{ t("app.breakInProgress", { time: store.breakRemaining }) }}
                        </UButton>
                    </template>

                    <template v-else>
                        <UButton
                            v-if="!store.showFreeActivityOptions"
                            color="neutral"
                            variant="outline"
                            block
                            leading-icon="i-lucide-coffee"
                            @click="store.showFreeActivityOptions = true"
                        >
                            {{ t("app.takeBreakFree") }}
                        </UButton>

                        <div v-else class="grid grid-cols-3 gap-2 sm:grid-cols-6">
                            <UButton color="neutral" variant="outline" size="sm" @click="store.startFreeActivity(5)">
                                {{ t("app.minutesShort", { minutes: 5 }) }}
                            </UButton>
                            <UButton color="neutral" variant="outline" size="sm" @click="store.startFreeActivity(10)">
                                {{ t("app.minutesShort", { minutes: 10 }) }}
                            </UButton>
                            <UButton color="neutral" variant="outline" size="sm" @click="store.startFreeActivity(15)">
                                {{ t("app.minutesShort", { minutes: 15 }) }}
                            </UButton>
                            <UButton color="neutral" variant="outline" size="sm" @click="store.startFreeActivity(30)">
                                {{ t("app.minutesShort", { minutes: 30 }) }}
                            </UButton>
                            <UButton color="neutral" variant="outline" size="sm" @click="store.startFreeActivity(45)">
                                {{ t("app.minutesShort", { minutes: 45 }) }}
                            </UButton>
                            <UInput
                                v-model="store.customMinutes"
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
                        size="lg"
                        class="cta-focus"
                        leading-icon="i-lucide-rocket"
                        @click="store.startFocus()"
                    >
                        {{ t("app.startFocus") }}
                    </UButton>

                    <p class="text-center text-[11px] text-muted">
                        {{ t("app.startFocusShortcutHint") }}
                    </p>
                </div>
            </div>

            <UCard
                variant="outline"
                class="h-full min-h-0 overflow-hidden"
                :ui="{ body: 'h-full min-h-0 overflow-hidden' }"
            >
                <HistoryList
                    :sessions="store.sessionHistory"
                    :has-more="store.historyHasMore"
                    :is-loading="store.historyLoading"
                    @load-more="store.loadMoreHistory()"
                />
            </UCard>
        </div>
    </UCard>
</template>

<style scoped>
.cta-focus {
    font-weight: 600;
    box-shadow: 0 10px 24px -8px rgba(16, 185, 129, 0.55);
    transition:
        transform 0.15s ease,
        box-shadow 0.15s ease;
}

.cta-focus:hover {
    transform: translateY(-1px);
    box-shadow: 0 14px 30px -8px rgba(16, 185, 129, 0.65);
}

.cta-focus:active {
    transform: translateY(0);
}

@media (prefers-reduced-motion: reduce) {
    .cta-focus {
        transition: none;
    }
    .cta-focus:hover {
        transform: none;
    }
}

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
