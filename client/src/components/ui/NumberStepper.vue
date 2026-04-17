<template>
  <div class="number-stepper">
    <button class="step-btn" :disabled="modelValue <= min" @click="decrement">
      <Minus :size="14" />
    </button>
    <input
      type="number"
      :value="modelValue"
      :min="min"
      :max="max"
      class="step-input"
      @input="onInput"
    />
    <button class="step-btn" :disabled="max !== undefined && modelValue >= max" @click="increment">
      <Plus :size="14" />
    </button>
  </div>
</template>

<script setup lang="ts">
import { Minus, Plus } from "lucide-vue-next";

const props = withDefaults(defineProps<{
  modelValue: number;
  min?: number;
  max?: number;
  step?: number;
}>(), {
  min: 0,
  step: 1,
});

const emit = defineEmits<{
  "update:modelValue": [value: number];
}>();

function clamp(val: number) {
  let v = val;
  if (v < props.min) v = props.min;
  if (props.max !== undefined && v > props.max) v = props.max;
  return v;
}

function onInput(e: Event) {
  const raw = (e.target as HTMLInputElement).value;
  emit("update:modelValue", clamp(raw === "" ? 0 : Number(raw)));
}

function increment() {
  emit("update:modelValue", clamp(props.modelValue + props.step));
}

function decrement() {
  emit("update:modelValue", clamp(props.modelValue - props.step));
}
</script>

<style scoped>
.number-stepper {
  display: flex;
  align-items: center;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--bg-tertiary);
  overflow: hidden;
  flex-shrink: 0;
}

.step-btn {
  width: 32px;
  height: 32px;
  padding: 0;
  margin: 0;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: var(--text-muted);
  border: none;
  border-radius: 0;
  cursor: pointer;
  transition: background 0.1s, color 0.1s;
}
.step-btn:hover:not(:disabled) {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
  box-shadow: none;
}
.step-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.step-input {
  flex: 1;
  min-width: 44px;
  width: 44px;
  height: 32px;
  padding: 0;
  margin: 0;
  text-align: center;
  border: none;
  border-left: 1px solid var(--border);
  border-right: 1px solid var(--border);
  background: transparent;
  color: var(--text-normal);
  font-size: 0.8125rem;
  font-family: inherit;
  font-weight: 600;
  outline: none;
  -moz-appearance: textfield;
}
.step-input::-webkit-outer-spin-button,
.step-input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
</style>
