<template>
  <div class="app-layout">
    <div class="left-panel">
      <div class="left-panel-top">
        <ServerList />
        <Sidebar v-if="state?.connected" />
      </div>
      <VoiceBar />
    </div>

    <div v-if="state?.connected" class="main-area">
      <ChatHeader />
      <div class="main-body">
        <ChatBody />
        <UserList />
      </div>
    </div>

    <EmptyState v-else-if="!store.savedServers.length" />

    <div v-else class="disconnected-state">
      <p>Selectionne un serveur a gauche pour te connecter.</p>
    </div>

    <AddServerModal v-if="store.showAddServerModal" />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from "vue";
import { store, connectAll, activeState } from "./store";
import ServerList from "./components/ServerList.vue";
import Sidebar from "./components/Sidebar.vue";
import ChatHeader from "./components/ChatHeader.vue";
import ChatBody from "./components/ChatBody.vue";
import UserList from "./components/UserList.vue";
import EmptyState from "./components/EmptyState.vue";
import AddServerModal from "./components/AddServerModal.vue";
import VoiceBar from "./components/VoiceBar.vue";

const state = computed(() => activeState());

onMounted(() => {
  connectAll();
});
</script>
