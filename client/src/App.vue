<template>
  <div class="app-layout">
    <ServerList />

    <template v-if="state?.connected">
      <Sidebar />
      <Chat />
      <UserList />
    </template>

    <EmptyState v-else-if="!store.savedServers.length" />

    <div v-else class="disconnected-state">
      <p>Sélectionne un serveur à gauche pour te connecter.</p>
    </div>

    <AddServerModal v-if="store.showAddServerModal" />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from "vue";
import { store, connectAll, activeState } from "./store";
import ServerList from "./components/ServerList.vue";
import Sidebar from "./components/Sidebar.vue";
import Chat from "./components/Chat.vue";
import UserList from "./components/UserList.vue";
import EmptyState from "./components/EmptyState.vue";
import AddServerModal from "./components/AddServerModal.vue";

const state = computed(() => activeState());

onMounted(() => {
  connectAll();
});
</script>
