<template>
  <div class="sidebar">
    <div class="sidebar-header">
      <h2>{{ server?.name }}</h2>
    </div>

    <div class="channel-group">
      <div class="channel-group-title">Channels</div>
      <template v-for="ch in state?.channels" :key="ch.id">
        <div
          v-if="ch.kind === 'text'"
          class="channel-item"
          :class="{ active: ch.id === state?.activeChannelId }"
          @click="selectChannel(ch.id)"
        >
          <Hash class="channel-icon" :size="20" />
          <span>{{ ch.name }}</span>
        </div>
        <div
          v-else
          class="channel-item voice"
          :class="{
            active: ch.id === state?.activeChannelId,
            joined: state?.voiceChannelId === ch.id,
          }"
          @click="handleVoiceClick(ch.id)"
        >
          <Volume2 class="channel-icon" :size="20" />
          <span>{{ ch.name }}</span>
        </div>
        <div v-if="ch.kind === 'voice' && getVoiceUsers(ch.id).length" class="voice-users">
          <div v-for="uid in getVoiceUsers(ch.id)" :key="uid" class="voice-user">
            {{ resolveUser(uid) }}
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Hash, Volume2 } from "lucide-vue-next";
import {
  activeState,
  activeServer,
  selectChannel,
  resolveUser,
  joinVoiceChannel,
} from "../store";

const state = computed(() => activeState());
const server = computed(() => activeServer());

function getVoiceUsers(channelId: number): number[] {
  const users = state.value?.voiceState.get(channelId);
  return users ? [...users] : [];
}

function handleVoiceClick(channelId: number) {
  if (state.value?.voiceChannelId === channelId) {
    state.value.activeChannelId = channelId;
  } else {
    joinVoiceChannel(channelId);
  }
}
</script>
