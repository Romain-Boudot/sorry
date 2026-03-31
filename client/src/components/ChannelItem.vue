<template>
  <div
    v-if="channel.kind === 'text'"
    class="channel-item"
    :class="{ active: channel.id === state?.activeChannelId }"
    :data-channel-id="channel.id"
    @click="selectChannel(channel.id)"
    @contextmenu.prevent.stop="emit('contextmenu', $event)"
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
    :data-channel-id="channel.id"
    @click="handleVoiceClick(channel.id)"
    @contextmenu.prevent.stop="emit('contextmenu', $event)"
  >
    <Volume2 class="channel-icon" :size="20" />
    <span>{{ channel.name }}</span>
  </div>
  <div v-if="channel.kind === 'voice' && voiceUsers.length" class="voice-users">
    <div v-for="[uid, vs] in voiceUsers" :key="uid" class="voice-user">
      <span class="voice-dot" :class="{ speaking: !vs.muted && !vs.deafened && isUserSpeaking(uid) }"></span>
      <span class="voice-user-name">{{ resolveUser(uid) }}</span>
      <MicOff v-if="vs.muted || vs.force_muted" class="voice-status-icon" :class="{ forced: vs.force_muted }" :size="12" />
      <Headphones v-if="vs.deafened || vs.force_deafened" class="voice-status-icon" :class="{ forced: vs.force_deafened }" :size="12" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Hash, Volume2, MicOff, Headphones } from "lucide-vue-next";
import { activeState, selectChannel, resolveUser, joinVoiceChannel, isUserSpeaking } from "../store";
import type { Channel, VoiceUserState } from "../api";

const props = defineProps<{ channel: Channel }>();
const emit = defineEmits<{ contextmenu: [e: MouseEvent] }>();

const state = computed(() => activeState());

const voiceUsers = computed((): [number, VoiceUserState][] => {
  const map = state.value?.voiceState.get(props.channel.id);
  return map ? [...map.entries()] : [];
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
  margin: 3px 8px;
  cursor: pointer;
  color: var(--text-muted);
  border-radius: 8px;
  user-select: none;
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

.voice-user-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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

.voice-status-icon {
  color: var(--text-faint);
  flex-shrink: 0;
}

.voice-status-icon.forced {
  color: var(--danger);
}
</style>
