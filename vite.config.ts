import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import ui from "@nuxt/ui/vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  // Warm, Anthropic-inspired palette: the accent is a custom terracotta scale
  // ("clay", defined in src/styles/main.css via @theme) and neutrals use the
  // warm `stone` scale so surfaces read ivory/charcoal instead of blue-gray.
  plugins: [
    vue(),
    ui({ ui: { colors: { primary: "clay", neutral: "stone" } } }),
  ],

  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes("node_modules")) {
            return;
          }

          if (id.includes("@nuxt/ui") || id.includes("@iconify")) {
            return "nuxt-ui";
          }

          if (
            id.includes("/vue/") ||
            id.includes("/@vue/") ||
            id.includes("vue-router") ||
            id.includes("vue-i18n")
          ) {
            return "vue-vendor";
          }

          if (id.includes("@tauri-apps")) {
            return "tauri-vendor";
          }
        },
      },
    },
  },

  // Pre-bundle the deps hit during boot so the dev server never pauses
  // mid-startup to optimize a newly discovered dependency (which triggers a
  // full page reload while the splash is up). @nuxt/ui manages its own
  // optimizeDeps entries via its Vite plugin.
  optimizeDeps: {
    include: [
      "vue",
      "pinia",
      "vue-i18n",
      "@iconify/vue",
      "@tauri-apps/api/core",
      "@tauri-apps/api/event",
      "@tauri-apps/api/window",
      "@tauri-apps/plugin-autostart",
    ],
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    // Transform the boot-critical module graph while `tauri dev` is still
    // compiling Rust, so the first window paint isn't stuck waiting on
    // on-demand transforms.
    warmup: {
      clientFiles: ["./src/main.ts", "./src/components/PlanningView.vue"],
    },
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      // Also ignore the d.ts files that unplugin-auto-import/-components
      // rewrite on dev-server boot — watching them triggers a full page
      // reload (splash flicker) right as the app starts.
      ignored: [
        "**/src-tauri/**",
        "**/auto-imports.d.ts",
        "**/components.d.ts",
      ],
    },
  },
}));
