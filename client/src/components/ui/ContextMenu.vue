<template>
  <Teleport to="body">
    <div class="context-menu-overlay" @click="close" @contextmenu.prevent="close">
      <div
        class="context-menu"
        :style="{ top: `${y}px`, left: `${x}px` }"
        @click.stop
      >
        <div
          v-for="item in items"
          :key="item.label"
          class="context-menu-item"
          :class="{ danger: item.danger }"
          @click="item.action(); if (!item.keepOpen) close()"
        >
          <component v-if="item.icon" :is="item.icon" :size="14" />
          {{ item.label }}
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import type { Component } from "vue";

export interface MenuItem {
  label: string;
  action: () => void;
  danger?: boolean;
  icon?: Component;
  keepOpen?: boolean;
}

defineProps<{
  x: number;
  y: number;
  items: MenuItem[];
}>();

const emit = defineEmits<{ close: [] }>();

function close() {
  emit("close");
}
</script>

<style scoped>
.context-menu-overlay {
  position: fixed;
  inset: 0;
  z-index: 200;
}

.context-menu {
  position: fixed;
  background: var(--bg-tertiary);
  border-radius: 8px;
  padding: 8px;
  min-width: 200px;
  box-shadow: 0 8px 16px rgba(0, 0, 0, 0.24);
  z-index: 201;
}

.context-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-muted);
  user-select: none;
}

.context-menu-item:hover {
  background: var(--accent);
  color: var(--accent-fg);
}

.context-menu-item.danger { color: var(--danger); }
.context-menu-item.danger:hover { background: var(--danger); color: var(--text-bright); }
</style>
