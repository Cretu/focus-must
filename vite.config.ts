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
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
