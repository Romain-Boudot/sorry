/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

// Stubs until `bun install` pulls in the real types. Both plugins are loaded
// only via dynamic import inside Tauri (no-op in the web build).
declare module "@tauri-apps/plugin-updater";
declare module "@tauri-apps/plugin-process";
