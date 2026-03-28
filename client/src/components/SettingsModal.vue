<template>
  <div class="modal-overlay" @click.self="close" @keydown.esc.window="close">
    <div class="settings">
      <div class="settings-sidebar">
        <div
          v-for="tab in tabs"
          :key="tab.id"
          class="settings-tab"
          :class="{ active: activeTab === tab.id }"
          @click="activeTab = tab.id"
        >
          <component :is="tab.icon" :size="16" />
          <span>{{ tab.label }}</span>
        </div>
      </div>

      <div class="settings-content">
        <div class="settings-header">
          <h2>{{ activeTabLabel }}</h2>
          <button class="settings-close" @click="close">
            <X :size="20" />
          </button>
        </div>

        <!-- Profil -->
        <div v-if="activeTab === 'profile'" class="settings-body">
          <div class="settings-section">
            <label>Display name par defaut</label>
            <p class="settings-hint">Utilise comme nom par defaut quand tu rejoins un nouveau serveur.</p>
            <div class="settings-input-row">
              <input v-model="defaultDisplayName" type="text" placeholder="Mon pseudo" maxlength="32" />
              <button class="settings-save-btn" @click="saveDefaultName">Sauvegarder</button>
            </div>
          </div>

        </div>

        <!-- Audio -->
        <div v-if="activeTab === 'audio'" class="settings-body">
          <div class="settings-section">
            <label>Microphone</label>
            <Dropdown
              v-model="selectedMic"
              :options="micOptions"
              placeholder="Par defaut"
              @update:model-value="onMicChange"
            />
          </div>

          <div class="settings-section">
            <label>Haut-parleur</label>
            <Dropdown
              v-model="selectedSpeaker"
              :options="speakerOptions"
              placeholder="Par defaut"
              @update:model-value="onSpeakerChange"
            />
          </div>
        </div>

        <!-- A propos -->
        <div v-if="activeTab === 'about'" class="settings-body">
          <div class="settings-section">
            <div class="about-app">
              <h3>Sorry</h3>
              <p class="about-version">v0.1.0</p>
              <p class="about-tagline">Open-source, self-hosted, no account required.</p>
              <p class="about-desc">
                Pas de tracking, pas de telemetrie, pas de compte centralise.
                Ton serveur, tes donnees, tes regles.
              </p>
              <p class="about-stack">Rust + Axum + Vue.js + LiveKit</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { X, UserRound, Volume2, Info } from "lucide-vue-next";
import Dropdown from "./Dropdown.vue";
import { store } from "../store";

const activeTab = ref("profile");

const tabs = [
  { id: "profile", label: "Profil", icon: UserRound },
  { id: "audio", label: "Audio", icon: Volume2 },
  { id: "about", label: "A propos", icon: Info },
];

const activeTabLabel = computed(() => tabs.find((t) => t.id === activeTab.value)?.label ?? "");

// Profile
const defaultDisplayName = ref(localStorage.getItem("defaultDisplayName") || "");

function saveDefaultName() {
  localStorage.setItem("defaultDisplayName", defaultDisplayName.value.trim());
}


// Audio
const microphones = ref<MediaDeviceInfo[]>([]);
const speakers = ref<MediaDeviceInfo[]>([]);
const selectedMic = ref(localStorage.getItem("audioInputDevice") || "");
const selectedSpeaker = ref(localStorage.getItem("audioOutputDevice") || "");

onMounted(async () => {
  try {
    const devices = await navigator.mediaDevices.enumerateDevices();
    microphones.value = devices.filter((d) => d.kind === "audioinput");
    speakers.value = devices.filter((d) => d.kind === "audiooutput");
  } catch {
    // Permissions pas encore accordees
  }
});

const micOptions = computed(() => [
  { value: "", label: "Par defaut" },
  ...microphones.value.map((d) => ({
    value: d.deviceId,
    label: d.label || `Micro ${d.deviceId.slice(0, 8)}`,
  })),
]);

const speakerOptions = computed(() => [
  { value: "", label: "Par defaut" },
  ...speakers.value.map((d) => ({
    value: d.deviceId,
    label: d.label || `Speaker ${d.deviceId.slice(0, 8)}`,
  })),
]);

function onMicChange() {
  localStorage.setItem("audioInputDevice", selectedMic.value);
}

function onSpeakerChange() {
  localStorage.setItem("audioOutputDevice", selectedSpeaker.value);
}

function close() {
  store.showSettingsModal = false;
}
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.settings {
  background: var(--bg-primary);
  border-radius: 8px;
  width: 800px;
  height: 600px;
  display: flex;
  overflow: hidden;
}

.settings-sidebar {
  width: 190px;
  background: var(--bg-secondary);
  padding: 16px 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.settings-tab {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-muted);
  cursor: pointer;
  transition: background 0.1s, color 0.1s;
}

.settings-tab:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
}

.settings-tab.active {
  background: var(--bg-modifier-active);
  color: var(--header-primary);
}

.settings-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px 12px;
}

.settings-header h2 {
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--header-primary);
}

.settings-close {
  width: 32px;
  height: 32px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}

.settings-close:hover {
  color: var(--text-normal);
  background: var(--bg-modifier-hover);
  box-shadow: none;
}

.settings-body {
  padding: 0 24px 24px;
}

.settings-section {
  margin-bottom: 24px;
}

.settings-section label {
  display: block;
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-muted);
  margin-bottom: 4px;
}

.settings-hint {
  font-size: 0.75rem;
  color: var(--text-faint);
  margin-bottom: 8px;
}

.settings-input-row {
  display: flex;
  gap: 8px;
}

.settings-input-row input {
  flex: 1;
  padding: 8px 10px;
  border-radius: 6px;
  border: none;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.875rem;
  font-family: inherit;
  outline: none;
}

.settings-input-row input::placeholder {
  color: var(--text-faint);
}

.settings-save-btn {
  width: auto;
  padding: 8px 16px;
  margin: 0;
  font-size: 0.8125rem;
  border-radius: 6px;
}

.settings-success {
  font-size: 0.75rem;
  color: var(--green);
  margin-top: 6px;
}


.about-app {
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 60px 0;
}

.about-app h3 {
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--header-primary);
  margin-bottom: 4px;
}

.about-version {
  font-size: 0.8125rem;
  color: var(--text-faint);
  margin-bottom: 12px;
}

.about-tagline {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--accent);
  margin-bottom: 12px;
}

.about-desc {
  font-size: 0.8125rem;
  color: var(--text-muted);
  line-height: 1.5;
  margin-bottom: 16px;
  max-width: 320px;
}

.about-stack {
  font-size: 0.75rem;
  color: var(--text-faint);
}
</style>
