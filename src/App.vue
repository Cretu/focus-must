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
const PausedView = defineAsyncComponent(
    () => import("./components/PausedView.vue"),
);
const SettingsView = defineAsyncComponent(
    () => import("./components/SettingsView.vue"),
);
const AnalyticsView = defineAsyncComponent(
    () => import("./components/AnalyticsView.vue"),
);

// Detect this window's role from its label.
const windowLabel = getCurrentWindow().label;
const isOverlay = windowLabel.startsWith("overlay-");

const store = useAppStore();
const snowEffect = useSnowEffect();
const { snowEnabled } = snowEffect;

const nuxtUiLocale = computed(() =>
    store.effectiveLocale === "en-US" ? en : zh_cn,
);

// During an active session, a manual peek shouldn't dim the screen. Paused is an
// editing screen, so it keeps the glass background.
const isPeeking = computed(() => store.isFocusing && !store.isPaused);

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
        <div v-else class="overlay-container" :class="{ 'is-peeking': isPeeking }">
            <canvas
                :ref="snowEffect.setSnowCanvas"
                class="snow-canvas"
                v-show="snowEnabled"
            ></canvas>

            <UCard v-if="store.isBooting" class="w-[min(420px,86vw)]">
                <div class="space-y-3 text-center">
                    <div class="startup-spinner" aria-hidden="true"></div>
                    <h1 class="brand-title text-xl font-semibold">Focus Must</h1>
                    <UProgress :model-value="null" size="sm" />
                    <p class="text-sm text-muted">
                        {{ $t("app.startupLoadingApps") }}
                    </p>
                </div>
            </UCard>

            <PausedView v-else-if="store.isPaused" />

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
            circle at 16% 12%,
            rgba(16, 185, 129, 0.1),
            transparent 44%
        ),
        radial-gradient(
            circle at 84% 88%,
            rgba(99, 102, 241, 0.1),
            transparent 46%
        ),
        radial-gradient(
            circle at 50% -10%,
            rgba(255, 255, 255, 0.06),
            transparent 55%
        ),
        linear-gradient(
            135deg,
            rgba(255, 255, 255, 0.05),
            rgba(255, 255, 255, 0.01)
        ),
        rgba(15, 23, 42, 0.16);
    backdrop-filter: blur(24px) saturate(135%);
    -webkit-backdrop-filter: blur(24px) saturate(135%);
}

/* Peek mode: no screen dimming during a manual look at an active session.
   The window is transparent, so the live desktop shows through unmasked. */
.overlay-container.is-peeking,
:global(html.light) .overlay-container.is-peeking {
    background: transparent;
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
}

.overlay-container.is-peeking::after {
    display: none;
}

/* Soft vignette to focus attention on the centered card */
.overlay-container::after {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: radial-gradient(
        ellipse at center,
        transparent 55%,
        rgba(2, 6, 23, 0.22) 100%
    );
}

:global(html.light) .overlay-container {
    background:
        radial-gradient(
            circle at 16% 12%,
            rgba(16, 185, 129, 0.06),
            transparent 44%
        ),
        radial-gradient(
            circle at 84% 88%,
            rgba(99, 102, 241, 0.05),
            transparent 46%
        ),
        radial-gradient(
            circle at 50% -10%,
            rgba(255, 255, 255, 0.5),
            transparent 55%
        ),
        linear-gradient(
            135deg,
            rgba(255, 255, 255, 0.08),
            rgba(255, 255, 255, 0.015)
        ),
        rgba(236, 245, 255, 0.16);
}

:global(html.light) .overlay-container::after {
    background: radial-gradient(
        ellipse at center,
        transparent 60%,
        rgba(100, 116, 139, 0.12) 100%
    );
}

.brand-title {
    background: linear-gradient(135deg, #34d399, #818cf8);
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
    letter-spacing: 0.3px;
}

.startup-spinner {
    width: 52px;
    height: 52px;
    margin: 0 auto;
    border-radius: 50%;
    border: 3px solid rgba(148, 163, 184, 0.22);
    border-top-color: #10b981;
    border-right-color: rgba(129, 140, 248, 0.75);
    animation: spin 0.9s linear infinite;
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}
</style>
