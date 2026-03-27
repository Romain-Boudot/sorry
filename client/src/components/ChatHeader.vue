<template>
  <div class="chat-header">
    <Volume2 v-if="isVoice" class="channel-icon" :size="18" />
    <Hash v-else class="channel-icon" :size="18" />
    <span>{{ activeChannel?.name }}</span>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Hash, Volume2 } from "lucide-vue-next";
import { activeState, isActiveChannelVoice } from "../store";

const state = computed(() => activeState());
const isVoice = computed(() => isActiveChannelVoice());
const activeChannel = computed(() =>
  state.value?.channels.find((c) => c.id === state.value?.activeChannelId)
);
</script>

<style scoped>
.chat-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 16px;
  height: 48px;
  font-weight: 600;
  font-size: 0.9375rem;
  color: var(--header-primary);
  background: var(--bg-primary);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.channel-icon {
  color: var(--text-muted);
}
</style>
