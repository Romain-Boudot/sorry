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
          <p class="settings-section-intro">Ces parametres sont stockes localement sur cet appareil. Quand tu rejoins un nouveau serveur, ton avatar et ton pseudo par defaut seront utilises automatiquement. Tu pourras les changer ensuite pour chaque serveur independamment.</p>

          <div class="settings-section">
            <label>Avatar par defaut</label>
            <p class="settings-hint">Applique automatiquement quand tu rejoins un nouveau serveur.</p>
            <div class="avatar-setting">
              <div class="avatar-preview" @click="avatarInput?.click()">
                <img v-if="defaultAvatarPreview" :src="defaultAvatarPreview" />
                <span v-else class="avatar-placeholder">?</span>
                <div class="avatar-overlay">
                  <Camera :size="16" />
                </div>
              </div>
              <div class="avatar-actions">
                <button class="settings-save-btn" @click="avatarInput?.click()">Changer</button>
                <button v-if="defaultAvatarPreview" class="settings-remove-btn" @click="removeDefaultAvatar">Supprimer</button>
              </div>
              <input ref="avatarInput" type="file" accept="image/png,image/jpeg,image/gif,image/webp" hidden @change="onDefaultAvatarSelect" />
            </div>
          </div>

          <div class="settings-section">
            <label>Display name par defaut</label>
            <p class="settings-hint">Applique automatiquement quand tu rejoins un nouveau serveur. Pour changer ton pseudo sur un serveur existant, passe par les parametres du serveur.</p>
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
            <div class="mic-test">
              <button class="mic-test-btn" :class="{ active: micTesting }" @click="toggleMicTest">
                <Mic :size="14" />
                {{ micTesting ? 'Arreter le test' : 'Tester le micro' }}
              </button>
              <div v-if="micTesting" class="mic-level-bar">
                <div class="mic-level-fill" :style="`width:${micLevel}%`"></div>
              </div>
            </div>
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
import { X, UserRound, Volume2, Info, Mic, Camera } from "lucide-vue-next";
import Dropdown from "./ui/Dropdown.vue";
import { store } from "../store";
import { onUnmounted } from "vue";
import { switchMicrophone, switchSpeaker } from "../voice";

const activeTab = ref("profile");

const tabs = [
  { id: "profile", label: "Profil", icon: UserRound },
  { id: "audio", label: "Audio", icon: Volume2 },
  { id: "about", label: "A propos", icon: Info },
];

const activeTabLabel = computed(() => tabs.find((t) => t.id === activeTab.value)?.label ?? "");

// Profile
const defaultDisplayName = ref(localStorage.getItem("defaultDisplayName") || "");
const avatarInput = ref<HTMLInputElement>();
const defaultAvatarPreview = ref<string | null>(null);

// Load saved default avatar preview
{
  const saved = localStorage.getItem("defaultAvatarPreview");
  if (saved) defaultAvatarPreview.value = saved;
}

function saveDefaultName() {
  localStorage.setItem("defaultDisplayName", defaultDisplayName.value.trim());
}

function onDefaultAvatarSelect(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  // Store the file as a data URL for preview + later use
  const reader = new FileReader();
  reader.onload = () => {
    defaultAvatarPreview.value = reader.result as string;
    localStorage.setItem("defaultAvatarPreview", reader.result as string);
    localStorage.setItem("defaultAvatarName", file.name);
    localStorage.setItem("defaultAvatarType", file.type);
  };
  reader.readAsDataURL(file);
  input.value = "";
}

function removeDefaultAvatar() {
  defaultAvatarPreview.value = null;
  localStorage.removeItem("defaultAvatarPreview");
  localStorage.removeItem("defaultAvatarName");
  localStorage.removeItem("defaultAvatarType");
}


// Audio
const microphones = ref<MediaDeviceInfo[]>([]);
const speakers = ref<MediaDeviceInfo[]>([]);
const selectedMic = ref(store.audioInputDevice);
const selectedSpeaker = ref(store.audioOutputDevice);

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
  store.audioInputDevice = selectedMic.value;
  localStorage.setItem("audioInputDevice", selectedMic.value);
  if (selectedMic.value) switchMicrophone(selectedMic.value);
}

function onSpeakerChange() {
  store.audioOutputDevice = selectedSpeaker.value;
  localStorage.setItem("audioOutputDevice", selectedSpeaker.value);
  if (selectedSpeaker.value) switchSpeaker(selectedSpeaker.value);
}

// Mic test
const micTesting = ref(false);
const micLevel = ref(0);
let micStream: MediaStream | null = null;
let micAnalyser: AnalyserNode | null = null;
let micAnimFrame: number | null = null;

async function toggleMicTest() {
  if (micTesting.value) {
    stopMicTest();
    return;
  }
  try {
    const constraints: MediaStreamConstraints = {
      audio: selectedMic.value ? { deviceId: { exact: selectedMic.value } } : true,
    };
    micStream = await navigator.mediaDevices.getUserMedia(constraints);
    const ctx = new AudioContext();
    const source = ctx.createMediaStreamSource(micStream);
    micAnalyser = ctx.createAnalyser();
    micAnalyser.fftSize = 256;
    source.connect(micAnalyser);
    micTesting.value = true;
    updateMicLevel();
  } catch {}
}

function updateMicLevel() {
  if (!micAnalyser) return;
  const data = new Uint8Array(micAnalyser.frequencyBinCount);
  micAnalyser.getByteFrequencyData(data);
  const avg = data.reduce((a, b) => a + b, 0) / data.length;
  micLevel.value = Math.min(100, avg * 1.5);
  micAnimFrame = requestAnimationFrame(updateMicLevel);
}

function stopMicTest() {
  micTesting.value = false;
  micLevel.value = 0;
  if (micStream) {
    micStream.getTracks().forEach((t) => t.stop());
    micStream = null;
  }
  if (micAnimFrame) {
    cancelAnimationFrame(micAnimFrame);
    micAnimFrame = null;
  }
  micAnalyser = null;
}

function close() {
  stopMicTest();
  store.showSettingsModal = false;
}
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: var(--overlay);
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

.settings-section-intro {
  font-size: 0.8125rem;
  color: var(--text-faint);
  line-height: 1.5;
  margin-bottom: 20px;
  padding: 12px;
  background: var(--bg-secondary);
  border-radius: 6px;
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


.avatar-setting {
  display: flex;
  align-items: center;
  gap: 16px;
}

.avatar-preview {
  width: 72px;
  height: 72px;
  border-radius: 50%;
  background: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  position: relative;
  overflow: hidden;
  flex-shrink: 0;
}

.avatar-preview img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.avatar-placeholder {
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--text-faint);
}

.avatar-overlay {
  position: absolute;
  inset: 0;
  background: var(--overlay-light);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-bright);
  opacity: 0;
  transition: opacity 0.15s;
}

.avatar-preview:hover .avatar-overlay {
  opacity: 1;
}

.avatar-actions {
  display: flex;
  gap: 8px;
}

.settings-remove-btn {
  width: auto;
  padding: 8px 16px;
  margin: 0;
  font-size: 0.8125rem;
  border-radius: 6px;
  background: transparent;
  color: var(--danger);
  border: 1px solid var(--danger);
}

.settings-remove-btn:hover {
  background: var(--danger-bg-hover);
  box-shadow: none;
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

.mic-test {
  margin-top: 10px;
}

.mic-test-btn {
  width: auto;
  padding: 6px 12px;
  margin: 0;
  font-size: 0.75rem;
  border-radius: 6px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: var(--bg-tertiary);
  color: var(--text-muted);
}
.mic-test-btn:hover { background: var(--bg-modifier-hover); box-shadow: none; }
.mic-test-btn.active { background: var(--danger); color: var(--text-bright); }
.mic-test-btn.active:hover { background: var(--danger); opacity: 0.9; box-shadow: none; }

.mic-level-bar {
  margin-top: 8px;
  height: 6px;
  border-radius: 3px;
  background: var(--bg-tertiary);
  overflow: hidden;
}

.mic-level-fill {
  height: 100%;
  border-radius: 3px;
  background: var(--green);
}
</style>
