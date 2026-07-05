<script setup lang="ts">
import { onMounted, onUnmounted, computed, defineAsyncComponent, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { en, zh_cn } from "@nuxt/ui/locale";
import { useAppStore } from "./stores/appStore";
import { useSnowEffect } from "./composables/useSnowEffect";
import { hideSplash } from "./splash";
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

// Detect this window's role from its label. Outside Tauri (plain-browser
// debugging via `npm run dev`) getCurrentWindow() throws — fall back to
// "main" so the app still mounts instead of hanging behind the splash.
const windowLabel = (() => {
    try {
        return getCurrentWindow().label;
    } catch {
        return "main";
    }
})();
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

// The HTML splash (index.html) covers the window from first paint. Fade it
// out once the store has finished booting — or immediately on overlay
// windows, where splash.js should already have suppressed it.
watch(
    () => store.isBooting,
    (booting) => {
        if (!booting && !isOverlay) {
            hideSplash();
        }
    },
);

onMounted(() => {
    if (isOverlay) {
        hideSplash();
    } else {
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

            <!-- While booting, the HTML splash covers this window. -->
            <template v-if="store.isBooting" />

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
            rgba(217, 119, 87, 0.1),
            transparent 44%
        ),
        radial-gradient(
            circle at 84% 88%,
            rgba(217, 119, 87, 0.07),
            transparent 46%
        ),
        radial-gradient(
            circle at 50% -10%,
            rgba(250, 249, 245, 0.06),
            transparent 55%
        ),
        linear-gradient(
            135deg,
            rgba(250, 249, 245, 0.05),
            rgba(250, 249, 245, 0.01)
        ),
        rgba(30, 29, 27, 0.2);
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
        rgba(20, 19, 17, 0.24) 100%
    );
}

:global(html.light) .overlay-container {
    background:
        radial-gradient(
            circle at 16% 12%,
            rgba(217, 119, 87, 0.07),
            transparent 44%
        ),
        radial-gradient(
            circle at 84% 88%,
            rgba(217, 119, 87, 0.05),
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
        rgba(245, 243, 236, 0.2);
}

:global(html.light) .overlay-container::after {
    background: radial-gradient(
        ellipse at center,
        transparent 60%,
        rgba(120, 113, 108, 0.12) 100%
    );
}
</style>
