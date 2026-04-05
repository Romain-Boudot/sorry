<template>
  <div class="audio-controls">
    <!-- Mute button + dropdown -->
    <div class="audio-btn-group">
      <button
        class="audio-btn"
        :class="{ active: isMuted }"
        @click="toggleMute()"
        :title="isMuted ? 'Unmute' : 'Mute'"
      >
        <MicOff v-if="isMuted" :size="18" />
        <Mic v-else :size="18" />
      </button>
      <button
        class="audio-dropdown-btn"
        :class="{ active: isMuted }"
        @click.stop="toggleDropdown('mic')"
        title="Choisir le micro"
      >
        <ChevronUp :size="14" />
      </button>
    </div>

    <!-- Deafen button + dropdown -->
    <div class="audio-btn-group">
      <button
        class="audio-btn"
        :class="{ active: isDeafened }"
        @click="toggleDeafen()"
        :title="isDeafened ? 'Undeafen' : 'Deafen'"
      >
        <HeadphoneOff v-if="isDeafened" :size="18" />
        <Headphones v-else :size="18" />
      </button>
      <button
        class="audio-dropdown-btn"
        :class="{ active: isDeafened }"
        @click.stop="toggleDropdown('speaker')"
        title="Choisir le haut-parleur"
      >
        <ChevronUp :size="14" />
      </button>
    </div>

    <!-- Device dropdown -->
    <Teleport to="body">
      <div
        v-if="dropdown"
        class="device-dropdown"
        :style="{ left: dropdownPos.x + 'px', bottom: dropdownPos.y + 'px' }"
        @click.stop
      >
        <div class="device-dropdown-title">
          {{ dropdown === 'mic' ? 'Microphone' : 'Haut-parleur' }}
        </div>
        <div
          v-for="device in (dropdown === 'mic' ? micDevices : speakerDevices)"
          :key="device.deviceId"
          class="device-item"
          :class="{ selected: device.deviceId === (dropdown === 'mic' ? selectedMic : selectedSpeaker) }"
          @click="selectDevice(device.deviceId)"
        >
          <Check v-if="device.deviceId === (dropdown === 'mic' ? selectedMic : selectedSpeaker)" :size="14" />
          <span>{{ device.label || 'Device ' + device.deviceId.slice(0, 8) }}</span>
        </div>
        <div v-if="(dropdown === 'mic' ? micDevices : speakerDevices).length === 0" class="device-item empty">
          Aucun appareil detecte
        </div>
      </div>
      <div v-if="dropdown" class="device-backdrop" @click="dropdown = null" />
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { Mic, MicOff, Headphones, HeadphoneOff, ChevronUp, Check } from "lucide-vue-next";
import { store, activeState, toggleMute, toggleDeafen } from "../store";
import { getAudioDevices, switchMicrophone, switchSpeaker } from "../voice";

const state = computed(() => activeState());
const isMuted = computed(() => state.value?.isMuted ?? false);
const isDeafened = computed(() => state.value?.isDeafened ?? false);
const dropdown = ref<"mic" | "speaker" | null>(null);
const dropdownPos = ref({ x: 0, y: 0 });
const micDevices = ref<MediaDeviceInfo[]>([]);
const speakerDevices = ref<MediaDeviceInfo[]>([]);
const selectedMic = computed(() => store.audioInputDevice);
const selectedSpeaker = computed(() => store.audioOutputDevice);

async function toggleDropdown(type: "mic" | "speaker") {
  if (dropdown.value === type) {
    dropdown.value = null;
    return;
  }

  const { inputs, outputs } = await getAudioDevices();
  micDevices.value = inputs;
  speakerDevices.value = outputs;

  const el = document.querySelector(`.audio-btn-group:nth-child(${type === "mic" ? 1 : 2})`) as HTMLElement;
  if (el) {
    const rect = el.getBoundingClientRect();
    dropdownPos.value = {
      x: rect.left,
      y: window.innerHeight - rect.top + 4,
    };
  }

  dropdown.value = type;
}

async function selectDevice(deviceId: string) {
  if (dropdown.value === "mic") {
    store.audioInputDevice = deviceId;
    localStorage.setItem("audioInputDevice", deviceId);
    await switchMicrophone(deviceId);
  } else if (dropdown.value === "speaker") {
    store.audioOutputDevice = deviceId;
    localStorage.setItem("audioOutputDevice", deviceId);
    await switchSpeaker(deviceId);
  }
  dropdown.value = null;
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") dropdown.value = null;
}

onMounted(() => document.addEventListener("keydown", onKeydown));
onUnmounted(() => document.removeEventListener("keydown", onKeydown));
</script>

<style scoped>
.audio-controls {
  display: flex;
  gap: 4px;
}

.audio-btn-group {
  display: flex;
  border-radius: 8px;
  overflow: hidden;
}

.audio-btn {
  width: 32px;
  height: 32px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  border: none;
  border-radius: 8px 0 0 8px;
  transition: background 0.1s, color 0.1s;
}

.audio-btn:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
  box-shadow: none;
}

.audio-btn.active {
  background: rgba(208, 80, 80, 0.15);
  color: var(--danger);
}

.audio-btn.active:hover {
  background: rgba(208, 80, 80, 0.25);
  color: var(--danger);
  box-shadow: none;
}

.audio-dropdown-btn {
  width: 16px;
  height: 32px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  border: none;
  border-left: 1px solid transparent;
  border-radius: 0 8px 8px 0;
  transition: background 0.1s, color 0.1s;
}

.audio-btn-group:hover .audio-dropdown-btn {
  border-left-color: var(--border);
}

.audio-btn-group:hover .audio-btn:not(:hover):not(.active),
.audio-btn-group:hover .audio-dropdown-btn:not(:hover):not(.active) {
  background: rgba(128, 128, 128, 0.06);
}

.audio-btn-group:hover .audio-btn.active:not(:hover),
.audio-btn-group:hover .audio-dropdown-btn.active:not(:hover) {
  background: rgba(208, 80, 80, 0.08);
}

.audio-dropdown-btn:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
  box-shadow: none;
}

.audio-dropdown-btn.active {
  background: rgba(208, 80, 80, 0.15);
  color: var(--danger);
  border-left-color: rgba(208, 80, 80, 0.3);
}

.audio-dropdown-btn.active:hover {
  background: rgba(208, 80, 80, 0.25);
  color: var(--danger);
  box-shadow: none;
}
</style>

<style>
.device-backdrop {
  position: fixed;
  inset: 0;
  z-index: 999;
}

.device-dropdown {
  position: fixed;
  z-index: 1000;
  min-width: 220px;
  max-width: 320px;
  background: var(--bg-floating);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 6px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
}

.device-dropdown-title {
  font-size: 0.6875rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-faint);
  padding: 4px 8px 6px;
}

.device-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 4px;
  font-size: 0.8125rem;
  color: var(--text-normal);
  cursor: pointer;
  transition: background 0.1s;
}

.device-item:hover {
  background: var(--bg-modifier-hover);
}

.device-item.selected {
  color: var(--accent);
}

.device-item.empty {
  color: var(--text-faint);
  cursor: default;
  font-style: italic;
}

.device-item span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
