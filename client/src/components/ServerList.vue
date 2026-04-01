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
      <img v-if="server.iconUrl" :src="`${server.url}${server.iconUrl}`" class="server-icon-img" />
      <span v-else>{{ server.name[0]?.toUpperCase() }}</span>
      <span
        class="status-dot"
        :class="{
          connected: getState(server.id)?.connected,
          muted: getState(server.id)?.muted,
        }"
      ></span>
      <span class="unread-badge" v-if="getUnread(server.id) > 0">
        {{ getUnread(server.id) }}
      </span>
      <span class="voice-indicator" v-if="getState(server.id)?.voiceChannelId">
        <Phone :size="8" fill="currentColor" />
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
import { Plus, Plug, Unplug, Trash2, ToggleLeft, ToggleRight, Phone } from "lucide-vue-next";
import { store, switchToServer, muteServer, unmuteServer, removeServer, persistServers } from "../store";
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
  const server = store.savedServers.find((s) => s.id === serverId);
  const items: MenuItem[] = [];

  if (state?.muted) {
    items.push({ label: "Reconnecter", icon: Plug, action: () => unmuteServer(serverId) });
  } else if (state?.connected) {
    items.push({ label: "Se deconnecter", icon: Unplug, action: () => muteServer(serverId) });
  }

  const auto = server?.autoConnect !== false;
  items.push({
    label: auto ? "Auto-connexion: on" : "Auto-connexion: off",
    icon: auto ? ToggleRight : ToggleLeft,
    keepOpen: true,
    action: () => {
      if (server) {
        server.autoConnect = !auto;
        persistServers();
        openMenu(event, serverId);
      }
    },
  });

  items.push({
    label: "Supprimer",
    icon: Trash2,
    action: () => removeServer(serverId),
    danger: true,
  });

  menu.value = { x: event.clientX, y: event.clientY, items };
}
</script>

<style scoped>
.server-list {
  grid-area: servers;
  background: var(--bg-secondary);
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 12px 0;
  gap: 8px;
  overflow-y: auto;
}

.server-icon {
  position: relative;
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: var(--bg-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 1.125rem;
  cursor: pointer;
  color: var(--text-muted);
  transition: border-radius 0.15s, background 0.15s, color 0.15s;
  user-select: none;
}

.server-icon-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: inherit;
}

.server-icon::before {
  content: '';
  position: absolute;
  left: -16px;
  width: 4px;
  border-radius: 0 4px 4px 0;
  background: var(--header-primary);
  transition: height 0.15s;
  height: 0;
}

.server-icon:hover::before { height: 20px; }
.server-icon.active::before { height: 40px; }

.server-icon:hover {
  border-radius: 16px;
  background: var(--accent);
  color: #fff;
}

.server-icon.active {
  border-radius: 16px;
  background: var(--accent);
  color: #fff;
}

.server-icon.add-server {
  background: var(--bg-primary);
  color: var(--green);
  font-size: 1.5rem;
}

.server-icon.add-server:hover {
  background: var(--green);
  color: #fff;
  border-radius: 16px;
}

.server-icon.muted { opacity: 0.3; }
.server-icon.disconnected { opacity: 0.5; }

.status-dot {
  position: absolute;
  bottom: 0;
  left: 0;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--danger);
  border: 2px solid var(--bg-secondary);
}

.status-dot.connected {
  background: var(--green);
}

.status-dot.muted {
  background: var(--text-muted);
}

.unread-badge {
  position: absolute;
  bottom: -2px;
  right: -2px;
  background: var(--danger);
  color: #fff;
  font-size: 0.625rem;
  font-weight: 700;
  min-width: 16px;
  height: 16px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 4px;
  border: 3px solid var(--bg-secondary);
}

.voice-indicator {
  position: absolute;
  top: -2px;
  right: -2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--green);
  color: #000;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px solid var(--bg-secondary);
}

.server-separator {
  width: 32px;
  height: 2px;
  background: var(--border);
  border-radius: 1px;
  margin: 2px 0;
}
</style>
