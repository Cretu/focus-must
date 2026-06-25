<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import BlockedAppPanel from "./BlockedAppPanel.vue";
import type { BlockedAppEvent } from "../types/contracts";

const appName = ref("");
const bundleId = ref("");
let unlisten: UnlistenFn | null = null;

function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
        event.preventDefault();
        continueFocus();
    }
}

onMounted(async () => {
    window.addEventListener("keydown", onKeydown);
    unlisten = await listen<BlockedAppEvent>("blocked-app", (event) => {
        appName.value = event.payload.name;
        bundleId.value = event.payload.bundle_id;
    });
});

onUnmounted(() => {
    window.removeEventListener("keydown", onKeydown);
    if (unlisten) unlisten();
});

function continueFocus() {
    invoke("dismiss_distraction").catch((error) => {
        console.error("Failed to dismiss distraction:", error);
    });
}

function useOnce() {
    invoke("allow_app_temporarily", {
        bundleId: bundleId.value,
        durationMinutes: 2,
    }).catch((error) => {
        console.error("Failed to grant temporary pass:", error);
    });
}
</script>

<template>
    <div class="prompt-card">
        <BlockedAppPanel
            :app-name="appName"
            @continue="continueFocus"
            @temp-allow="useOnce"
        />
    </div>
</template>

<style scoped>
.prompt-card {
    position: fixed;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    border-radius: 18px;
    background:
        radial-gradient(circle at 20% 0%, rgba(16, 185, 129, 0.12), transparent 60%),
        radial-gradient(circle at 100% 100%, rgba(99, 102, 241, 0.12), transparent 60%),
        rgba(15, 23, 42, 0.97);
    border: 1px solid rgba(148, 163, 184, 0.18);
    box-shadow: 0 24px 60px rgba(2, 6, 23, 0.55);
    color: rgba(255, 255, 255, 0.92);
    user-select: none;
}

:global(html.light) .prompt-card {
    background:
        radial-gradient(circle at 20% 0%, rgba(16, 185, 129, 0.1), transparent 60%),
        radial-gradient(circle at 100% 100%, rgba(99, 102, 241, 0.1), transparent 60%),
        rgba(248, 250, 252, 0.98);
    border: 1px solid rgba(148, 163, 184, 0.25);
    color: rgba(15, 23, 42, 0.9);
}
</style>
