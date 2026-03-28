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
      <div v-for="[uid, vs] in participants" :key="uid" class="voice-participant">
        <div class="voice-participant-avatar" :class="{ speaking: !vs.muted && !vs.deafened && isUserSpeaking(uid) }">
          {{ resolveUser(uid)[0]?.toUpperCase() }}
        </div>
        <span class="voice-participant-name">{{ resolveUser(uid) }}</span>
        <div class="voice-participant-icons">
          <MicOff v-if="vs.muted || vs.force_muted" :size="14" :class="{ forced: vs.force_muted }" />
          <HeadphoneOff v-if="vs.deafened || vs.force_deafened" :size="14" :class="{ forced: vs.force_deafened }" />
        </div>
        <!-- Force mute/deafen pour les admins -->
        <div v-if="uid !== state?.user?.id && (canMuteMembers || canDeafenMembers)" class="voice-participant-actions">
          <button
            v-if="canMuteMembers"
            class="voice-action-btn"
            :class="{ active: vs.force_muted }"
            @click="forceMute(uid, !vs.force_muted)"
            :title="vs.force_muted ? 'Unmute' : 'Force mute'"
          >
            <MicOff :size="12" />
          </button>
          <button
            v-if="canDeafenMembers"
            class="voice-action-btn"
            :class="{ active: vs.force_deafened }"
            @click="forceDeafen(uid, !vs.force_deafened)"
            :title="vs.force_deafened ? 'Undeafen' : 'Force deafen'"
          >
            <HeadphoneOff :size="12" />
          </button>
        </div>
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
import { Phone, Volume2, Loader, AlertCircle, MicOff, HeadphoneOff } from "lucide-vue-next";
import { activeState, resolveUser, joinVoiceChannel, isUserSpeaking, forceMute, forceDeafen } from "../store";
import * as perms from "../permissions";
import type { VoiceUserState } from "../api";

const state = computed(() => activeState());
const channelId = computed(() => state.value?.activeChannelId);

const participants = computed((): [number, VoiceUserState][] => {
  if (!state.value?.activeChannelId) return [];
  const map = state.value.voiceState.get(state.value.activeChannelId);
  return map ? [...map.entries()] : [];
});

const canMuteMembers = computed(() =>
  perms.has(state.value?.permissions ?? 0, perms.MUTE_MEMBERS)
);

const canDeafenMembers = computed(() =>
  perms.has(state.value?.permissions ?? 0, perms.DEAFEN_MEMBERS)
);

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
  position: relative;
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

.voice-participant-icons {
  display: flex;
  gap: 4px;
  color: var(--text-faint);
}

.voice-participant-icons .forced {
  color: var(--danger);
}

.voice-participant-actions {
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.1s;
}

.voice-participant:hover .voice-participant-actions {
  opacity: 1;
}

.voice-action-btn {
  width: 22px;
  height: 22px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: var(--bg-secondary);
  color: var(--text-faint);
  border: none;
  cursor: pointer;
  transition: background 0.1s, color 0.1s;
}

.voice-action-btn:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
  box-shadow: none;
}

.voice-action-btn.active {
  background: var(--danger);
  color: #fff;
}

.voice-action-btn.active:hover {
  background: var(--danger);
  box-shadow: none;
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
  border-radius: 8px;
}

.voice-join-btn:hover { opacity: 0.9; }
</style>
