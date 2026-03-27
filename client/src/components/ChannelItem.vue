<template>
  <div
    v-if="channel.kind === 'text'"
    class="channel-item"
    :class="{ active: channel.id === state?.activeChannelId }"
    @click="selectChannel(channel.id)"
  >
    <Hash class="channel-icon" :size="20" />
    <span>{{ channel.name }}</span>
  </div>
  <div
    v-else
    class="channel-item voice"
    :class="{
      active: channel.id === state?.activeChannelId,
      joined: state?.voiceChannelId === channel.id,
    }"
    @click="handleVoiceClick(channel.id)"
  >
    <Volume2 class="channel-icon" :size="20" />
    <span>{{ channel.name }}</span>
  </div>
  <div v-if="channel.kind === 'voice' && voiceUsers.length" class="voice-users">
    <div v-for="uid in voiceUsers" :key="uid" class="voice-user">
      <span class="voice-dot" :class="{ speaking: isUserSpeaking(uid) }"></span>
      {{ resolveUser(uid) }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Hash, Volume2 } from "lucide-vue-next";
import { activeState, selectChannel, resolveUser, joinVoiceChannel, isUserSpeaking } from "../store";
import type { Channel } from "../api";

const props = defineProps<{ channel: Channel }>();

const state = computed(() => activeState());

const voiceUsers = computed(() => {
  const users = state.value?.voiceState.get(props.channel.id);
  return users ? [...users] : [];
});

function handleVoiceClick(channelId: number) {
  if (state.value?.voiceChannelId === channelId) {
    state.value.activeChannelId = channelId;
  } else {
    joinVoiceChannel(channelId);
  }
}
</script>

<style scoped>
.channel-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  margin: 1px 8px;
  cursor: pointer;
  color: var(--text-muted);
  border-radius: 4px;
  font-size: 0.9375rem;
  font-weight: 500;
  transition: background 0.1s, color 0.1s;
}

.channel-item:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
}

.channel-item.active {
  background: var(--bg-modifier-active);
  color: var(--header-primary);
}

.channel-icon {
  color: var(--text-faint);
  width: 20px;
  flex-shrink: 0;
}

.channel-item.active .channel-icon {
  color: var(--text-normal);
}

.channel-item.joined .channel-icon {
  color: var(--green);
}

.voice-users {
  padding: 0 0 2px 48px;
}

.voice-user {
  font-size: 0.8125rem;
  color: var(--text-muted);
  padding: 2px 0;
  display: flex;
  align-items: center;
  gap: 6px;
}

.voice-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--text-faint);
  flex-shrink: 0;
  transition: background 0.15s;
}

.voice-dot.speaking {
  background: var(--green);
}
</style>
