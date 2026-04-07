<template>
  <div class="emoji-picker" ref="pickerEl" :style="{ left: x + 'px', top: y + 'px' }">
    <div class="emoji-grid">
      <button
        v-for="emoji in emojis"
        :key="emoji"
        class="emoji-btn"
        @click="$emit('select', emoji)"
      >
        {{ emoji }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";

defineProps<{
  x: number;
  y: number;
}>();

const emit = defineEmits<{
  select: [emoji: string];
  close: [];
}>();

const pickerEl = ref<HTMLElement>();

const emojis = [
  // Smileys
  "😀", "😂", "🥲", "😍", "🤩", "😘", "😎", "🤔",
  "😅", "😭", "🥺", "😤", "🤯", "😱", "🫡", "🤡",
  // Hands
  "👍", "👎", "👏", "🙌", "🤝", "✌️", "🤞", "💪",
  // Hearts
  "❤️", "🧡", "💛", "💚", "💙", "💜", "🖤", "💔",
  // Symbols
  "✅", "❌", "⭐", "🔥", "💯", "🎉", "🚀", "💡",
  // Misc
  "👀", "🙏", "💀", "🫠", "🤖", "👻", "🎵", "☕",
];

function onClickOutside(e: MouseEvent) {
  if (pickerEl.value && !pickerEl.value.contains(e.target as Node)) {
    emit("close");
  }
}

onMounted(() => {
  setTimeout(() => document.addEventListener("click", onClickOutside), 0);
});

onUnmounted(() => {
  document.removeEventListener("click", onClickOutside);
});
</script>

<style scoped>
.emoji-picker {
  position: fixed;
  z-index: 50;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
}

.emoji-grid {
  display: grid;
  grid-template-columns: repeat(8, 1fr);
  gap: 2px;
}

.emoji-btn {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: transparent;
  border: none;
  cursor: pointer;
  font-size: 1.125rem;
  padding: 0;
  transition: background 0.1s;
}

.emoji-btn:hover {
  background: var(--bg-modifier-hover);
}
</style>
