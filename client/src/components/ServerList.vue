<template>
  <div class="server-list">
    <div
      v-for="server in store.savedServers"
      :key="server.id"
      class="server-icon"
      :class="{
        active: server.id === store.activeServerId,
        muted: getState(server.id)?.muted,
        disconnected: !getState(server.id)?.connected && !getState(server.id)?.muted,
      }"
      :title="server.name"
      @click="switchToServer(server.id)"
      @contextmenu.prevent="openMenu($event, server.id)"
    >
      {{ server.name[0]?.toUpperCase() }}
      <span class="unread-badge" v-if="getUnread(server.id) > 0">
        {{ getUnread(server.id) }}
      </span>
    </div>

    <div class="server-separator"></div>

    <div
      class="server-icon add-server"
      title="Ajouter un serveur"
      @click="store.showAddServerModal = true"
    >
      <Plus :size="20" />
    </div>

    <ContextMenu
      v-if="menu"
      :x="menu.x"
      :y="menu.y"
      :items="menu.items"
      @close="menu = null"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { Plus } from "lucide-vue-next";
import { store, switchToServer, muteServer, unmuteServer, removeServer } from "../store";
import ContextMenu, { type MenuItem } from "./ContextMenu.vue";

const menu = ref<{ x: number; y: number; items: MenuItem[] } | null>(null);

function getState(serverId: string) {
  return store.serverStates.get(serverId);
}

function getUnread(serverId: string): number {
  return getState(serverId)?.unreadCount ?? 0;
}

function openMenu(event: MouseEvent, serverId: string) {
  const state = getState(serverId);
  const items: MenuItem[] = [];

  if (state?.muted) {
    items.push({ label: "Reconnecter", action: () => unmuteServer(serverId) });
  } else if (state?.connected) {
    items.push({ label: "Se deconnecter", action: () => muteServer(serverId) });
  }

  items.push({
    label: "Supprimer",
    action: () => removeServer(serverId),
    danger: true,
  });

  menu.value = { x: event.clientX, y: event.clientY, items };
}
</script>
