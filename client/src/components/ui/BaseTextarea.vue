<template>
  <textarea
    ref="textareaEl"
    :value="modelValue"
    :placeholder="placeholder"
    :rows="rows"
    :maxlength="maxlength"
    :disabled="disabled"
    class="base-textarea"
    @input="$emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
    @keydown="$emit('keydown', $event)"
  />
</template>

<script setup lang="ts">
import { ref } from "vue";

withDefaults(defineProps<{
  modelValue?: string;
  placeholder?: string;
  rows?: number;
  maxlength?: number;
  disabled?: boolean;
}>(), {
  rows: 3,
});

defineEmits<{
  "update:modelValue": [value: string];
  keydown: [event: KeyboardEvent];
}>();

const textareaEl = ref<HTMLTextAreaElement>();

function focus() {
  textareaEl.value?.focus();
}

defineExpose({ focus, el: textareaEl });
</script>

<style scoped>
.base-textarea {
  flex: 1;
  min-width: 0;
  padding: 8px 10px;
  border-radius: 6px;
  border: 1px solid transparent;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.875rem;
  font-family: inherit;
  outline: none;
  resize: vertical;
  min-height: 60px;
  transition: border-color 0.15s;
}
.base-textarea:focus {
  border-color: var(--accent);
}
.base-textarea::placeholder {
  color: var(--text-faint);
}
.base-textarea:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
