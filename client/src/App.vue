<template>
  <div class="app-shell" :class="{ 'is-tauri': isTauri }">
    <TopBar />
    <div class="app-grid">
      <ServerList />
      <Sidebar v-if="state?.connected" />
      <div v-if="state?.connected" class="main-area">
        <ChatHeader />
        <div class="main-body">
          <VoiceView v-if="isVoice" />
          <ChatBody v-else />
          <UserList v-if="!isVoice" />
        </div>
      </div>
      <EmptyState v-else-if="!store.savedServers.length" class="main-area" />
      <div v-else class="main-area disconnected-state">
        <p>Selectionne un serveur a gauche pour te connecter.</p>
      </div>

      <div class="bottom-bar">
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
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Settings } from "lucide-vue-next";
import { store, connectAll, restoreNav, activeState, isActiveChannelVoice, resolveUserColor } from "./store";
import { setMentionResolver } from "./markdown";
import TopBar from "./components/TopBar.vue";
import ServerList from "./components/ServerList.vue";
import Sidebar from "./components/Sidebar.vue";
import ChatHeader from "./components/ChatHeader.vue";
import ChatBody from "./components/ChatBody.vue";
import VoiceView from "./components/VoiceView.vue";
import UserList from "./components/UserList.vue";
import EmptyState from "./components/EmptyState.vue";
import AddServerModal from "./components/AddServerModal.vue";
import SettingsModal from "./components/SettingsModal.vue";
import ServerSettingsModal from "./components/ServerSettingsModal.vue";
import ChannelSettingsModal from "./components/ChannelSettingsModal.vue";
import GroupSettingsModal from "./components/GroupSettingsModal.vue";
import VoiceBar from "./components/VoiceBar.vue";
import AudioControls from "./components/AudioControls.vue";

const state = computed(() => activeState());
const isVoice = computed(() => isActiveChannelVoice());
const isTauri = ref("__TAURI_INTERNALS__" in window);

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

  await connectAll();
  restoreNav();
});
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
  border-radius: 8px;
  border: 1px solid var(--border);
  overflow: hidden;
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
  color: var(--text-bright);
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
}

.disconnected-state p {
  color: var(--text-muted);
}
</style>
