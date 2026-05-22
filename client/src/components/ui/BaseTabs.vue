<template>
  <div class="base-tabs" :class="[variant]">
    <button
      v-for="item in items"
      :key="item.id"
      type="button"
      class="tab-btn"
      :class="{ active: modelValue === item.id }"
      @click="$emit('update:modelValue', item.id)"
    >
      <component v-if="item.icon" :is="item.icon" :size="14" />
      <span class="tab-label">{{ item.label }}</span>
      <span v-if="item.count !== undefined && item.count > 0" class="tab-count">{{ item.count }}</span>
    </button>
  </div>
</template>

<script setup lang="ts">
import type { Component } from "vue";

export interface TabItem {
  id: string;
  label: string;
  icon?: Component;
  count?: number;
}

withDefaults(defineProps<{
  modelValue: string;
  items: TabItem[];
  /** "segmented" (pill-style), "stretched" (equal-width fills container) */
  variant?: "segmented" | "stretched";
}>(), {
  variant: "segmented",
});

defineEmits<{
  "update:modelValue": [value: string];
}>();
</script>

<style scoped>
.base-tabs {
  display: inline-flex;
  background: var(--bg-tertiary);
  border-radius: 8px;
  padding: 3px;
  gap: 2px;
}

.base-tabs.stretched {
  display: flex;
  width: 100%;
}

.tab-btn {
  width: auto;
  margin: 0;
  padding: 6px 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  background: transparent;
  color: var(--text-muted);
  font-size: 0.8125rem;
  font-weight: 600;
  border-radius: 6px;
  border: none;
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
  box-shadow: none;
}

.base-tabs.stretched .tab-btn {
  flex: 1;
}

.tab-btn:hover:not(.active) {
  color: var(--text-normal);
  background: var(--bg-modifier-hover);
}

.tab-btn.active {
  background: var(--bg-primary);
  color: var(--header-primary);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
}

.tab-count {
  font-size: 0.6875rem;
  font-weight: 700;
  padding: 1px 6px;
  border-radius: 999px;
  background: var(--bg-modifier-active);
  color: var(--text-muted);
  min-width: 18px;
  text-align: center;
}
.tab-btn.active .tab-count {
  background: var(--accent);
  color: var(--accent-fg);
}
</style>
