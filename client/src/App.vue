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

      <!-- Footer spans server + channels columns -->
      <div class="bottom-bar">
        <VoiceBar />
        <div class="sidebar-footer">
          <div class="user-info">
            <div class="user-avatar">{{ state?.user?.display_name?.[0]?.toUpperCase() }}</div>
            <div class="user-meta">
              <span class="user-name">{{ state?.user?.display_name }}</span>
              <span class="user-status">En ligne</span>
            </div>
          </div>
          <div class="user-controls">
            <button class="user-control-btn" title="Parametres">
              <Settings :size="18" />
            </button>
          </div>
        </div>
      </div>
    </div>
    <AddServerModal v-if="store.showAddServerModal" />
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
import VoiceBar from "./components/VoiceBar.vue";

const state = computed(() => activeState());
const isVoice = computed(() => isActiveChannelVoice());

onMounted(() => {
  connectAll();
});
</script>
