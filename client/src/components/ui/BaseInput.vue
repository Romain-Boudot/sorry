<template>
  <input
    ref="inputEl"
    :type="type"
    :value="modelValue"
    :placeholder="placeholder"
    :maxlength="maxlength"
    :min="min"
    :max="max"
    :disabled="disabled"
    :inputmode="inputmode"
    :pattern="pattern"
    :required="required"
    :autofocus="autofocus"
    class="base-input"
    @input="onInput"
    @keydown="$emit('keydown', $event)"
  />
</template>

<script setup lang="ts">
import { ref } from "vue";

const props = withDefaults(defineProps<{
  modelValue?: string | number;
  type?: string;
  placeholder?: string;
  maxlength?: number;
  min?: number;
  max?: number;
  disabled?: boolean;
  inputmode?: "text" | "numeric" | "decimal" | "tel" | "email" | "url" | "search" | "none";
  pattern?: string;
  required?: boolean;
  autofocus?: boolean;
}>(), {
  type: "text",
});

const emit = defineEmits<{
  "update:modelValue": [value: string | number];
  keydown: [event: KeyboardEvent];
}>();

const inputEl = ref<HTMLInputElement>();

function onInput(e: Event) {
  const val = (e.target as HTMLInputElement).value;
  emit("update:modelValue", props.type === "number" ? (val === "" ? 0 : Number(val)) : val);
}

function focus() {
  inputEl.value?.focus();
}

defineExpose({ focus, el: inputEl });
</script>

<style scoped>
.base-input {
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
  transition: border-color 0.15s;
}
.base-input:focus {
  border-color: var(--accent);
}
.base-input::placeholder {
  color: var(--text-faint);
}
.base-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* nombre : cacher les fleches natives */
.base-input[type="number"] {
  -moz-appearance: textfield;
}
.base-input[type="number"]::-webkit-outer-spin-button,
.base-input[type="number"]::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
</style>
