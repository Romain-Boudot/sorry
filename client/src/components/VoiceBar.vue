<template>
  <div class="voice-bar" v-if="voiceServer">
    <div class="voice-bar-info">
      <div class="voice-bar-status">
        <Phone :size="14" />
        <span>{{ voiceChannelName }}</span>
      </div>
      <div class="voice-bar-server">{{ voiceServer.name }}</div>
    </div>
    <div class="voice-bar-controls">
      <button
        class="voice-bar-btn"
        :class="{ active: isMuted }"
        @click="toggleMute()"
        :title="isMuted ? 'Unmute' : 'Mute'"
      >
        <MicOff v-if="isMuted" :size="16" />
        <Mic v-else :size="16" />
      </button>
      <button
        class="voice-bar-btn danger"
        @click="leaveVoiceChannel()"
        title="Deconnecter"
      >
        <PhoneOff :size="16" />
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Phone, Mic, MicOff, PhoneOff } from "lucide-vue-next";
import { store, leaveVoiceChannel, toggleMute } from "../store";

const voiceServer = computed(() => {
  for (const server of store.savedServers) {
    const state = store.serverStates.get(server.id);
    if (state?.voiceChannelId) return server;
  }
  return null;
});

const voiceState = computed(() => {
  if (!voiceServer.value) return null;
  return store.serverStates.get(voiceServer.value.id);
});

const voiceChannelName = computed(() => {
  if (!voiceState.value) return "";
  const ch = voiceState.value.channels.find(
    (c) => c.id === voiceState.value?.voiceChannelId
  );
  return ch?.name ?? "";
});

const isMuted = computed(() => voiceState.value?.isMuted ?? false);
</script>
