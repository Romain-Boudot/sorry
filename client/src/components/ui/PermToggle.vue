<template>
  <div class="perm-toggle" :class="{ dual: mode === 'dual', disabled }">
    <button
      v-for="opt in options"
      :key="opt.value"
      class="toggle-btn"
      :class="[opt.value, { active: modelValue === opt.value }]"
      :title="opt.label"
      :disabled="disabled"
      @click="!disabled && $emit('update:modelValue', opt.value)"
    >{{ opt.icon }}</button>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

export type TriState = "inherit" | "allow" | "deny";
export type DualState = "allow" | "deny";

const props = withDefaults(defineProps<{
  modelValue: TriState | DualState;
  mode?: "tri" | "dual";
  disabled?: boolean;
}>(), {
  mode: "tri",
  disabled: false,
});

defineEmits<{
  "update:modelValue": [value: TriState | DualState];
}>();

const options = computed(() => {
  if (props.mode === "dual") {
    return [
      { value: "allow" as const, label: "Autoriser", icon: "\u2713" },
      { value: "deny" as const, label: "Refuser", icon: "\u2715" },
    ];
  }
  return [
    { value: "inherit" as const, label: "Heriter", icon: "/" },
    { value: "allow" as const, label: "Autoriser", icon: "\u2713" },
    { value: "deny" as const, label: "Refuser", icon: "\u2715" },
  ];
});
</script>

<style scoped>
.perm-toggle {
  display: flex;
  border-radius: 6px;
  overflow: hidden;
  border: 1px solid var(--border);
}

.toggle-btn {
  width: 28px;
  height: 24px;
  padding: 0;
  margin: 0;
  font-size: 0.75rem;
  background: var(--bg-tertiary);
  color: var(--text-faint);
  cursor: pointer;
  border: none;
  border-radius: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.1s, color 0.1s;
}
.toggle-btn:not(:last-child) { border-right: 1px solid var(--border); }
.toggle-btn:hover { background: var(--bg-modifier-hover); box-shadow: none; }

.toggle-btn.active.inherit { background: var(--bg-modifier-active); color: var(--text-normal); }
.toggle-btn.active.allow { background: var(--green-bg); color: var(--green); }
.toggle-btn.active.deny { background: var(--danger-bg-hover); color: var(--danger); }
</style>
