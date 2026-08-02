import { createApp } from "vue";
import App from "./App.vue";
import router from "./router";
import "./style.css";
import { isMacOS, isTauriRuntime, resolveIsMacOS } from "./lib/adapters/environment";

async function bootstrap() {
  // Prefer Rust OS (WKWebView UA can lie); fall back to navigator.
  let mac = isMacOS();
  if (isTauriRuntime()) {
    try {
      mac = await resolveIsMacOS();
    } catch {
      /* keep heuristic */
    }
  }
  document.documentElement.dataset.platform = mac ? "macos" : "other";
  document.documentElement.classList.toggle("is-macos", mac);
  // Expose for App.vue sync first paint without a second await race
  (window as unknown as { __OPENMESH_IS_MACOS__?: boolean }).__OPENMESH_IS_MACOS__ = mac;

  createApp(App).use(router).mount("#app");
}

bootstrap();
