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

<style scoped>
.voice-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
}

.voice-bar-info {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.voice-bar-status {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--green);
  font-weight: 600;
  font-size: 0.8125rem;
}

.voice-bar-server {
  font-size: 0.6875rem;
  color: var(--text-muted);
  font-weight: 400;
}

.voice-bar-controls {
  display: flex;
  gap: 4px;
}

.voice-bar-btn {
  width: 32px;
  height: 32px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  border: none;
  transition: background 0.1s, color 0.1s;
}

.voice-bar-btn:hover { background: var(--bg-modifier-hover); color: var(--text-normal); box-shadow: none; }
.voice-bar-btn.active { background: var(--danger); color: #fff; }
.voice-bar-btn.active:hover { background: var(--danger); box-shadow: none; }
.voice-bar-btn.danger { color: var(--text-muted); }
.voice-bar-btn.danger:hover { background: rgba(208, 80, 80, 0.15); color: var(--danger); box-shadow: none; }
</style>
