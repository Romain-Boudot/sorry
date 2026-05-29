<template>
  <Teleport to="body">
  <div class="toast-container">
    <TransitionGroup name="toast">
      <div
        v-for="toast in toasts"
        :key="toast.id"
        class="toast"
        :class="toast.type"
      >
        <Check v-if="toast.type === 'success'" :size="14" />
        <AlertCircle v-else-if="toast.type === 'error'" :size="14" />
        <Info v-else :size="14" />
        <span>{{ toast.message }}</span>
      </div>
    </TransitionGroup>
  </div>
  </Teleport>
</template>

<script setup lang="ts">
import { Check, AlertCircle, Info } from "lucide-vue-next";
import { toasts } from "../../composables/useToast";
</script>

<style scoped>
.toast-container {
  position: fixed;
  top: 52px;
  right: 16px;
  z-index: 300;
  display: flex;
  flex-direction: column;
  gap: 8px;
  pointer-events: none;
}

.toast {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 16px;
  border-radius: 8px;
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-bright);
  background: var(--bg-floating);
  border: 1px solid var(--border);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  pointer-events: auto;
}

.toast.success {
  border-left: 3px solid var(--green);
}

.toast.success svg {
  color: var(--green);
}

.toast.error {
  border-left: 3px solid var(--danger);
}

.toast.error svg {
  color: var(--danger);
}

.toast.info {
  border-left: 3px solid var(--accent);
}

.toast.info svg {
  color: var(--accent);
}

.toast-enter-active {
  transition: all 0.3s ease;
}

.toast-leave-active {
  transition: all 0.2s ease;
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(40px);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(40px);
}
</style>
