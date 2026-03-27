<template>
  <div class="voice-view">
    <div class="voice-view-status" :class="state?.voiceStatus">
      <Loader v-if="state?.voiceStatus === 'connecting'" :size="20" class="spin" />
      <Phone v-else-if="state?.voiceStatus === 'connected'" :size="20" />
      <AlertCircle v-else-if="state?.voiceStatus === 'error'" :size="20" />
      <Volume2 v-else :size="20" />
      <span>{{ statusText }}</span>
    </div>

    <div class="voice-view-participants">
      <div v-for="uid in participants" :key="uid" class="voice-participant">
        <div class="voice-participant-avatar" :class="{ speaking: isUserSpeaking(uid) }">
          {{ resolveUser(uid)[0]?.toUpperCase() }}
        </div>
        <span class="voice-participant-name">{{ resolveUser(uid) }}</span>
      </div>
      <div v-if="!participants.length && state?.voiceStatus !== 'connecting'" class="voice-view-empty">
        Personne dans ce channel
      </div>
    </div>

    <div class="voice-view-action">
      <button
        v-if="state?.voiceStatus === 'idle' || state?.voiceStatus === 'error'"
        class="voice-join-btn"
        @click="joinVoiceChannel(channelId!)"
      >
        <Phone :size="18" />
        Rejoindre
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Phone, Volume2, Loader, AlertCircle } from "lucide-vue-next";
import { activeState, resolveUser, joinVoiceChannel, isUserSpeaking } from "../store";

const state = computed(() => activeState());
const channelId = computed(() => state.value?.activeChannelId);

const participants = computed(() => {
  if (!state.value?.activeChannelId) return [];
  const users = state.value.voiceState.get(state.value.activeChannelId);
  return users ? [...users] : [];
});

const statusText = computed(() => {
  switch (state.value?.voiceStatus) {
    case "connecting": return "Connexion en cours...";
    case "connected": return "Connecte";
    case "error": return "Erreur de connexion";
    default: return "Vocal";
  }
});
</script>

<style scoped>
.voice-view {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: var(--bg-primary);
  gap: 24px;
  padding: 40px;
}

.voice-view-status {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  font-size: 1rem;
  color: var(--text-muted);
}

.voice-view-status.connecting { color: var(--text-normal); }
.voice-view-status.connected { color: var(--green); }
.voice-view-status.error { color: var(--danger); }

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.voice-view-participants {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
  justify-content: center;
  max-width: 480px;
}

.voice-participant {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
}

.voice-participant-avatar {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: var(--bg-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 1rem;
  color: var(--text-normal);
  border: 2px solid transparent;
  transition: border-color 0.15s;
}

.voice-participant-avatar.speaking {
  border-color: var(--green);
}

.voice-participant-name {
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--text-muted);
}

.voice-view-empty {
  color: var(--text-faint);
  font-size: 0.875rem;
}

.voice-view-action {
  display: flex;
}

.voice-join-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  width: auto;
  padding: 10px 24px;
  background: var(--green);
  color: #fff;
  font-weight: 600;
  font-size: 0.875rem;
  border-radius: 4px;
}

.voice-join-btn:hover { opacity: 0.9; }
</style>
