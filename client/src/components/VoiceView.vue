<template>
  <div class="voice-view">
    <!-- Idle / error -->
    <div v-if="state?.voiceStatus === 'idle' || state?.voiceStatus === 'error'" class="voice-idle">
      <div class="voice-idle-status" :class="state?.voiceStatus">
        <AlertCircle v-if="state?.voiceStatus === 'error'" :size="20" />
        <Volume2 v-else :size="20" />
        <span>{{ statusText }}</span>
      </div>
      <button class="voice-join-btn" @click="joinVoiceChannel(channelId!)">
        <Phone :size="18" />
        Rejoindre
      </button>
    </div>

    <!-- Connecting -->
    <div v-else-if="state?.voiceStatus === 'connecting'" class="voice-idle">
      <div class="voice-idle-status connecting">
        <Loader :size="20" class="spin" />
        <span>Connexion en cours...</span>
      </div>
    </div>

    <!-- Connected -->
    <div v-else class="voice-stage" :class="{ 'has-streams': hasStreams, 'has-spotlight': spotlightTile }">
      <!-- Stream area: only rendered when at least one stream exists -->
      <div v-if="hasStreams" class="voice-streams">
        <!-- Spotlight tile (if any) -->
        <div v-if="spotlightTile" class="voice-spotlight">
          <ParticipantTile
            :tile="spotlightTile"
            :is-spotlight="true"
            :is-speaking="isUserSpeaking(spotlightTile.uid)"
            :is-self="spotlightTile.uid === state?.user?.id"
            :is-watching-screen="isWatchingScreen(spotlightTile.uid)"
            :is-remote-muted="isRemoteMuted(spotlightTile.uid)"
            :has-audio="hasAudio(spotlightTile.uid)"
            :quality="qualityFor(spotlightTile.uid)"
            @toggle-watch="onToggleWatch(spotlightTile)"
            @toggle-spotlight="spotlightKey = null"
            @stats="onShowStats(spotlightTile)"
            @fullscreen="onFullscreen(spotlightTile)"
            @mute-local="toggleLocalMute(spotlightTile.uid)"
            @context-menu="(e: MouseEvent) => openContextMenu(spotlightTile!, e)"
          />
        </div>

        <!-- Stream grid / thumbnails -->
        <div class="voice-grid" :class="{ thumbnails: spotlightTile, empty: !nonSpotlightStreams.length }">
          <ParticipantTile
            v-for="tile in nonSpotlightStreams"
            :key="tile.key"
            :tile="tile"
            :is-spotlight="false"
            :is-speaking="isUserSpeaking(tile.uid)"
            :is-self="tile.uid === state?.user?.id"
            :is-watching-screen="isWatchingScreen(tile.uid)"
            :is-remote-muted="isRemoteMuted(tile.uid)"
            :has-audio="hasAudio(tile.uid)"
            :quality="qualityFor(tile.uid)"
            @toggle-watch="onToggleWatch(tile)"
            @toggle-spotlight="setSpotlight(tile)"
            @stats="onShowStats(tile)"
            @fullscreen="onFullscreen(tile)"
            @mute-local="toggleLocalMute(tile.uid)"
            @context-menu="(e) => openContextMenu(tile, e)"
          />
        </div>
      </div>

      <!-- User cards: compact strip at the bottom when streams exist, else centered grid -->
      <div class="voice-users" :class="{ strip: hasStreams }">
        <VoiceUserCard
          v-for="p in participants"
          :key="p.uid"
          :name="p.name"
          :avatar-url="p.avatarUrl"
          :voice-state="p.voiceState"
          :is-speaking="isUserSpeaking(p.uid)"
          :is-self="p.uid === state?.user?.id"
          :quality="qualityFor(p.uid)"
          :compact="hasStreams"
          @click="(e) => openUserCard(p.uid, e)"
          @context-menu="(e) => openUserContextMenu(p, e)"
        />
      </div>
    </div>

    <UserCard
      v-if="cardUser"
      :user="cardUser"
      :x="cardX"
      :y="cardY"
      @close="cardUser = null"
    />

    <!-- Stats popup -->
    <div v-if="statsTrack" class="stats-popup" @click.self="statsTrack = null">
      <div class="stats-card">
        <div class="stats-header">
          <span>Stats — {{ statsTrack.name }}</span>
          <button class="stats-close" @click="statsTrack = null"><X :size="14" /></button>
        </div>

        <div v-if="liveStats" class="stats-body">
          <!-- Health banner (publisher only) -->
          <div v-if="isLocalStats && liveStats.limitationReason" class="stats-health" :class="healthClass">
            <component :is="healthIcon" :size="14" />
            <div class="stats-health-text">
              <div class="stats-health-title">{{ healthTitle }}</div>
              <div class="stats-health-sub">{{ healthSub }}</div>
            </div>
          </div>

          <!-- Summary -->
          <div class="stats-section">
            <div class="stats-row"><span>Source</span><span>{{ statsTrack.kind === 'screen' ? 'Ecran' : 'Camera' }}</span></div>
            <div class="stats-row"><span>Resolution</span><span>{{ liveStats.width }}x{{ liveStats.height }}</span></div>
            <div class="stats-row"><span>FPS</span><span>{{ liveStats.fps }}</span></div>
            <div class="stats-row">
              <span>{{ isLocalStats ? 'Bitrate total (envoi)' : 'Bitrate (reception)' }}</span>
              <span>{{ formatKbps(liveStats.bitrateKbps) }}</span>
            </div>
            <div v-if="isLocalStats && liveStats.targetBitrateKbps" class="stats-row">
              <span title="Bitrate cible que le BWE tente d'atteindre">Bitrate cible</span>
              <span>{{ formatKbps(liveStats.targetBitrateKbps) }}</span>
            </div>
            <div class="stats-row"><span>Qualite reseau</span><span>{{ qualityLabel(qualityFor(statsTrack.uid)) }}</span></div>
          </div>

          <!-- Layers (simulcast breakdown) -->
          <div v-if="liveStats.layers.length > 1" class="stats-section">
            <div class="stats-section-title">Layers simulcast</div>
            <table class="stats-layers">
              <thead>
                <tr>
                  <th>Layer</th>
                  <th>Resolution</th>
                  <th>FPS</th>
                  <th>Bitrate</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="(l, i) in liveStats.layers" :key="i" :class="{ inactive: !l.active }">
                  <td>{{ layerLabel(l.rid, i, liveStats.layers.length) }}</td>
                  <td>{{ l.width }}x{{ l.height }}</td>
                  <td>{{ l.fps }}</td>
                  <td>{{ formatKbps(l.bitrateKbps) }}</td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Capture (publisher only) — shows the OS/browser source rate -->
          <div v-if="isLocalStats && liveStats.captureFps !== undefined" class="stats-section">
            <div class="stats-section-title">
              Capture (source)
              <span v-if="captureCapped" class="stats-cap-badge" title="La source delivre moins que demande">limitee</span>
            </div>
            <div class="stats-row">
              <span title="Resolution captee depuis l'ecran ou la webcam">Resolution source</span>
              <span>{{ liveStats.captureWidth }}x{{ liveStats.captureHeight }}</span>
            </div>
            <div class="stats-row">
              <span title="Frames recues du source par seconde — cap par l'OS / le navigateur / la cam">FPS source</span>
              <span :class="{ 'stats-warn': captureCapped }">{{ liveStats.captureFps }}</span>
            </div>
            <div v-if="captureCapped" class="stats-hint">
              Le navigateur cap la capture sous le FPS demande. Cause probable : politique browser pour `getDisplayMedia` (Chrome Windows = 30 FPS par defaut), ou refresh-rate de l'ecran source.
            </div>
          </div>

          <!-- Encoder health (publisher only) -->
          <div v-if="isLocalStats" class="stats-section">
            <div class="stats-section-title">Encodeur</div>
            <div v-if="liveStats.encoderImplementation" class="stats-row">
              <span>Implementation</span>
              <span>{{ formatEncoder(liveStats.encoderImplementation) }}</span>
            </div>
            <div v-if="liveStats.encodeMsPerFrame !== undefined" class="stats-row">
              <span title="Temps moyen pour encoder une frame">Temps/frame</span>
              <span>{{ liveStats.encodeMsPerFrame }} ms</span>
            </div>
          </div>

          <!-- Loss counters -->
          <div
            v-if="(liveStats.nackCount ?? 0) + (liveStats.pliCount ?? 0) + (liveStats.firCount ?? 0) > 0"
            class="stats-section"
          >
            <div class="stats-section-title">Signaux de perte</div>
            <div class="stats-row"><span title="Retransmission demandees">NACK</span><span>{{ liveStats.nackCount ?? 0 }}</span></div>
            <div class="stats-row"><span title="Picture Loss Indication : frame perdue">PLI</span><span>{{ liveStats.pliCount ?? 0 }}</span></div>
            <div class="stats-row"><span title="Full Intra Request : keyframe demandee">FIR</span><span>{{ liveStats.firCount ?? 0 }}</span></div>
          </div>
        </div>
        <div v-else class="stats-body stats-loading">Mesure en cours...</div>
      </div>
    </div>

    <!-- Right-click context menu -->
    <ContextMenu
      v-if="ctxMenu"
      :x="ctxMenu.x"
      :y="ctxMenu.y"
      :items="ctxMenu.items"
      @close="ctxMenu = null"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, onUnmounted } from "vue";
import { Phone, Volume2, Loader, AlertCircle, X, Star, Maximize, MonitorOff, Monitor, MicOff, HeadphoneOff, Info, VolumeX, EyeOff, CheckCircle2, Cpu, Gauge, AlertTriangle } from "lucide-vue-next";
import { Track } from "livekit-client";
import { activeState, resolveUser, joinVoiceChannel, isUserSpeaking, forceMute, forceDeafen, resolveAvatarUrl } from "../store";
import { getCurrentRoom, mediaState, watchScreen, unwatchScreen, toggleRemoteMute, getTrackStats, setViewQuality, getViewQuality, type TrackStats, type QualityLevel, type ViewQuality } from "../voice";
import { streamSettings } from "../streamSettings";
import * as perms from "../permissions";
import type { VoiceUserState } from "../api";
import ParticipantTile, { type Tile } from "./voice/ParticipantTile.vue";
import VoiceUserCard from "./voice/VoiceUserCard.vue";
import UserCard from "./UserCard.vue";
import ContextMenu, { type MenuItem } from "./ui/ContextMenu.vue";
import type { User } from "../api";

const state = computed(() => activeState());
const channelId = computed(() => state.value?.activeChannelId);

// LiveKit identity ↔ uid bridge (server uses `user-${id}`).
const identityForUid = (uid: number) => `user-${uid}`;

// ── Live video tracks pulled from the LiveKit room ──
interface LiveTrack { identity: string; source: "camera" | "screen_share"; track: any }

const liveTracks = computed((): LiveTrack[] => {
  void state.value?.videoTrackVersion;
  void mediaState.version;
  const room = getCurrentRoom();
  if (!room) return [];
  const list: LiveTrack[] = [];

  // Local: always render local previews
  for (const pub of room.localParticipant.videoTrackPublications.values()) {
    if (pub.track && !pub.isMuted) {
      list.push({
        identity: room.localParticipant.identity,
        source: pub.source === Track.Source.ScreenShare ? "screen_share" : "camera",
        track: pub.track,
      });
    }
  }

  // Remote: only subscribed + unmuted
  for (const p of room.remoteParticipants.values()) {
    for (const pub of p.videoTrackPublications.values()) {
      if (!pub.track || pub.isMuted || !pub.isSubscribed) continue;
      list.push({
        identity: p.identity,
        source: pub.source === Track.Source.ScreenShare ? "screen_share" : "camera",
        track: pub.track,
      });
    }
  }
  return list;
});

function findLive(identity: string, source: "camera" | "screen_share") {
  return liveTracks.value.find((t) => t.identity === identity && t.source === source);
}

// ── Stream tiles (camera / screen / screen-pending) ──
// User avatars live in `userCards` below — the two lists are rendered separately.
interface VoiceUserEntry { uid: number; identity: string; name: string; avatarUrl: string | null; voiceState: VoiceUserState }

const participants = computed((): VoiceUserEntry[] => {
  if (!state.value?.activeChannelId) return [];
  const map = state.value.voiceState.get(state.value.activeChannelId);
  if (!map) return [];
  return [...map.entries()].map(([uid, vs]) => ({
    uid,
    identity: identityForUid(uid),
    name: resolveUser(uid),
    avatarUrl: resolveAvatarUrl(uid),
    voiceState: vs,
  }));
});

const streamTiles = computed((): Tile[] => {
  const list: Tile[] = [];
  for (const p of participants.value) {
    // Camera (live)
    const cam = findLive(p.identity, "camera");
    if (cam) {
      list.push({
        key: `${p.identity}-camera`,
        uid: p.uid, identity: p.identity, name: p.name, avatarUrl: p.avatarUrl,
        kind: "camera", videoTrack: cam.track, voiceState: p.voiceState,
      });
    }
    // Screen share (live or pending)
    if (p.voiceState.screen_sharing) {
      const screen = findLive(p.identity, "screen_share");
      list.push(screen ? {
        key: `${p.identity}-screen`,
        uid: p.uid, identity: p.identity, name: p.name, avatarUrl: p.avatarUrl,
        kind: "screen", videoTrack: screen.track, voiceState: p.voiceState,
      } : {
        key: `${p.identity}-screen-pending`,
        uid: p.uid, identity: p.identity, name: p.name, avatarUrl: p.avatarUrl,
        kind: "screen-pending", voiceState: p.voiceState,
      });
    }
  }
  return list;
});

const hasStreams = computed(() => streamTiles.value.length > 0);

// ── Spotlight (streams only) ──
const spotlightKey = ref<string | null>(null);

const spotlightTile = computed(() =>
  spotlightKey.value ? streamTiles.value.find((t) => t.key === spotlightKey.value) ?? null : null
);

const nonSpotlightStreams = computed(() =>
  spotlightKey.value ? streamTiles.value.filter((t) => t.key !== spotlightKey.value) : streamTiles.value
);

function setSpotlight(tile: Tile) {
  spotlightKey.value = spotlightKey.value === tile.key ? null : tile.key;
}

watch(streamTiles, (list) => {
  if (spotlightKey.value && !list.some((t) => t.key === spotlightKey.value)) {
    spotlightKey.value = null;
  }
});

// ── Per-participant action helpers (delegated to voice.ts state) ──
const isWatchingScreen = (uid: number) => (void mediaState.version, mediaState.watchedScreens.has(identityForUid(uid)));
const isRemoteMuted = (uid: number) => (void mediaState.version, mediaState.mutedRemotes.has(identityForUid(uid)));
const hasAudio = (uid: number) => (void mediaState.version, mediaState.audioElements.has(identityForUid(uid)));
const qualityFor = (uid: number): QualityLevel | null => (void mediaState.version, mediaState.quality.get(identityForUid(uid)) ?? null);
const toggleLocalMute = (uid: number) => toggleRemoteMute(identityForUid(uid));

function qualityLabel(q: QualityLevel | null): string {
  switch (q) {
    case "excellent": return "Excellente";
    case "good": return "Correcte";
    case "poor": return "Faible";
    default: return "Inconnue";
  }
}

// ── Tile callbacks ──
function onToggleWatch(t: Tile) {
  const id = identityForUid(t.uid);
  if (mediaState.watchedScreens.has(id)) unwatchScreen(id);
  else watchScreen(id);
}

function onFullscreen(t: Tile) {
  const tile = document.querySelector(`[data-tile-key="${t.key}"] video`) as HTMLVideoElement | null;
  if (tile?.requestFullscreen) tile.requestFullscreen().catch(() => {});
}

// ── Stats popup ──
const statsTrack = ref<Tile | null>(null);
const liveStats = ref<TrackStats | null>(null);
let statsTimer: ReturnType<typeof setInterval> | null = null;

function onShowStats(t: Tile) {
  statsTrack.value = t;
  liveStats.value = null;
  refreshStats();
  statsTimer = setInterval(refreshStats, 1500);
}

async function refreshStats() {
  const t = statsTrack.value;
  if (!t || (t.kind !== "camera" && t.kind !== "screen")) return;
  liveStats.value = await getTrackStats(t.identity, t.kind === "screen" ? "screen_share" : "camera");
}

watch(statsTrack, (v) => {
  if (!v && statsTimer) { clearInterval(statsTimer); statsTimer = null; }
});

// ── Stats display helpers ──
const isLocalStats = computed(() => statsTrack.value?.uid === state.value?.user?.id);

const healthClass = computed(() => {
  switch (liveStats.value?.limitationReason) {
    case "cpu": return "warn";
    case "bandwidth": return "warn";
    case "other": return "neutral";
    case "none":
    default: return "ok";
  }
});

const healthIcon = computed(() => {
  switch (liveStats.value?.limitationReason) {
    case "cpu": return Cpu;
    case "bandwidth": return Gauge;
    case "other": return AlertTriangle;
    case "none":
    default: return CheckCircle2;
  }
});

const healthTitle = computed(() => {
  switch (liveStats.value?.limitationReason) {
    case "cpu": return "Encodeur en galere (CPU)";
    case "bandwidth": return "Bande passante limitee";
    case "other": return "Limitation diverse";
    case "none":
    default: return "Stream en bonne sante";
  }
});

const healthSub = computed(() => {
  switch (liveStats.value?.limitationReason) {
    case "cpu":
      return "Ton processeur n'arrive pas a suivre. Baisse resolution ou FPS.";
    case "bandwidth":
      return "Congestion entre toi et le serveur — pas ton reseau local. Essaie un preset plus leger.";
    case "other":
      return "Encodeur contraint pour une autre raison.";
    case "none":
    default:
      return "Si les FPS sont bas c'est surement juste du contenu statique (optimisation encodeur normale).";
  }
});

function formatKbps(kbps: number): string {
  if (kbps >= 1000) return `${(kbps / 1000).toFixed(1)} Mbps`;
  return `${kbps} kbps`;
}

function formatEncoder(impl: string): string {
  // Heuristic: hardware encoders usually mention "External" or vendor names.
  const hw = /External|Hardware|Intel|NVENC|VideoToolbox|MediaCodec|AMF|QuickSync/i.test(impl);
  return `${impl} (${hw ? "materiel" : "logiciel"})`;
}

/** True if the OS/browser is delivering noticeably fewer FPS than the user requested. */
const captureCapped = computed(() => {
  const t = statsTrack.value;
  if (!t || !liveStats.value?.captureFps) return false;
  const targetFps = t.kind === "screen"
    ? streamSettings.screen.preset.fps
    : streamSettings.camera.preset.fps;
  return liveStats.value.captureFps < targetFps - 5;
});

function layerLabel(rid: string | undefined, index: number, total: number): string {
  if (rid) return rid.toUpperCase();
  // No rid → derive from position. LiveKit usually orders high→low.
  if (total === 1) return "Single";
  if (total === 2) return index === 0 ? "HIGH" : "LOW";
  return ["HIGH", "MED", "LOW"][index] ?? `L${index}`;
}
onUnmounted(() => { if (statsTimer) clearInterval(statsTimer); });

// ── Right-click context menu ──
const canMuteMembers = computed(() => perms.has(state.value?.permissions ?? 0, perms.MUTE_MEMBERS));
const canDeafenMembers = computed(() => perms.has(state.value?.permissions ?? 0, perms.DEAFEN_MEMBERS));

const ctxMenu = ref<{ x: number; y: number; items: MenuItem[] } | null>(null);

function openContextMenu(tile: Tile, e: MouseEvent) {
  const items: MenuItem[] = [];
  const isSelf = tile.uid === state.value?.user?.id;

  // Spotlight toggle
  items.push({
    label: spotlightKey.value === tile.key ? "Sortir du focus" : "Mettre en focus",
    icon: spotlightKey.value === tile.key ? MonitorOff : Star,
    action: () => setSpotlight(tile),
  });

  if (tile.kind === "camera" || tile.kind === "screen") {
    items.push({ label: "Plein ecran", icon: Maximize, action: () => onFullscreen(tile) });
    items.push({ label: "Stats", icon: Info, action: () => onShowStats(tile) });
  }

  if (tile.kind === "screen" && !isSelf) {
    items.push({ label: "Arreter de regarder", icon: EyeOff, action: () => onToggleWatch(tile), danger: true });
  } else if (tile.kind === "screen-pending" && !isSelf) {
    items.push({ label: "Regarder le stream", icon: Monitor, action: () => onToggleWatch(tile) });
  }

  // Quality selector for remote video tiles we're actually watching.
  if (!isSelf && (tile.kind === "camera" || tile.kind === "screen")) {
    const src = tile.kind === "screen" ? "screen_share" : "camera";
    const current = getViewQuality(tile.identity, src);
    const options: { id: ViewQuality; label: string }[] = [
      { id: "auto",   label: "Auto (adaptatif)" },
      { id: "high",   label: "Haute" },
      { id: "medium", label: "Moyenne" },
      { id: "low",    label: "Basse" },
    ];
    for (const opt of options) {
      items.push({
        label: `${current === opt.id ? "✓ " : ""}Qualite : ${opt.label}`,
        icon: Gauge,
        action: () => setViewQuality(tile.identity, src, opt.id),
      });
    }
  }

  if (!isSelf && hasAudio(tile.uid)) {
    items.push({
      label: isRemoteMuted(tile.uid) ? "Reactiver le son" : "Mute pour moi",
      icon: isRemoteMuted(tile.uid) ? Volume2 : VolumeX,
      action: () => toggleLocalMute(tile.uid),
    });
  }

  if (!isSelf && canMuteMembers.value && tile.voiceState) {
    items.push({
      label: tile.voiceState.force_muted ? "Unmute (admin)" : "Force mute",
      icon: MicOff,
      action: () => forceMute(tile.uid, !tile.voiceState!.force_muted),
      danger: !tile.voiceState.force_muted,
    });
  }
  if (!isSelf && canDeafenMembers.value && tile.voiceState) {
    items.push({
      label: tile.voiceState.force_deafened ? "Undeafen (admin)" : "Force deafen",
      icon: HeadphoneOff,
      action: () => forceDeafen(tile.uid, !tile.voiceState!.force_deafened),
      danger: !tile.voiceState.force_deafened,
    });
  }

  if (items.length) ctxMenu.value = { x: e.clientX, y: e.clientY, items };
}

// ── User card popup (same component used elsewhere — chat hover, etc.) ──
const cardUser = ref<User | null>(null);
const cardX = ref(0);
const cardY = ref(0);

function openUserCard(uid: number, e: MouseEvent) {
  const user = state.value?.users.get(uid);
  if (!user) return;
  const el = e.currentTarget as HTMLElement;
  const rect = el.getBoundingClientRect();
  cardX.value = rect.right + 8;
  cardY.value = rect.top;
  cardUser.value = user;
}

/** Context menu for the compact user cards — subset: mute-for-me + admin actions. */
function openUserContextMenu(p: VoiceUserEntry, e: MouseEvent) {
  const items: MenuItem[] = [];
  const isSelf = p.uid === state.value?.user?.id;

  if (!isSelf && hasAudio(p.uid)) {
    items.push({
      label: isRemoteMuted(p.uid) ? "Reactiver le son" : "Mute pour moi",
      icon: isRemoteMuted(p.uid) ? Volume2 : VolumeX,
      action: () => toggleLocalMute(p.uid),
    });
  }
  if (!isSelf && canMuteMembers.value) {
    items.push({
      label: p.voiceState.force_muted ? "Unmute (admin)" : "Force mute",
      icon: MicOff,
      action: () => forceMute(p.uid, !p.voiceState.force_muted),
      danger: !p.voiceState.force_muted,
    });
  }
  if (!isSelf && canDeafenMembers.value) {
    items.push({
      label: p.voiceState.force_deafened ? "Undeafen (admin)" : "Force deafen",
      icon: HeadphoneOff,
      action: () => forceDeafen(p.uid, !p.voiceState.force_deafened),
      danger: !p.voiceState.force_deafened,
    });
  }

  if (items.length) ctxMenu.value = { x: e.clientX, y: e.clientY, items };
}

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
  background: var(--bg-primary);
  min-height: 0;
  overflow: hidden;
  position: relative;
}

/* ── Idle states ── */
.voice-idle {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 24px;
  padding: 40px;
}
.voice-idle-status {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  font-size: 1rem;
  color: var(--text-muted);
}
.voice-idle-status.connecting { color: var(--text-normal); }
.voice-idle-status.connected { color: var(--green); }
.voice-idle-status.error { color: var(--danger); }
.spin { animation: spin 1s linear infinite; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }

.voice-join-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  width: auto;
  padding: 10px 24px;
  background: var(--green);
  color: var(--text-bright);
  font-weight: 600;
  font-size: 0.875rem;
  border-radius: 8px;
}
.voice-join-btn:hover { opacity: 0.9; }

/* ── Stage (connected) ── */
.voice-stage {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: 12px;
  gap: 12px;
}

/* Stream area (only rendered when streams exist) */
.voice-streams {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.voice-spotlight {
  flex: 1;
  min-height: 0;
  display: flex;
}

.voice-spotlight .ptile {
  width: 100%;
  height: 100%;
}

/* ── Stream grid ── */
.voice-grid {
  display: grid;
  gap: 8px;
  grid-auto-rows: minmax(0, 1fr);
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  flex: 1;
  min-height: 0;
}

.voice-grid > * {
  aspect-ratio: 16 / 9;
  min-height: 0;
}

/* Thumbnail strip below the spotlight */
.voice-grid.thumbnails {
  flex: 0 0 auto;
  display: flex;
  flex-wrap: nowrap;
  gap: 8px;
  overflow-x: auto;
  overflow-y: hidden;
  height: 130px;
  align-items: stretch;
}
.voice-grid.thumbnails > * {
  flex: 0 0 auto;
  width: 220px;
  height: 100%;
  aspect-ratio: auto;
}
.voice-grid.thumbnails.empty { display: none; }

/* ── User cards ── */
.voice-users {
  /* No streams mode → centered grid */
  flex: 1;
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, max-content));
  gap: 16px;
  justify-content: center;
  align-content: center;
  min-height: 0;
  overflow-y: auto;
  padding: 16px;
}

/* Streams present → compact horizontal strip, centered, no background */
.voice-users.strip {
  flex: 0 0 auto;
  display: flex;
  flex-wrap: nowrap;
  gap: 4px;
  padding: 6px 8px;
  overflow-x: auto;
  overflow-y: hidden;
  justify-content: center;
  align-items: center;
}

/* Stats popup (unchanged shape) */
.stats-popup {
  position: absolute;
  inset: 0;
  background: var(--overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 50;
}
.stats-card {
  background: var(--bg-primary);
  border-radius: 8px;
  width: 320px;
  border: 1px solid var(--border);
  overflow: hidden;
}
.stats-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  background: var(--bg-secondary);
  font-weight: 600;
  font-size: 0.8125rem;
  color: var(--header-primary);
}
.stats-close {
  width: 24px;
  height: 24px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: var(--text-muted);
  border: none;
  border-radius: 4px;
  cursor: pointer;
}
.stats-close:hover { background: var(--bg-modifier-hover); color: var(--text-normal); }
.stats-body {
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.stats-loading {
  text-align: center;
  font-size: 0.8125rem;
  color: var(--text-faint);
  padding: 24px;
}
.stats-row {
  display: flex;
  justify-content: space-between;
  font-size: 0.8125rem;
  color: var(--text-normal);
}
.stats-row span:first-child { color: var(--text-muted); }

/* ── Stats sections / health / layers ── */
.stats-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding-top: 8px;
  border-top: 1px solid var(--border);
}
.stats-section:first-child { border-top: none; padding-top: 0; }

.stats-section-title {
  font-size: 0.625rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-muted);
  margin-bottom: 2px;
}

.stats-health {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 6px;
  background: var(--bg-secondary);
  border-left: 3px solid var(--text-muted);
}
.stats-health.ok { border-left-color: var(--green); color: var(--green); }
.stats-health.warn { border-left-color: #f0a020; color: #f0a020; }
.stats-health.neutral { border-left-color: var(--text-muted); color: var(--text-muted); }

.stats-health-text { flex: 1; min-width: 0; }
.stats-health-title {
  font-size: 0.8125rem;
  font-weight: 600;
}
.stats-health-sub {
  font-size: 0.6875rem;
  color: var(--text-muted);
  margin-top: 2px;
  line-height: 1.3;
}

.stats-layers {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.75rem;
  font-variant-numeric: tabular-nums;
}
.stats-layers th {
  text-align: left;
  font-weight: 600;
  color: var(--text-muted);
  font-size: 0.6875rem;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  padding: 4px 6px;
}
.stats-layers td {
  padding: 4px 6px;
  color: var(--text-normal);
  border-top: 1px solid var(--border);
}
.stats-layers th:last-child,
.stats-layers td:last-child { text-align: right; }
.stats-layers tr.inactive td { color: var(--text-faint); font-style: italic; }

.stats-cap-badge {
  font-size: 0.5625rem;
  background: #f0a020;
  color: #1a1a1a;
  padding: 1px 6px;
  border-radius: 8px;
  text-transform: uppercase;
  letter-spacing: 0;
  font-weight: 700;
  margin-left: 6px;
}
.stats-warn { color: #f0a020; font-weight: 600; }
.stats-hint {
  font-size: 0.6875rem;
  color: var(--text-muted);
  line-height: 1.35;
  background: var(--bg-secondary);
  padding: 6px 8px;
  border-radius: 4px;
  margin-top: 4px;
}
</style>
