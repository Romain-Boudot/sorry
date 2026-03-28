<template>
  <div class="voice-bar" v-if="voiceServer">
    <div class="voice-bar-info">
      <div class="voice-bar-status">
        <Phone :size="14" />
        <span>{{ voiceChannelName }}</span>
      </div>
      <div class="voice-bar-server">{{ voiceServer.name }}</div>
    </div>
    <button
      class="voice-bar-btn danger"
      @click="leaveVoiceChannel()"
      title="Deconnecter"
    >
      <PhoneOff :size="16" />
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Phone, PhoneOff } from "lucide-vue-next";
import { store, leaveVoiceChannel } from "../store";

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
  min-width: 0;
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

.voice-bar-btn {
  width: 32px;
  height: 32px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  border: none;
  transition: background 0.1s, color 0.1s;
}

.voice-bar-btn.danger:hover {
  background: rgba(208, 80, 80, 0.15);
  color: var(--danger);
  box-shadow: none;
}
</style>
