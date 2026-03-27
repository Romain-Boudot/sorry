<template>
  <div class="voice-view">
    <!-- Status -->
    <div class="voice-view-status" :class="state?.voiceStatus">
      <Loader v-if="state?.voiceStatus === 'connecting'" :size="20" class="spin" />
      <Phone v-else-if="state?.voiceStatus === 'connected'" :size="20" />
      <AlertCircle v-else-if="state?.voiceStatus === 'error'" :size="20" />
      <Volume2 v-else :size="20" />
      <span>{{ statusText }}</span>
    </div>

    <!-- Participants -->
    <div class="voice-view-participants">
      <div
        v-for="uid in participants"
        :key="uid"
        class="voice-participant"
      >
        <div class="voice-participant-avatar">
          {{ resolveUser(uid)[0]?.toUpperCase() }}
        </div>
        <span class="voice-participant-name">{{ resolveUser(uid) }}</span>
      </div>

      <div v-if="!participants.length && state?.voiceStatus !== 'connecting'" class="voice-view-empty">
        Personne dans ce channel
      </div>
    </div>

    <!-- Action -->
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
import { activeState, resolveUser, joinVoiceChannel } from "../store";

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
