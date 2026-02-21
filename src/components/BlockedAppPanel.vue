<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { BlockedAppEvent } from "../types/contracts";

defineProps<{
    blockedAppState: BlockedAppEvent;
    returnCountdown: number;
}>();

const { t } = useI18n();
</script>

<template>
    <div class="space-y-4">
        <UIcon name="i-lucide-circle-x" class="mx-auto block text-6xl text-error" />
        <UAlert color="error" variant="soft" :title="t('app.blockedDetected')">
            <template #description>
                {{ t("app.openedApp", { name: blockedAppState.name }) }}
            </template>
        </UAlert>

        <template v-if="blockedAppState.return_to_bundle_id">
            <UBadge color="error" variant="soft" class="px-4 py-2 text-base">
                {{ t("app.returnInSeconds", { seconds: returnCountdown }) }}
            </UBadge>
            <UAlert color="neutral" variant="outline">
                <template #description>
                    {{ t("app.returningTo", { name: blockedAppState.return_to_name }) }}
                </template>
            </UAlert>
        </template>

        <UAlert v-else color="warning" variant="soft">
            <template #description>
                <span class="inline-flex items-center gap-1.5">
                    <UIcon name="i-lucide-triangle-alert" class="text-base" />
                    <span>{{ t("app.manualSwitchHint") }}</span>
                </span>
            </template>
        </UAlert>
    </div>
</template>
