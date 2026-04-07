<template>
  <div v-if="reactions.length || showPicker" class="reactions">
    <button
      v-for="r in reactions"
      :key="r.emoji"
      class="reaction-pill"
      :class="{ active: r.user_ids.includes(myId) }"
      :title="reactionTooltip(r)"
      @click="$emit('toggle', r.emoji)"
    >
      <span class="reaction-emoji">{{ r.emoji }}</span>
      <span class="reaction-count">{{ r.count }}</span>
    </button>
    <button class="reaction-add" title="Ajouter une reaction" @click="$emit('open-picker', $event)">
      <SmilePlus :size="14" />
    </button>
  </div>
</template>

<script setup lang="ts">
import { SmilePlus } from "lucide-vue-next";
import type { Reaction } from "../../api";
import { resolveUser } from "../../store";

defineProps<{
  reactions: Reaction[];
  myId: number;
  showPicker?: boolean;
}>();

defineEmits<{
  toggle: [emoji: string];
  "open-picker": [e: MouseEvent];
}>();

function reactionTooltip(r: Reaction): string {
  const names = r.user_ids.map((id) => resolveUser(id));
  if (names.length <= 3) return names.join(", ");
  return `${names.slice(0, 3).join(", ")} et ${names.length - 3} autre(s)`;
}
</script>

<style scoped>
.reactions {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 4px;
}

.reaction-pill {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  width: auto;
  padding: 2px 8px;
  margin: 0;
  border-radius: 12px;
  background: var(--bg-tertiary);
  border: 1px solid transparent;
  cursor: pointer;
  font-size: 0.8125rem;
  color: var(--text-muted);
  transition: background 0.1s, border-color 0.1s;
}

.reaction-pill:hover {
  background: var(--bg-modifier-hover);
}

.reaction-pill.active {
  border-color: var(--accent);
  background: rgba(88, 101, 242, 0.15);
}

.reaction-emoji {
  font-size: 0.9375rem;
  line-height: 1;
}

.reaction-count {
  font-size: 0.75rem;
  font-weight: 600;
}

.reaction-add {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 26px;
  margin: 0;
  padding: 0;
  border-radius: 12px;
  background: var(--bg-tertiary);
  border: 1px solid transparent;
  cursor: pointer;
  color: var(--text-faint);
  opacity: 0;
  transition: opacity 0.1s, background 0.1s, color 0.1s;
}

.reactions:hover .reaction-add {
  opacity: 1;
}

.reaction-add:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
}
</style>
