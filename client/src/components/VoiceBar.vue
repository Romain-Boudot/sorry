<template>
  <div class="voice-bar" v-if="voiceEntry">
    <div class="voice-bar-main">
      <div class="voice-bar-info">
        <div class="voice-bar-status" :class="voiceEntry.state.voiceStatus">
          <Loader2 v-if="voiceEntry.state.voiceStatus === 'connecting'" :size="14" class="spin" />
          <AlertCircle v-else-if="voiceEntry.state.voiceStatus === 'error'" :size="14" />
          <Phone v-else :size="14" />
          <span>{{ statusLabel }}</span>
        </div>
        <div class="voice-bar-server">{{ voiceEntry.server.name }} - {{ channelName }}</div>
      </div>
      <div class="voice-bar-actions">
        <div ref="connRef" class="conn-wrapper" @mouseenter="openTooltip" @mouseleave="showTooltip = false">
          <div class="conn-indicator" :class="connClass">
            <Loader2 v-if="!connStats" :size="14" class="spin" />
            <RadioTower v-else :size="14" />
          </div>
        </div>
        <Teleport to="body">
          <Transition name="tooltip">
            <div v-if="showTooltip" class="conn-tooltip" :style="tooltipStyle">
              <template v-if="connStats">
                <div class="tooltip-row tooltip-transport">
                  <span class="tooltip-dot" :class="connClass"></span>
                  <span>{{ transportLabel }}</span>
                  <span v-if="connStats.rtt" class="tooltip-rtt">{{ connStats.rtt }}ms</span>
                </div>
              </template>
              <template v-else>
                <div class="tooltip-row tooltip-pending">Analyse de la connexion...</div>
              </template>
            </div>
          </Transition>
        </Teleport>
        <button
          class="voice-bar-btn danger"
          @click="leaveVoiceChannel()"
          title="Deconnecter"
        >
          <PhoneOff :size="16" />
        </button>
      </div>
    </div>
    <div v-if="canStream && voiceEntry.state.voiceStatus === 'connected'" class="voice-bar-share">
      <div class="share-group">
        <button
          class="share-btn"
          :class="{ active: isCameraOn }"
          @click="onToggleCamera"
          title="Webcam"
        >
          <Loader2 v-if="cameraLoading" :size="15" class="spin" />
          <Video v-else-if="isCameraOn" :size="15" />
          <VideoOff v-else :size="15" />
          <span>Webcam</span>
        </button>
        <button
          class="share-chevron"
          :class="{ active: openPopover === 'camera' }"
          @click="togglePopover('camera')"
          title="Qualite webcam"
        >
          <ChevronUp :size="13" />
        </button>
      </div>
      <div class="share-group">
        <button
          class="share-btn"
          :class="{ active: isScreenSharing }"
          @click="onScreenShareClick"
          title="Ecran"
        >
          <MonitorOff v-if="isScreenSharing" :size="15" />
          <Monitor v-else :size="15" />
          <span>Ecran</span>
        </button>
        <button
          class="share-chevron"
          :class="{ active: openPopover === 'screen' }"
          @click="togglePopover('screen')"
          title="Qualite ecran"
        >
          <ChevronUp :size="13" />
        </button>
      </div>
      <button
        class="share-btn disabled"
        disabled
        title="Bientot"
      >
        <Radio :size="15" />
        <span>OBS</span>
      </button>
    </div>
  </div>

  <StreamQualityPopover
    v-if="openPopover"
    :kind="openPopover"
    @close="openPopover = null"
  />
</template>

<script setup lang="ts">
import { computed, ref, watch, onUnmounted } from "vue";
import { Phone, PhoneOff, Loader2, AlertCircle, RadioTower, Monitor, MonitorOff, Video, VideoOff, Radio, ChevronUp } from "lucide-vue-next";
import { store, leaveVoiceChannel, toggleScreenShare, toggleCamera } from "../store";
import { getConnectionStats, type ConnectionStats } from "../voice";
import * as perms from "../permissions";
import StreamQualityPopover from "./voice/StreamQualityPopover.vue";

const showTooltip = ref(false);
const connRef = ref<HTMLElement | null>(null);
const tooltipStyle = ref<Record<string, string>>({});

function openTooltip() {
  if (connRef.value) {
    const rect = connRef.value.getBoundingClientRect();
    const centerX = rect.left + rect.width / 2;
    tooltipStyle.value = {
      position: "fixed",
      bottom: `${window.innerHeight - rect.top + 8}px`,
      left: `${centerX}px`,
      transform: "translateX(-50%)",
    };
  }
  showTooltip.value = true;
}

const isScreenSharing = computed(() => voiceEntry.value?.state.isScreenSharing ?? false);
const isCameraOn = computed(() => voiceEntry.value?.state.isCameraOn ?? false);

const cameraLoading = ref(false);

async function onToggleCamera() {
  cameraLoading.value = true;
  await toggleCamera();
  cameraLoading.value = false;
}

const canStream = computed(() => {
  const st = voiceEntry.value?.state;
  if (!st) return false;
  return perms.has(st.permissions, perms.STREAM);
});

function onScreenShareClick() {
  toggleScreenShare();
}

// ── Quality modal ──
const openPopover = ref<"camera" | "screen" | null>(null);

function togglePopover(kind: "camera" | "screen") {
  openPopover.value = openPopover.value === kind ? null : kind;
}

const voiceEntry = computed(() => {
  for (const server of store.savedServers) {
    const state = store.serverStates.get(server.id);
    if (state && state.voiceStatus !== "idle") return { server, state };
  }
  return null;
});

const channelName = computed(() => {
  if (!voiceEntry.value) return "";
  const { state } = voiceEntry.value;
  const targetId = state.voiceChannelId ?? state.voiceConnectingChannelId;
  const ch = state.channels.find((c) => c.id === targetId);
  return ch?.name;
});

const statusLabel = computed(() => {
  if (!voiceEntry.value) return "";
  const { state } = voiceEntry.value;
  if (state.voiceStatus === "connecting") return "Connexion...";
  if (state.voiceStatus === "error") return "Erreur de connexion";
  const ch = state.channels.find((c) => c.id === state.voiceChannelId);
  return ch?.name ?? "";
});

// Connection stats
const connStats = ref<ConnectionStats | null>(null);
let statsInterval: ReturnType<typeof setInterval> | null = null;

async function pollStats() {
  connStats.value = await getConnectionStats();
}

watch(() => voiceEntry.value?.state.voiceStatus, (status) => {
  if (statsInterval) { clearInterval(statsInterval); statsInterval = null; }
  if (status === "connected") {
    setTimeout(pollStats, 1500);
    statsInterval = setInterval(pollStats, 3000);
  } else {
    connStats.value = null;
  }
}, { immediate: true });

onUnmounted(() => {
  if (statsInterval) clearInterval(statsInterval);
});

const connClass = computed(() => connStats.value?.transport ?? "pending");

const transportLabel = computed(() => {
  switch (connStats.value?.transport) {
    case "udp": return "UDP direct";
    case "tcp": return "TCP fallback";
    case "turn-udp": return "Relais TURN (UDP)";
    case "turn-tcp": return "Relais TURN (TCP)";
    default: return "Connexion...";
  }
});
</script>

<style scoped>
.voice-bar {
  border-bottom: 1px solid var(--border);
}

.voice-bar-main {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
}

.voice-bar-share {
  display: flex;
  gap: 4px;
  padding: 0 8px 8px;
}

.share-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 5px 0;
  border-radius: 6px;
  background: var(--bg-tertiary);
  color: var(--text-muted);
  font-size: 0.6875rem;
  font-weight: 600;
  border: none;
  cursor: pointer;
  margin: 0;
  transition: background 0.1s, color 0.1s;
}

.share-btn:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
  box-shadow: none;
}

.share-btn.active {
  background: rgba(59, 165, 93, 0.15);
  color: var(--green);
}

.share-btn.active:hover {
  background: rgba(59, 165, 93, 0.25);
  box-shadow: none;
}

.share-btn.disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

/* Group: main share button + chevron sit side-by-side, sharing one rounded container. */
.share-group {
  flex: 1;
  display: flex;
  gap: 1px;
  border-radius: 6px;
  overflow: hidden;
  background: var(--bg-tertiary);
}
.share-group .share-btn {
  border-radius: 0;
  flex: 1;
}
.share-chevron {
  width: 22px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-tertiary);
  color: var(--text-muted);
  border: none;
  border-radius: 0;
  cursor: pointer;
  transition: background 0.1s, color 0.1s;
}
.share-chevron:hover { background: var(--bg-modifier-hover); color: var(--text-normal); box-shadow: none; }
.share-chevron.active { background: var(--bg-modifier-hover); color: var(--text-bright); }

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

.voice-bar-status.connecting { color: var(--text-muted); }
.voice-bar-status.error { color: var(--danger); }

.voice-bar-server {
  font-size: 0.6875rem;
  color: var(--text-muted);
  font-weight: 400;
}

.voice-bar-actions {
  display: flex;
  align-items: center;
  gap: 4px;
}

.conn-wrapper {
  position: relative;
}

.conn-indicator {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  cursor: default;
  transition: background 0.15s;
}

.conn-indicator:hover {
  background: var(--bg-modifier-hover);
}

.conn-indicator.udp { color: var(--green); }
.conn-indicator.tcp { color: var(--yellow, #f0b232); }
.conn-indicator.turn-udp { color: var(--blue, #5865f2); }
.conn-indicator.turn-tcp { color: var(--orange, #e67e22); }
.conn-indicator.unknown,
.conn-indicator.pending { color: var(--text-faint); }


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
  background: var(--danger-bg-hover);
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

<style>
.conn-tooltip {
  background: var(--bg-floating, #18191c);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 12px;
  min-width: 180px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  z-index: 9999;
  pointer-events: none;
}

.conn-tooltip .tooltip-row {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.75rem;
  white-space: nowrap;
}

.conn-tooltip .tooltip-transport {
  font-weight: 600;
  color: var(--text-normal);
}

.conn-tooltip .tooltip-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.conn-tooltip .tooltip-dot.udp { background: var(--green); }
.conn-tooltip .tooltip-dot.tcp { background: var(--yellow, #f0b232); }
.conn-tooltip .tooltip-dot.turn-udp { background: var(--blue, #5865f2); }
.conn-tooltip .tooltip-dot.turn-tcp { background: var(--orange, #e67e22); }
.conn-tooltip .tooltip-dot.unknown,
.conn-tooltip .tooltip-dot.pending { background: var(--text-faint); }

.conn-tooltip .tooltip-rtt {
  margin-left: auto;
  color: var(--text-muted);
  font-weight: 400;
  font-variant-numeric: tabular-nums;
}

.conn-tooltip .tooltip-pending {
  color: var(--text-muted);
  font-style: italic;
}

.tooltip-enter-active { transition: opacity 0.12s, transform 0.12s; }
.tooltip-leave-active { transition: opacity 0.08s, transform 0.08s; }
.tooltip-enter-from,
.tooltip-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(4px);
}
</style>
