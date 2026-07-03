<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useAppStore } from "../stores/appStore";
import AppGrid from "./AppGrid.vue";

const store = useAppStore();
const { t } = useI18n();
</script>

<template>
    <UCard class="flex max-h-[80vh] w-[min(820px,92vw)] flex-col overflow-hidden">
        <template #header>
            <div class="flex items-center gap-2">
                <UIcon name="i-lucide-circle-pause" class="text-2xl text-primary" />
                <h1 class="font-serif text-xl font-semibold leading-tight tracking-tight">{{ t("app.pausedTitle") }}</h1>
            </div>
        </template>

        <div class="min-h-0 flex-1 space-y-4 overflow-y-auto pr-1">
            <UAlert color="neutral" variant="soft">
                <template #description>{{ t("app.pausedHint") }}</template>
            </UAlert>

            <UCard variant="soft">
                <template #header>
                    <div class="flex items-center justify-between gap-2">
                        <div class="flex items-center gap-1.5 text-sm font-semibold text-muted">
                            <UIcon name="i-lucide-layout-grid" class="text-base" />
                            <span>{{ t("app.appsNeeded") }}</span>
                            <span class="text-xs font-normal">({{ t("app.appsNeededHint") }})</span>
                        </div>
                        <UButton
                            color="neutral"
                            variant="outline"
                            size="xs"
                            leading-icon="i-lucide-rotate-ccw"
                            :loading="store.appsLoading"
                            @click="store.loadApps()"
                        >
                            {{ t("app.refresh") }}
                        </UButton>
                    </div>
                </template>

                <AppGrid
                    :apps="store.runningApps"
                    :selected-apps="store.pausedSelectedApps"
                    :loading="store.appsLoading"
                    @toggle-app="(id: string) => store.togglePausedApp(id)"
                />
            </UCard>
        </div>

        <template #footer>
            <div class="flex gap-2">
                <UButton
                    color="neutral"
                    variant="outline"
                    leading-icon="i-lucide-square"
                    @click="store.confirmEndFocus()"
                >
                    {{ t("app.endFocus") }}
                </UButton>
                <UButton
                    color="primary"
                    variant="solid"
                    class="flex-1 justify-center text-center"
                    leading-icon="i-lucide-play"
                    @click="store.resumeFocus()"
                >
                    {{ t("app.resumeFocus") }}
                </UButton>
            </div>
        </template>
    </UCard>
</template>
