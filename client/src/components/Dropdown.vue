<template>
  <div class="dropdown" ref="dropdownRef">
    <div class="dropdown-trigger" @click="open = !open">
      <span class="dropdown-value">{{ selectedLabel || placeholder }}</span>
      <ChevronDown :size="14" class="dropdown-arrow" :class="{ flipped: open }" />
    </div>

    <div class="dropdown-menu" v-if="open">
      <div
        v-for="option in options"
        :key="option.value"
        class="dropdown-item"
        :class="{ active: option.value === modelValue }"
        @click="select(option.value)"
      >
        {{ option.label }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { ChevronDown } from "lucide-vue-next";

export interface DropdownOption {
  value: string;
  label: string;
}

const props = defineProps<{
  modelValue: string;
  options: DropdownOption[];
  placeholder?: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const open = ref(false);
const dropdownRef = ref<HTMLElement>();

const selectedLabel = computed(() =>
  props.options.find((o) => o.value === props.modelValue)?.label
);

function select(value: string) {
  emit("update:modelValue", value);
  open.value = false;
}

function onClickOutside(e: MouseEvent) {
  if (dropdownRef.value && !dropdownRef.value.contains(e.target as Node)) {
    open.value = false;
  }
}

onMounted(() => document.addEventListener("click", onClickOutside));
onUnmounted(() => document.removeEventListener("click", onClickOutside));
</script>

<style scoped>
.dropdown {
  position: relative;
  width: 100%;
}

.dropdown-trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  border-radius: 8px;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.875rem;
  cursor: pointer;
  transition: background 0.1s;
}

.dropdown-trigger:hover {
  background: var(--bg-modifier-hover);
}

.dropdown-value {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dropdown-arrow {
  color: var(--text-faint);
  flex-shrink: 0;
  transition: transform 0.15s;
}

.dropdown-arrow.flipped {
  transform: rotate(180deg);
}

.dropdown-menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  background: var(--bg-tertiary);
  border-radius: 8px;
  padding: 4px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  z-index: 50;
  max-height: 200px;
  overflow-y: auto;
}

.dropdown-item {
  padding: 8px 10px;
  border-radius: 8px;
  font-size: 0.8125rem;
  color: var(--text-muted);
  cursor: pointer;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  transition: background 0.08s, color 0.08s;
}

.dropdown-item:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
}

.dropdown-item.active {
  color: var(--header-primary);
  background: var(--bg-modifier-active);
}
</style>
