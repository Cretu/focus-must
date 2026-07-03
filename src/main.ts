import { createApp } from "vue";
import { createPinia } from "pinia";
import ui from "@nuxt/ui/vue-plugin";
import { addCollection } from "@iconify/vue";
import App from "./App.vue";
import { i18n } from "./i18n";
import lucideIcons from "./generated/lucide-icons";
import "./styles/main.css";

// Register the bundled icon set before mounting so every UIcon renders
// immediately (and offline) instead of fetching from the Iconify API.
addCollection(lucideIcons);

createApp(App).use(createPinia()).use(i18n).use(ui).mount("#app");
