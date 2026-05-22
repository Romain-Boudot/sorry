<template>
  <div class="app-shell" :class="{ 'is-tauri': isTauri }">
    <ToastContainer />
    <TopBar />
    <div v-if="pendingUpdate && !installing" class="update-banner">
      <Download :size="14" />
      <span class="update-banner-text">
        Mise a jour disponible : <strong>v{{ pendingUpdate.version }}</strong>
      </span>
      <button class="update-banner-btn" @click="installUpdate">Installer</button>
      <button class="update-banner-dismiss" @click="dismissUpdate" title="Plus tard">
        <X :size="14" />
      </button>
    </div>
    <div v-else-if="installing" class="update-banner installing">
      <Loader2 :size="14" class="spinner" />
      <span class="update-banner-text">
        Telechargement de la mise a jour... {{ Math.round(installProgress * 100) }}%
      </span>
      <div class="update-progress-bar">
        <div class="update-progress-fill" :style="{ width: (installProgress * 100) + '%' }"></div>
      </div>
    </div>
    <div class="app-grid">
      <ServerList />
      <Sidebar v-if="store.activeServerId" />
      <div v-if="store.activeServerId" class="main-area has-sidebar">
        <ChatHeader
          v-if="!onDmTab"
          :gallery-open="showGallery"
          :pins-open="showPins"
          @jump-to="onSearchJump"
          @toggle-gallery="showGallery = !showGallery; showPins = false"
          @toggle-pins="showPins = !showPins; showGallery = false"
        />
        <div class="main-body">
          <DmView v-if="onDmTab" />
          <template v-else>
            <VoiceView v-if="isVoice" />
            <ChatBody v-else ref="chatBodyRef" />
            <PinnedMessages v-if="showPins && !isVoice" @close="showPins = false" @jump-to="(id: number) => { onSearchJump(id); showPins = false }" />
            <FileGallery v-if="showGallery && !isVoice" @close="showGallery = false" />
            <UserList v-if="!isVoice && !showGallery && !showPins" />
          </template>
        </div>
      </div>
      <div v-else class="main-area disconnected-state">
        <div class="disconnected-content">
          <h1 class="disconnected-logo">Sorry</h1>
          <p v-if="store.savedServers.length">Selectionne un serveur pour te connecter.</p>
          <p v-else>Tu n'as rejoint aucun serveur.</p>
          <button class="disconnected-btn" @click="store.showAddServerModal = true">
            <Plus :size="16" />
            Ajouter un serveur
          </button>
        </div>
      </div>

      <div v-if="store.activeServerId" class="bottom-bar">
        <div class="bottom-card">
          <VoiceBar />
          <div class="user-row">
            <div class="user-info">
              <div class="user-avatar">{{ state?.user?.display_name?.[0]?.toUpperCase() }}</div>
              <div class="user-meta">
                <span class="user-name">{{ state?.user?.display_name }}</span>
                <span class="user-status">En ligne</span>
              </div>
            </div>
            <div class="user-controls">
              <AudioControls />
              <button class="user-control-btn" title="Parametres" @click="store.showSettingsModal = true">
                <Settings :size="18" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
    <AddServerModal v-if="store.showAddServerModal" />
    <SettingsModal v-if="store.showSettingsModal" />
    <ServerSettingsModal v-if="store.showServerSettingsModal" />
    <ChannelSettingsModal v-if="store.channelSettingsId" />
    <GroupSettingsModal v-if="store.groupSettingsId" />
    <ReauthModal v-if="store.reauthServerId" />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Settings, Plus, Download, X, Loader2 } from "lucide-vue-next";
import { store, connectAll, restoreNav, activeState, isActiveChannelVoice, resolveUserColor } from "./store";
import { autoCheckOnStartup, installUpdate, pendingUpdate, installing, installProgress } from "./composables/useUpdater";
import { setMentionResolver } from "./markdown";
import TopBar from "./components/TopBar.vue";
import ServerList from "./components/ServerList.vue";
import Sidebar from "./components/Sidebar.vue";
import ChatHeader from "./components/ChatHeader.vue";
import ChatBody from "./components/ChatBody.vue";
import DmView from "./components/DmView.vue";
import FileGallery from "./components/chat/FileGallery.vue";
import PinnedMessages from "./components/chat/PinnedMessages.vue";
import ToastContainer from "./components/ui/ToastContainer.vue";
import VoiceView from "./components/VoiceView.vue";
import UserList from "./components/UserList.vue";

import AddServerModal from "./components/AddServerModal.vue";
import SettingsModal from "./components/SettingsModal.vue";
import ServerSettingsModal from "./components/ServerSettingsModal.vue";
import ChannelSettingsModal from "./components/ChannelSettingsModal.vue";
import GroupSettingsModal from "./components/GroupSettingsModal.vue";
import ReauthModal from "./components/ReauthModal.vue";
import VoiceBar from "./components/VoiceBar.vue";
import AudioControls from "./components/AudioControls.vue";

const state = computed(() => activeState());
const isVoice = computed(() => isActiveChannelVoice());
const onDmTab = computed(() => state.value?.activeTab === "dms");
const isTauri = ref("__TAURI_INTERNALS__" in window);
const chatBodyRef = ref<InstanceType<typeof ChatBody>>();
const showGallery = ref(false);
const showPins = ref(false);

function onSearchJump(messageId: number) {
  chatBodyRef.value?.scrollToMessage(messageId);
}

function handleInviteParams(params: URLSearchParams) {
  const invite = params.get("invite");
  const server = params.get("server");
  if (invite && server) {
    store.prefillServerUrl = server;
    store.prefillInviteCode = invite;
    store.showAddServerModal = true;
  }
}

function parseDeepLink(url: string) {
  // sorry://invite?code=ABC&server=https://...
  try {
    const parsed = new URL(url);
    const params = new URLSearchParams(parsed.search);
    // Remap code → invite for consistency
    if (params.has("code")) {
      params.set("invite", params.get("code")!);
    }
    handleInviteParams(params);
  } catch {}
}

// Set up mention resolver for markdown rendering
setMentionResolver((kind, id) => {
  const s = activeState();
  if (!s) return null;
  if (kind === "user") {
    const user = s.users.get(id);
    if (!user) return null;
    return { name: user.display_name, color: resolveUserColor(id) };
  }
  // role (hide Admin role id=1)
  if (id === 1) return null;
  const role = s.roles.find((r) => r.id === id);
  if (!role) return null;
  return { name: role.name, color: role.color };
});

onMounted(async () => {
  // Parse invite from hash: #invite=CODE&server=URL
  const hash = window.location.hash.slice(1);
  if (hash) {
    handleInviteParams(new URLSearchParams(hash));
    window.location.hash = "";
  }

  // Listen for deep links (Tauri)
  if ("__TAURI_INTERNALS__" in window) {
    try {
      const { onOpenUrl } = await import("@tauri-apps/plugin-deep-link");
      await onOpenUrl((urls) => {
        for (const url of urls) parseDeepLink(url);
      });
    } catch {}

    // Also listen for single-instance forwarded events
    const { listen } = await import("@tauri-apps/api/event");
    await listen<string>("deep-link-open", (event) => {
      parseDeepLink(event.payload);
    });
  }

  restoreNav();
  connectAll();
  autoCheckOnStartup();
});

// "Plus tard" : on cache la banniere pour la session courante. Le check
// rejouera au prochain demarrage de l'app.
function dismissUpdate() {
  pendingUpdate.value = null;
}
</script>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--bg-primary);
}

.app-shell.is-tauri {
  border-radius: 10px;
  overflow: hidden;
}

.app-grid {
  flex: 1;
  display: grid;
  grid-template:
    "servers  channels  main" 1fr
    "bottom   bottom    main" auto
    / 72px 240px 1fr;
  overflow: hidden;
  background: var(--bg-secondary);
}

.main-area {
  grid-area: main;
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.main-area.has-sidebar {
  border-top: 1px solid var(--border);
}

.main-body {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.bottom-bar {
  grid-area: bottom;
  display: flex;
  flex-direction: column;
  background: var(--bg-secondary);
  padding: 0 8px 8px;
}

.bottom-card {
  background: var(--bg-floating);
  border-radius: 10px;
  border: 1px solid var(--border);
  overflow: hidden;
  box-shadow: var(--shadow-1);
}

.user-row {
  padding: 0 8px;
  height: var(--bar-height);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 2px;
  min-width: 0;
}

.user-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 0.8rem;
  flex-shrink: 0;
  color: var(--accent-fg);
}

.user-meta {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.user-name {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--header-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.user-status {
  font-size: 0.6875rem;
  color: var(--text-muted);
  font-family: var(--font-mono);
  display: flex;
  align-items: center;
  gap: 5px;
}

.user-status::before {
  content: "";
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--green);
  flex-shrink: 0;
}

.user-controls {
  display: flex;
  gap: 2px;
}

.user-control-btn {
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
}

.user-control-btn:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
  box-shadow: none;
}

.disconnected-state {
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-primary);
  grid-column: channels / -1;
}

.disconnected-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  text-align: center;
}

.disconnected-logo {
  font-size: 2.75rem;
  font-weight: 700;
  color: var(--text-normal);
  letter-spacing: -0.03em;
  user-select: none;
  position: relative;
  padding-bottom: 6px;
}

.disconnected-logo::after {
  content: "";
  position: absolute;
  left: 50%;
  bottom: 0;
  transform: translateX(-50%);
  width: 28px;
  height: 3px;
  background: var(--accent);
  border-radius: 2px;
}

.disconnected-content p {
  font-size: 0.875rem;
  color: var(--text-muted);
}

.disconnected-btn {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  width: auto;
  margin-top: 12px;
  padding: 10px 18px;
  font-size: 0.875rem;
  font-weight: 600;
  border-radius: 8px;
  background: var(--accent);
  color: var(--accent-fg);
  transition: background 0.15s;
}

.disconnected-btn:hover {
  background: var(--accent-hover);
}

/* ── Update banner ── */
.update-banner {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 12px;
  background: var(--accent);
  color: var(--accent-fg);
  font-size: 0.8125rem;
  font-weight: 500;
  flex-shrink: 0;
}

.update-banner.installing {
  background: var(--bg-tertiary);
  color: var(--text-normal);
  position: relative;
  overflow: hidden;
}

.update-banner-text {
  flex: 1;
}

.update-banner-text strong {
  font-weight: 700;
}

.update-banner-btn {
  width: auto;
  padding: 4px 12px;
  font-size: 0.75rem;
  font-weight: 600;
  background: oklch(0 0 0 / 0.20);
  color: inherit;
  border: none;
  border-radius: 6px;
  cursor: pointer;
}

.update-banner-btn:hover {
  background: oklch(0 0 0 / 0.32);
  box-shadow: none;
}

.update-banner-dismiss {
  width: 24px;
  height: 24px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  color: inherit;
  opacity: 0.8;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}

.update-banner-dismiss:hover {
  background: oklch(0 0 0 / 0.20);
  opacity: 1;
  box-shadow: none;
}

.update-progress-bar {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--bg-modifier-hover);
}

.update-progress-fill {
  height: 100%;
  background: var(--accent);
  transition: width 0.15s linear;
}

.spinner {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>
