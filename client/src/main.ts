import { createApp } from "vue";
import App from "./App.vue";
import "./style.css";

if ("__TAURI_INTERNALS__" in window) {
  import("@tauri-apps/api/window").then(({ getCurrentWindow }) => {
    getCurrentWindow().show().catch((e) => console.error("window show failed", e));
  });
}

createApp(App).mount("#app");
