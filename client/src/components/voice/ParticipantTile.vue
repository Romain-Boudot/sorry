<template>
  <div
    class="ptile"
    :class="{
      speaking: isSpeaking,
      'is-spotlight': isSpotlight,
      'is-self': isSelf,
      'is-video': hasVideo,
      'is-pending': tile.kind === 'screen-pending',
    }"
    :data-tile-key="tile.key"
    @dblclick="tile.kind !== 'screen-pending' && $emit('toggleSpotlight')"
    @contextmenu.prevent="$emit('contextMenu', $event)"
  >
    <!-- Video layer -->
    <video v-if="hasVideo" ref="videoEl" autoplay playsinline muted></video>

    <!-- Avatar layer (always visible behind video for fallback while frames load) -->
    <div v-else-if="tile.kind !== 'screen-pending'" class="ptile-avatar-layer">
      <div class="ptile-avatar">
        <img v-if="tile.avatarUrl" :src="tile.avatarUrl" />
        <span v-else>{{ tile.name[0]?.toUpperCase() }}</span>
      </div>
    </div>

    <!-- Screen-share-pending layer: implicit "click to watch" -->
    <button
      v-else
      class="ptile-pending-cta"
      @click="$emit('toggleWatch')"
      :title="`Regarder l'ecran de ${tile.name}`"
    >
      <div class="ptile-avatar dimmed">
        <img v-if="tile.avatarUrl" :src="tile.avatarUrl" />
        <span v-else>{{ tile.name[0]?.toUpperCase() }}</span>
      </div>
      <div class="ptile-pending-overlay">
        <Monitor :size="22" />
        <span>Regarder l'ecran</span>
      </div>
    </button>

    <!-- Source badge (top-left, only on screen tiles) -->
    <div v-if="tile.kind === 'screen'" class="ptile-badge">
      <Monitor :size="11" />
      <span>Ecran</span>
    </div>

    <!-- Quality dot (top-left, subtle) -->
    <span
      v-if="quality && quality !== 'excellent' && quality !== 'unknown' && tile.kind !== 'screen-pending'"
      class="ptile-quality"
      :class="quality"
      :title="quality === 'good' ? 'Connexion correcte' : 'Connexion faible'"
    ></span>

    <!-- Hover actions (top-right) -->
    <div v-if="tile.kind !== 'screen-pending'" class="ptile-actions" @click.stop @dblclick.stop>
      <button
        v-if="!isSelf && hasAudio"
        class="ptile-btn"
        :class="{ active: isRemoteMuted }"
        @click="$emit('muteLocal')"
        :title="isRemoteMuted ? 'Reactiver le son' : 'Mute pour moi'"
      >
        <VolumeX v-if="isRemoteMuted" :size="13" />
        <Volume2 v-else :size="13" />
      </button>
      <button
        v-if="hasVideo"
        class="ptile-btn"
        @click="$emit('stats')"
        title="Stats"
      >
        <Info :size="13" />
      </button>
      <button
        v-if="hasVideo"
        class="ptile-btn"
        @click="$emit('fullscreen')"
        title="Plein ecran"
      >
        <Maximize :size="13" />
      </button>
      <button
        v-if="tile.kind === 'screen' && !isSelf"
        class="ptile-btn danger"
        @click="$emit('toggleWatch')"
        title="Arreter de regarder"
      >
        <EyeOff :size="13" />
      </button>
    </div>

    <!-- Bottom name + voice state icons -->
    <div v-if="tile.kind !== 'screen-pending'" class="ptile-footer">
      <span class="ptile-name">{{ tile.name }}<span v-if="tile.kind === 'screen'" class="ptile-name-suffix">'s screen</span></span>
      <div v-if="tile.voiceState" class="ptile-status">
        <MicOff v-if="tile.voiceState.muted || tile.voiceState.force_muted" :size="11" :class="{ forced: tile.voiceState.force_muted }" />
        <HeadphoneOff v-if="tile.voiceState.deafened || tile.voiceState.force_deafened" :size="11" :class="{ forced: tile.voiceState.force_deafened }" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount, computed } from "vue";
import { Monitor, Maximize, EyeOff, Info, Volume2, VolumeX, MicOff, HeadphoneOff } from "lucide-vue-next";
import type { VoiceUserState } from "../../api";
import type { QualityLevel } from "../../voice";

export interface Tile {
  key: string;
  uid: number;
  identity: string;
  name: string;
  avatarUrl: string | null;
  /** 'avatar' = no stream | 'camera' = live cam | 'screen' = watched screen share | 'screen-pending' = available, opt-in */
  kind: "avatar" | "camera" | "screen" | "screen-pending";
  videoTrack?: any;
  voiceState?: VoiceUserState;
}

const props = defineProps<{
  tile: Tile;
  isSpotlight: boolean;
  isSpeaking: boolean;
  isSelf: boolean;
  isWatchingScreen: boolean;
  isRemoteMuted: boolean;
  hasAudio: boolean;
  quality: QualityLevel | null;
}>();

defineEmits<{
  toggleWatch: [];
  toggleSpotlight: [];
  stats: [];
  fullscreen: [];
  muteLocal: [];
  contextMenu: [event: MouseEvent];
}>();

const hasVideo = computed(() => props.tile.kind === "camera" || props.tile.kind === "screen");

const videoEl = ref<HTMLVideoElement>();

function attachCurrent() {
  const el = videoEl.value;
  const t = props.tile.videoTrack;
  if (!el || !t) return;
  t.detach().forEach((stale: HTMLElement) => { if (stale !== el) stale.remove(); });
  t.attach(el);
}

onMounted(attachCurrent);
watch(() => props.tile.videoTrack, attachCurrent);

onBeforeUnmount(() => {
  const el = videoEl.value;
  if (el) el.srcObject = null;
});
</script>

<style scoped>
.ptile {
  position: relative;
  width: 100%;
  height: 100%;
  border-radius: 10px;
  overflow: hidden;
  background: var(--bg-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: default;
  transition: outline-color 0.15s, transform 0.15s;
  outline: 2px solid transparent;
  outline-offset: -2px;
}

.ptile.speaking {
  outline-color: var(--green);
}

.ptile.is-video {
  background: #000;
}

.ptile video {
  width: 100%;
  height: 100%;
  object-fit: contain;
  background: #000;
  display: block;
}

/* ── Avatar layer ── */
.ptile-avatar-layer {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
}

.ptile-avatar {
  width: 30%;
  max-width: 96px;
  min-width: 48px;
  aspect-ratio: 1;
  border-radius: 50%;
  background: var(--accent);
  color: var(--text-bright);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: clamp(1rem, 4vw, 1.75rem);
  overflow: hidden;
  flex-shrink: 0;
}
.ptile-avatar img { width: 100%; height: 100%; object-fit: cover; }
.ptile-avatar.dimmed { opacity: 0.4; }

/* ── Screen-share pending CTA ── */
.ptile-pending-cta {
  width: 100%;
  height: 100%;
  background: var(--bg-secondary);
  border: 1px dashed var(--border);
  border-radius: 10px;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  cursor: pointer;
  color: var(--text-muted);
  transition: background 0.15s, color 0.15s, border-color 0.15s;
}
.ptile-pending-cta:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
  border-color: var(--accent);
  border-style: solid;
}

.ptile.is-pending {
  background: transparent;
}

.ptile-pending-overlay {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.8125rem;
  font-weight: 600;
}

/* ── Source badge ── */
.ptile-badge {
  position: absolute;
  top: 8px;
  left: 8px;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: var(--overlay);
  backdrop-filter: blur(4px);
  padding: 3px 8px;
  border-radius: 6px;
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--text-bright);
  pointer-events: none;
}

/* ── Quality dot ── */
.ptile-quality {
  position: absolute;
  top: 10px;
  left: 10px;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  z-index: 2;
}
.ptile.is-video .ptile-quality { left: auto; right: 10px; top: 10px; }
.ptile-quality.good { background: #f0a020; }
.ptile-quality.poor { background: var(--danger); }

/* ── Hover actions ── */
.ptile-actions {
  position: absolute;
  top: 8px;
  right: 8px;
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.15s;
  z-index: 3;
}
.ptile:hover .ptile-actions { opacity: 1; }

.ptile-btn {
  width: 26px;
  height: 26px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--overlay);
  backdrop-filter: blur(4px);
  border: none;
  border-radius: 6px;
  color: var(--text-bright);
  cursor: pointer;
}
.ptile-btn:hover { background: rgba(0, 0, 0, 0.7); }
.ptile-btn.danger:hover { background: var(--danger); }
.ptile-btn.active { background: var(--danger); }

/* ── Bottom footer (name + status icons) ── */
.ptile-footer {
  position: absolute;
  bottom: 8px;
  left: 8px;
  right: 8px;
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--overlay);
  backdrop-filter: blur(4px);
  padding: 3px 8px;
  border-radius: 6px;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-bright);
  pointer-events: none;
  max-width: calc(100% - 16px);
}

.ptile-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ptile-name-suffix {
  opacity: 0.6;
  font-weight: 500;
  margin-left: 4px;
}

.ptile-status {
  display: flex;
  gap: 4px;
  color: rgba(255, 255, 255, 0.7);
  flex-shrink: 0;
}
.ptile-status .forced { color: var(--danger); }
</style>
