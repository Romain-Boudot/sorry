<template>
  <div class="voice-bar" v-if="voiceEntry">
    <div class="voice-bar-info">
      <div class="voice-bar-status" :class="voiceEntry.state.voiceStatus">
        <Loader2 v-if="voiceEntry.state.voiceStatus === 'connecting'" :size="14" class="spin" />
        <AlertCircle v-else-if="voiceEntry.state.voiceStatus === 'error'" :size="14" />
        <Phone v-else :size="14" />
        <span>{{ statusLabel }}</span>
      </div>
      <div class="voice-bar-server">{{ voiceEntry.server.name }} - {{ channelName }}</div>
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
import { Phone, PhoneOff, Loader2, AlertCircle } from "lucide-vue-next";
import { store, leaveVoiceChannel } from "../store";

const voiceEntry = computed(() => {
  for (const server of store.savedServers) {
    const state = store.serverStates.get(server.id);
    console.log(state?.voiceStatus, state?.voiceConnectingChannelId)
    if (state && state.voiceStatus !== "idle") return { server, state };
  }
  return null;
});

const channelName = computed(() => {
  if (!voiceEntry.value) return "";
  const { state } = voiceEntry.value;
  const targetId = state.voiceChannelId ?? state.voiceConnectingChannelId;
  console.log(targetId)
  const ch = state.channels.find((c) => c.id === targetId);
  return ch?.name
})

const statusLabel = computed(() => {
  if (!voiceEntry.value) return "";
  const { state } = voiceEntry.value;
  if (state.voiceStatus === "connecting") return "Connexion...";
  if (state.voiceStatus === "error") return "Erreur de connexion";
  const ch = state.channels.find(
    (c) => c.id === state.voiceChannelId
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

.voice-bar-status.connecting {
  color: var(--text-muted);
}

.voice-bar-status.error {
  color: var(--danger);
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

@keyframes spin {
  to { transform: rotate(360deg); }
}

.spin {
  animation: spin 1s linear infinite;
}
</style>
