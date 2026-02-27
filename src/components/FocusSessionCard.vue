<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { useAppStore } from "../stores/appStore";
import BlockedAppPanel from "./BlockedAppPanel.vue";

const store = useAppStore();
const { t } = useI18n();

function requestEndFocus() {
    store.showEndConfirm = true;
}

function cancelEndFocus() {
    store.showEndConfirm = false;
}

function confirmEndFocus() {
    store.showEndConfirm = false;
    store.confirmEndFocus();
}
</script>

<template>
    <UCard class="w-[min(760px,90vw)] text-center">
        <BlockedAppPanel
            v-if="store.blockedAppState"
            :blocked-app-state="store.blockedAppState"
            :return-countdown="store.returnCountdown"
        />

        <div v-else class="space-y-5">
            <UIcon name="i-lucide-brain" class="mx-auto block text-6xl text-primary" />
            <h1 class="text-2xl font-semibold">{{ t("app.keepFocus") }}</h1>

            <div class="timer-display">{{ store.formattedTime }}</div>

            <UAlert color="neutral" variant="soft">
                <template #description>
                    {{ store.taskDescription || t("app.focusTaskFallback") }}
                </template>
            </UAlert>

            <div v-if="store.allowedAppNames.length > 0" class="flex flex-wrap justify-center gap-2">
                <UBadge
                    v-for="app in store.allowedAppNames"
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
            v-model:open="store.showEndConfirm"
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
