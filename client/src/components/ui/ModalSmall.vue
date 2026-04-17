<template>
  <div class="modal-overlay" @click.self="emit('close')">
    <div class="modal-small">
      <h3>{{ title }}</h3>
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";

defineProps<{ title: string }>();
const emit = defineEmits<{ close: [] }>();

onMounted(() => document.addEventListener("keydown", onKey));
onUnmounted(() => document.removeEventListener("keydown", onKey));
function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
}
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: var(--overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.modal-small {
  background: var(--bg-primary);
  padding: 24px;
  border-radius: 8px;
  width: 360px;
}

.modal-small h3 {
  font-size: 1rem;
  font-weight: 700;
  color: var(--header-primary);
  margin-bottom: 16px;
}

:slotted(.field) {
  margin-bottom: 14px;
}

:slotted(.field label) {
  display: block;
  font-size: 0.6875rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-muted);
  margin-bottom: 6px;
}

:slotted(.field input),
:slotted(.field select) {
  width: 100%;
  padding: 8px 10px;
  border-radius: 6px;
  border: none;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.875rem;
  font-family: inherit;
  outline: none;
}

:slotted(.field input::placeholder) {
  color: var(--text-faint);
}

:slotted(.modal-actions) {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  margin-top: 16px;
}

:slotted(.modal-actions button) {
  width: auto;
  padding: 8px 16px;
  font-size: 0.8125rem;
  border-radius: 6px;
}

:slotted(.btn-cancel) {
  background: transparent;
  color: var(--text-muted);
}

:slotted(.btn-cancel:hover) {
  color: var(--text-normal);
  background: transparent;
}
</style>
