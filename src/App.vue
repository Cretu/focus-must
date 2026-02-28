<script setup lang="ts">
import { onMounted, onUnmounted, computed, defineAsyncComponent } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { en, zh_cn } from "@nuxt/ui/locale";
import { useAppStore } from "./stores/appStore";
import { useSnowEffect } from "./composables/useSnowEffect";
import { localeOptionsWithText } from "./i18n";

const OverlayView = defineAsyncComponent(
    () => import("./components/OverlayView.vue"),
);
const PlanningView = defineAsyncComponent(
    () => import("./components/PlanningView.vue"),
);
const FocusSessionCard = defineAsyncComponent(
    () => import("./components/FocusSessionCard.vue"),
);
const SettingsView = defineAsyncComponent(
    () => import("./components/SettingsView.vue"),
);
const AnalyticsView = defineAsyncComponent(
    () => import("./components/AnalyticsView.vue"),
);

// Detect if this window is an overlay (secondary monitor)
const isOverlay = getCurrentWindow().label.startsWith("overlay-");

const store = useAppStore();
const snowEffect = useSnowEffect();
const { snowEnabled } = snowEffect;

const nuxtUiLocale = computed(() =>
    store.effectiveLocale === "en-US" ? en : zh_cn,
);

onMounted(() => {
    if (!isOverlay) {
        store.initialize();
    }
});

onUnmounted(() => {
    if (!isOverlay) {
        store.cleanup();
    }
});
</script>

<template>
    <UApp :locale="nuxtUiLocale">
        <!-- Overlay window: dedicated focus view -->
        <OverlayView v-if="isOverlay" />

        <!-- Main window: full app -->
        <div v-else class="overlay-container">
            <canvas
                :ref="snowEffect.setSnowCanvas"
                class="snow-canvas"
                v-show="snowEnabled"
            ></canvas>

            <UCard v-if="store.isBooting" class="w-[min(420px,86vw)]">
                <div class="space-y-3 text-center">
                    <div class="startup-spinner" aria-hidden="true"></div>
                    <h1 class="text-xl font-semibold">Focus Must</h1>
                    <UProgress :model-value="null" size="sm" />
                    <p class="text-sm text-muted">
                        {{ $t("app.startupLoadingApps") }}
                    </p>
                </div>
            </UCard>

            <PlanningView
                v-else-if="!store.isFocusing && store.currentView === 'planning'"
                v-model:snow-enabled="snowEnabled"
            />

            <SettingsView
                v-else-if="!store.isFocusing && store.currentView === 'settings'"
                :locale-options-with-text="localeOptionsWithText"
            />

            <AnalyticsView
                v-else-if="!store.isFocusing && store.currentView === 'analytics'"
            />

            <FocusSessionCard
                v-else-if="store.isFocusing"
            />
        </div>
    </UApp>
</template>

<style scoped>
.snow-canvas {
    position: fixed;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 100000;
}

.overlay-container {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    background:
        radial-gradient(
            circle at 18% 14%,
            rgba(59, 130, 246, 0.08),
            transparent 42%
        ),
        radial-gradient(
            circle at 82% 86%,
            rgba(16, 185, 129, 0.06),
            transparent 44%
        ),
        linear-gradient(
            135deg,
            rgba(255, 255, 255, 0.05),
            rgba(255, 255, 255, 0.01)
        ),
        rgba(20, 28, 44, 0.12);
    backdrop-filter: blur(22px) saturate(130%);
    -webkit-backdrop-filter: blur(22px) saturate(130%);
}

:global(html.light) .overlay-container {
    background:
        radial-gradient(
            circle at 18% 14%,
            rgba(59, 130, 246, 0.035),
            transparent 42%
        ),
        radial-gradient(
            circle at 82% 86%,
            rgba(16, 185, 129, 0.03),
            transparent 44%
        ),
        linear-gradient(
            135deg,
            rgba(255, 255, 255, 0.08),
            rgba(255, 255, 255, 0.015)
        ),
        rgba(236, 245, 255, 0.14);
}

.startup-spinner {
    width: 52px;
    height: 52px;
    margin: 0 auto;
    border-radius: 50%;
    border: 3px solid rgba(255, 255, 255, 0.2);
    border-top-color: rgba(16, 185, 129, 0.95);
    animation: spin 0.9s linear infinite;
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}
</style>
