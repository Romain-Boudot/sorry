<template>
  <div class="app-shell">
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
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from "vue";
import { Settings } from "lucide-vue-next";
import { store, connectAll, activeState, isActiveChannelVoice } from "./store";
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
import VoiceBar from "./components/VoiceBar.vue";

const state = computed(() => activeState());
const isVoice = computed(() => isActiveChannelVoice());

onMounted(() => {
  connectAll();
});
</script>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
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
  color: #fff;
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
  border-radius: 4px;
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
