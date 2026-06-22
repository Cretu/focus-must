<script setup lang="ts">
import { useI18n } from "vue-i18n";

defineProps<{
    appName: string;
}>();

const emit = defineEmits<{
    (e: "continue"): void;
    (e: "tempAllow"): void;
}>();

const { t } = useI18n();
</script>

<template>
    <div class="space-y-5">
        <UIcon name="i-lucide-eye-off" class="mx-auto block text-6xl text-primary" />
        <h1 class="text-2xl font-semibold">{{ t("app.distractionTitle") }}</h1>

        <UAlert
            color="warning"
            variant="soft"
            :title="t('app.distractionCollected', { name: appName })"
        >
            <template #description>
                {{ t("app.distractionHint") }}
            </template>
        </UAlert>

        <div class="flex justify-center gap-2">
            <UButton
                color="primary"
                variant="solid"
                leading-icon="i-lucide-arrow-left"
                @click="emit('continue')"
            >
                {{ t("app.continueFocus") }}
            </UButton>
            <UButton
                color="neutral"
                variant="outline"
                leading-icon="i-lucide-timer"
                @click="emit('tempAllow')"
            >
                {{ t("app.tempUseOnce") }}
            </UButton>
        </div>

        <p class="text-xs text-muted">{{ t("app.tempUseNote") }}</p>
    </div>
</template>
