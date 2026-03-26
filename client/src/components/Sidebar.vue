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
        <span class="channel-icon">#</span>
        <span>{{ ch.name }}</span>
      </div>
    </div>

    <div class="channel-group">
      <div class="channel-group-title">Voice</div>
      <div
        v-for="ch in voiceChannels"
        :key="ch.id"
        class="channel-item voice"
      >
        <span class="channel-icon">&#x1f50a;</span>
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
import { activeState, activeServer, selectChannel, resolveUser } from "../store";

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
</script>
