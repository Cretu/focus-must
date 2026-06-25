<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "vue-i18n";
import { useAppStore } from "../stores/appStore";

const store = useAppStore();
const { t } = useI18n();

// Dismiss the window and keep the session running (manual peek → back to work).
async function continueFocus() {
    try {
        await invoke("hide_windows");
    } catch (error) {
        console.error("Failed to hide window:", error);
    }
}

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
        <div class="space-y-5">
            <UIcon name="i-lucide-brain" class="focus-brain mx-auto block text-6xl text-primary" />
            <h1 class="text-2xl font-semibold">{{ t("app.keepFocus") }}</h1>

            <div class="timer-display">{{ store.formattedTime }}</div>

            <UAlert color="neutral" variant="soft">
                <template #description>
                    {{ store.currentFocusTask || t("app.focusTaskFallback") }}
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

            <div class="flex justify-center gap-2">
                <UButton
                    color="primary"
                    variant="solid"
                    leading-icon="i-lucide-arrow-right"
                    @click="continueFocus"
                >
                    {{ t("app.continueFocus") }}
                </UButton>
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

        <UModal
            v-model:open="store.showBreakReminder"
            :title="t('app.breakReminderTitle')"
            :description="t('app.breakReminderDesc', { minutes: store.breakReminderMinutes })"
            :dismissible="false"
        >
            <template #footer>
                <div class="flex w-full justify-end gap-2">
                    <UButton
                        color="neutral"
                        variant="outline"
                        leading-icon="i-lucide-arrow-right"
                        @click="store.dismissBreakReminder()"
                    >
                        {{ t("app.continueFocus") }}
                    </UButton>
                    <UButton
                        color="primary"
                        leading-icon="i-lucide-coffee"
                        @click="store.startBreakFromReminder(5)"
                    >
                        {{ t("app.startBreakNow", { minutes: 5 }) }}
                    </UButton>
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
    background: linear-gradient(135deg, #34d399, #818cf8);
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
    line-height: 1.05;
}

.focus-brain {
    filter: drop-shadow(0 0 14px rgba(16, 185, 129, 0.35));
    animation: brain-breathe 3.6s ease-in-out infinite;
}

@keyframes brain-breathe {
    0%,
    100% {
        opacity: 0.9;
        transform: scale(1);
    }
    50% {
        opacity: 1;
        transform: scale(1.06);
    }
}

@media (prefers-reduced-motion: reduce) {
    .focus-brain {
        animation: none;
    }
}
</style>
