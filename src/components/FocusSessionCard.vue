<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import BlockedAppPanel from "./BlockedAppPanel.vue";
import type { AppInfo, BlockedAppEvent } from "../types/contracts";

const props = defineProps<{
    blockedAppState: BlockedAppEvent | null;
    returnCountdown: number;
    formattedTime: string;
    taskDescription: string;
    allowedAppNames: AppInfo[];
    showEndConfirm: boolean;
}>();

const emit = defineEmits<{
    (event: "update:showEndConfirm", value: boolean): void;
    (event: "confirm-end"): void;
}>();

const { t } = useI18n();

const endConfirmOpen = computed({
    get: () => props.showEndConfirm,
    set: (value: boolean) => emit("update:showEndConfirm", value),
});

function requestEndFocus() {
    endConfirmOpen.value = true;
}

function cancelEndFocus() {
    endConfirmOpen.value = false;
}

function confirmEndFocus() {
    endConfirmOpen.value = false;
    emit("confirm-end");
}
</script>

<template>
    <UCard class="w-[min(760px,90vw)] text-center">
        <BlockedAppPanel
            v-if="blockedAppState"
            :blocked-app-state="blockedAppState"
            :return-countdown="returnCountdown"
        />

        <div v-else class="space-y-5">
            <UIcon name="i-lucide-brain" class="mx-auto block text-6xl text-primary" />
            <h1 class="text-2xl font-semibold">{{ t("app.keepFocus") }}</h1>

            <div class="timer-display">{{ formattedTime }}</div>

            <UAlert color="neutral" variant="soft">
                <template #description>
                    {{ taskDescription || t("app.focusTaskFallback") }}
                </template>
            </UAlert>

            <div v-if="allowedAppNames.length > 0" class="flex flex-wrap justify-center gap-2">
                <UBadge
                    v-for="app in allowedAppNames"
                    :key="app.bundle_id"
                    color="success"
                    variant="soft"
                    :label="app.name"
                />
            </div>

            <UAlert color="neutral" variant="outline">
                <template #description>
                    {{ t("app.focusHint") }}
                </template>
            </UAlert>

            <div class="flex justify-center">
                <UButton
                    color="neutral"
                    variant="outline"
                    leading-icon="i-lucide-square"
                    @click="requestEndFocus"
                >
                    {{ t("app.endFocus") }}
                </UButton>
            </div>
        </div>

        <UModal
            v-model:open="endConfirmOpen"
            :title="t('app.confirmEndTitle')"
            :description="t('app.confirmEndDescription')"
        >
            <template #footer>
                <div class="flex w-full justify-end gap-2">
                    <UButton color="neutral" variant="outline" @click="cancelEndFocus">
                        {{ t("app.cancel") }}
                    </UButton>
                    <UButton color="primary" @click="confirmEndFocus">{{ t("app.confirmEnd") }}</UButton>
                </div>
            </template>
        </UModal>
    </UCard>
</template>

<style scoped>
.timer-display {
    font-size: 56px;
    font-weight: 700;
    font-feature-settings: "tnum";
    font-variant-numeric: tabular-nums;
    letter-spacing: 2px;
}
</style>
