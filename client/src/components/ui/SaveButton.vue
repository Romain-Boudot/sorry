<template>
  <button
    class="save-btn"
    :class="{ saving: loading, saved: showCheck }"
    :disabled="disabled || loading"
    @click="$emit('click')"
  >
    <Loader2 v-if="loading" :size="14" class="save-spinner" />
    <Check v-else-if="showCheck" :size="14" class="save-check" />
    <template v-else>{{ label }}</template>
  </button>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import { Loader2, Check } from "lucide-vue-next";

const props = withDefaults(defineProps<{
  loading?: boolean;
  saved?: boolean;
  disabled?: boolean;
  label?: string;
}>(), {
  label: "Sauvegarder",
});

defineEmits<{ click: [] }>();

const showCheck = ref(false);
let timer: ReturnType<typeof setTimeout> | null = null;

watch(() => props.saved, (val) => {
  if (val) {
    showCheck.value = true;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => { showCheck.value = false; }, 2000);
  }
});
</script>

<style scoped>
.save-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  width: auto;
  min-width: 100px;
  height: 32px;
  padding: 0 14px;
  margin: 0;
  border-radius: 6px;
  font-size: 0.8125rem;
  font-weight: 500;
  border: none;
  cursor: pointer;
  background: var(--accent);
  color: var(--text-bright);
  transition: background 0.15s, opacity 0.15s;
  flex-shrink: 0;
}

.save-btn:hover:not(:disabled) {
  opacity: 0.9;
}

.save-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.save-btn.saved {
  background: var(--green, #23a55a);
}

.save-spinner {
  animation: spin 1s linear infinite;
}

.save-check {
  animation: pop 0.2s ease-out;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@keyframes pop {
  0% { transform: scale(0.5); }
  70% { transform: scale(1.15); }
  100% { transform: scale(1); }
}
</style>
