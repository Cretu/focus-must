import { createApp } from "vue";
import ui from "@nuxt/ui/vue-plugin";
import App from "./App.vue";
import "./styles/main.css";

createApp(App).use(ui).mount("#app");
