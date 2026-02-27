import { createApp } from "vue";
import { createPinia } from "pinia";
import ui from "@nuxt/ui/vue-plugin";
import App from "./App.vue";
import { i18n } from "./i18n";
import "./styles/main.css";

createApp(App).use(createPinia()).use(i18n).use(ui).mount("#app");
