<template>
  <div class="voice-view" :class="{ 'has-video': videoTracks.length > 0 }">
    <!-- Video streams -->
    <div v-if="videoTracks.length" class="voice-video-area">
      <div
        v-for="vt in videoTracks"
        :key="vt.identity + '-' + vt.source"
        class="voice-video-tile"
        :class="{ spotlight: videoTracks.length === 1 }"
        :data-track-key="vt.identity + '-' + vt.source"
      >
        <video autoplay playsinline muted />
        <div class="voice-video-label">
          <span>{{ vt.name }}</span>
          <span class="voice-video-source">{{ vt.source === 'screen_share' ? 'Ecran' : 'Camera' }}</span>
        </div>
      </div>
    </div>

    <div v-if="!videoTracks.length" class="voice-view-status" :class="state?.voiceStatus">
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
import { computed, ref, watch, nextTick } from "vue";
import { Phone, Volume2, Loader, AlertCircle, MicOff, HeadphoneOff } from "lucide-vue-next";
import { Track } from "livekit-client";
import { activeState, resolveUser, joinVoiceChannel, isUserSpeaking, forceMute, forceDeafen } from "../store";
import { getCurrentRoom } from "../voice";
import * as perms from "../permissions";
import type { VoiceUserState } from "../api";

const state = computed(() => activeState());
const channelId = computed(() => state.value?.activeChannelId);

// ── Video tracks ──
interface VideoTrackInfo {
  identity: string;
  name: string;
  source: "camera" | "screen_share";
  track: any; // Track from livekit-client
}

const videoTracks = computed((): VideoTrackInfo[] => {
  // Read videoTrackVersion to trigger reactivity
  const _version = state.value?.videoTrackVersion;
  const room = getCurrentRoom();
  if (!room) return [];

  const tracks: VideoTrackInfo[] = [];

  // Local participant tracks
  const local = room.localParticipant;
  for (const pub of local.videoTrackPublications.values()) {
    if (pub.track && !pub.isMuted) {
      tracks.push({
        identity: local.identity,
        name: local.name || local.identity,
        source: pub.source === Track.Source.ScreenShare ? "screen_share" : "camera",
        track: pub.track,
      });
    }
  }

  // Remote participant tracks
  for (const participant of room.remoteParticipants.values()) {
    for (const pub of participant.videoTrackPublications.values()) {
      if (pub.track && pub.isSubscribed && !pub.isMuted) {
        tracks.push({
          identity: participant.identity,
          name: participant.name || participant.identity,
          source: pub.source === Track.Source.ScreenShare ? "screen_share" : "camera",
          track: pub.track,
        });
      }
    }
  }

  return tracks;
});

const prevTrackKeys = new Set<string>();

watch(videoTracks, (tracks) => {
  const currentKeys = new Set(tracks.map((vt) => vt.identity + "-" + vt.source));

  // Detach removed tracks
  for (const key of prevTrackKeys) {
    if (!currentKeys.has(key)) {
      const tile = document.querySelector(`[data-track-key="${key}"]`);
      const video = tile?.querySelector("video");
      if (video) video.srcObject = null;
    }
  }

  prevTrackKeys.clear();
  for (const key of currentKeys) prevTrackKeys.add(key);

  nextTick(() => {
    for (const vt of tracks) {
      const key = vt.identity + "-" + vt.source;
      const tile = document.querySelector(`[data-track-key="${key}"]`);
      if (!tile) continue;
      const video = tile.querySelector("video");
      if (!video) continue;
      vt.track.detach().forEach((el: HTMLElement) => { if (el !== video) el.remove(); });
      vt.track.attach(video);
    }
  });
}, { immediate: true });

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
  overflow: hidden;
}

.voice-view.has-video {
  justify-content: flex-start;
  padding: 16px;
  gap: 16px;
}

/* ── Video area ── */
.voice-video-area {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: center;
  flex: 1;
  min-height: 0;
  width: 100%;
}

.voice-video-tile {
  position: relative;
  border-radius: 12px;
  overflow: hidden;
  background: var(--bg-tertiary);
  flex: 1 1 300px;
  max-width: 100%;
  min-height: 200px;
  max-height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.voice-video-tile.spotlight {
  flex: 1 1 100%;
}

.voice-video-tile video {
  width: 100%;
  height: 100%;
  object-fit: contain;
  background: #000;
  border-radius: 12px;
}

.voice-video-label {
  position: absolute;
  bottom: 8px;
  left: 8px;
  display: flex;
  align-items: center;
  gap: 6px;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  padding: 4px 10px;
  border-radius: 6px;
  font-size: 0.75rem;
  font-weight: 600;
  color: #fff;
}

.voice-video-source {
  font-weight: 400;
  opacity: 0.7;
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
