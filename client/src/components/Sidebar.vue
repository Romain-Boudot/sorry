<template>
  <div class="sidebar">
    <div class="sidebar-header">
      <h2>{{ server?.name }}</h2>
    </div>

    <div class="channel-group">
      <div class="channel-group-title">Text</div>
      <div
        v-for="ch in textChannels"
        :key="ch.id"
        class="channel-item"
        :class="{ active: ch.id === state?.activeChannelId }"
        @click="selectChannel(ch.id)"
      >
        <Hash class="channel-icon" :size="18" />
        <span>{{ ch.name }}</span>
      </div>
    </div>

    <div class="channel-group">
      <div class="channel-group-title">Voice</div>
      <div
        v-for="ch in voiceChannels"
        :key="ch.id"
        class="channel-item voice"
        :class="{ active: state?.voiceChannelId === ch.id }"
        @click="handleVoiceClick(ch.id)"
      >
        <Volume2 class="channel-icon" :size="18" />
        <span>{{ ch.name }}</span>
        <div class="voice-users" v-if="getVoiceUsers(ch.id).length">
          <div v-for="uid in getVoiceUsers(ch.id)" :key="uid" class="voice-user">
            {{ resolveUser(uid) }}
          </div>
        </div>
      </div>
    </div>

    <div class="sidebar-footer">
      <div class="user-info">
        <span class="user-avatar">{{ state?.user?.display_name?.[0]?.toUpperCase() }}</span>
        <span class="user-name">{{ state?.user?.display_name }}</span>
      </div>
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
  leaveVoiceChannel,
} from "../store";

const state = computed(() => activeState());
const server = computed(() => activeServer());

const textChannels = computed(() =>
  state.value?.channels.filter((c) => c.kind === "text") ?? []
);

const voiceChannels = computed(() =>
  state.value?.channels.filter((c) => c.kind === "voice") ?? []
);

function getVoiceUsers(channelId: number): number[] {
  const users = state.value?.voiceState.get(channelId);
  return users ? [...users] : [];
}

function getVoiceChannelName(): string {
  const ch = state.value?.channels.find((c) => c.id === state.value?.voiceChannelId);
  return ch?.name ?? "";
}

function handleVoiceClick(channelId: number) {
  if (state.value?.voiceChannelId === channelId) {
    leaveVoiceChannel();
  } else {
    joinVoiceChannel(channelId);
  }
}
</script>
