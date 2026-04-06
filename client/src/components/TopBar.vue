<template>
  <div class="top-bar" :class="{ 'is-tauri': isTauri }" data-tauri-drag-region>
    <span class="app-name" data-tauri-drag-region>Sorry</span>
    <span v-if="server" class="top-bar-name" data-tauri-drag-region>{{ server.name }}</span>
    <div v-if="isTauri" class="window-controls">
      <button class="wc-btn minimize" title="Réduire" @mousedown.stop @click="minimize">
        <svg width="10" height="1" viewBox="0 0 10 1"><path d="M0 0h10v1H0z" fill="currentColor"/></svg>
      </button>
      <button class="wc-btn maximize" title="Agrandir" @mousedown.stop @click="toggleMaximize">
        <svg width="10" height="10" viewBox="0 0 10 10"><path d="M0 0v10h10V0H0zm1 1h8v8H1V1z" fill="currentColor"/></svg>
      </button>
      <button class="wc-btn close" title="Fermer" @mousedown.stop @click="close">
        <svg width="10" height="10" viewBox="0 0 10 10"><path d="M1.007.293L5 4.286 8.993.293l.714.714L5.714 5l3.993 3.993-.714.714L5 5.714 1.007 9.707l-.714-.714L4.286 5 .293 1.007l.714-.714z" fill="currentColor"/></svg>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted } from "vue";
import { activeServer } from "../store";

const server = computed(() => activeServer());
const isTauri = ref(false);

let appWindow: import("@tauri-apps/api/window").Window | null = null;

onMounted(async () => {
  if ("__TAURI_INTERNALS__" in window) {
    isTauri.value = true;
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    appWindow = getCurrentWindow();
  }
});

function minimize() { appWindow?.minimize(); }
function toggleMaximize() { appWindow?.toggleMaximize(); }
function close() { appWindow?.close(); }
</script>

<style scoped>
.top-bar {
  height: 32px;
  background: var(--bg-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  position: relative;
  user-select: none;
}

.app-name {
  position: absolute;
  left: 12px;
  font-size: 0.8125rem;
  font-weight: 800;
  color: var(--text-muted);
  letter-spacing: -0.02em;
}

.top-bar-name {
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--text-muted);
}

.window-controls {
  position: absolute;
  top: 0;
  right: 0;
  display: flex;
  height: 32px;
}

.wc-btn {
  width: 46px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  color: var(--text-muted);
  font-size: 0.875rem;
  cursor: pointer;
  padding: 0;
  margin: 0;
  border-radius: 0;
  box-shadow: none;
  transition: background 0.1s, color 0.1s;
}

.wc-btn svg {
  pointer-events: none;
  flex-shrink: 0;
}

.wc-btn:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
  box-shadow: none;
}

.wc-btn.close:hover {
  background: #e81123;
  color: var(--text-bright);
}
</style>
