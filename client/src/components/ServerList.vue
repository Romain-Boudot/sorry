<template>
  <div class="server-list">
    <VueDraggable
      v-model="store.savedServers"
      :animation="150"
      handle=".server-icon"
      @end="persistServers()"
      class="server-drag-area"
    >
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
        <span class="unread-badge mention" v-if="getMentions(server.id) > 0"></span>
        <span class="unread-badge" v-else-if="getUnread(server.id) > 0"></span>
        <span class="voice-indicator" v-if="getState(server.id)?.voiceChannelId">
          <Phone :size="8" fill="currentColor" />
        </span>
        <span class="notif-muted-indicator" v-if="getNotifLevel(server.id) !== 'all'" :title="getNotifLevel(server.id) === 'nothing' ? 'Notifications desactivees' : 'Mentions uniquement'">
          <BellOff v-if="getNotifLevel(server.id) === 'nothing'" :size="10" />
          <BellMinus v-else :size="10" />
        </span>
      </div>
    </VueDraggable>

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
import { VueDraggable } from "vue-draggable-plus";
import { Plus, Plug, Unplug, Trash2, ToggleLeft, ToggleRight, Phone, Bell, BellMinus, BellOff } from "lucide-vue-next";
import { store, switchToServer, muteServer, unmuteServer, removeServer, persistServers, setNotificationPref, removeNotificationPref, activeState } from "../store";
import ContextMenu, { type MenuItem } from "./ui/ContextMenu.vue";

const menu = ref<{ x: number; y: number; items: MenuItem[] } | null>(null);

function getState(serverId: string) {
  return store.serverStates.get(serverId);
}

function getUnread(serverId: string): number {
  return getState(serverId)?.unreadCount ?? 0;
}

function getNotifLevel(serverId: string): string {
  const st = getState(serverId);
  if (!st) return "all";
  const now = new Date().toISOString();
  const pref = st.notificationPrefs.find((p) => p.scope === "server" && p.target_id === 0);
  if (pref && (!pref.mute_until || pref.mute_until >= now)) return pref.level;
  return "all";
}

function getMentions(serverId: string): number {
  const st = getState(serverId);
  if (!st) return 0;
  let total = 0;
  for (const count of st.channelMentions.values()) total += count;
  return total;
}

function getMuteUntil(duration: string): string | null {
  if (duration === "forever") return null;
  const hours: Record<string, number> = { "1h": 1, "5h": 5, "12h": 12, "1d": 24, "7d": 168 };
  const h = hours[duration] ?? 1;
  return new Date(Date.now() + h * 3600_000).toISOString();
}

const muteDurations = [
  { label: "1 heure", value: "1h" },
  { label: "5 heures", value: "5h" },
  { label: "12 heures", value: "12h" },
  { label: "1 jour", value: "1d" },
  { label: "7 jours", value: "7d" },
  { label: "Jusqu'a modification", value: "forever" },
];

function buildServerNotifItems(event: MouseEvent, serverId: string): MenuItem[] {
  const prevActive = store.activeServerId;
  const st = getState(serverId);
  if (!st) return [];
  const currentPref = st.notificationPrefs.find((p) => p.scope === "server" && p.target_id === 0);
  const currentLevel = currentPref?.level ?? "all";

  const items: MenuItem[] = [];

  if (currentLevel !== "all") {
    items.push({
      label: "Reactiver les notifications",
      icon: Bell,
      action: () => { store.activeServerId = serverId; removeNotificationPref("server", 0); store.activeServerId = prevActive; },
    });
  }

  if (currentLevel !== "mentions") {
    items.push({
      label: "Mentions uniquement",
      icon: BellMinus,
      keepOpen: true,
      action: () => {
        menu.value = { x: event.clientX, y: event.clientY, items: muteDurations.map((d) => ({
          label: d.label,
          action: () => { store.activeServerId = serverId; setNotificationPref("server", 0, "mentions", getMuteUntil(d.value)); store.activeServerId = prevActive; },
        })) };
      },
    });
  }

  if (currentLevel !== "nothing") {
    items.push({
      label: "Aucune notification",
      icon: BellOff,
      keepOpen: true,
      action: () => {
        menu.value = { x: event.clientX, y: event.clientY, items: muteDurations.map((d) => ({
          label: d.label,
          action: () => { store.activeServerId = serverId; setNotificationPref("server", 0, "nothing", getMuteUntil(d.value)); store.activeServerId = prevActive; },
        })) };
      },
    });
  }

  return items;
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

  // Notification settings
  if (state?.connected) {
    items.push(...buildServerNotifItems(event, serverId));
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

.server-drag-area {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
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
  width: 3px;
  border-radius: 0 3px 3px 0;
  background: var(--accent);
  transition: height 0.15s;
  height: 0;
}

.server-icon:hover::before { height: 20px; }
.server-icon.active::before { height: 40px; }

.server-icon:hover {
  border-radius: 16px;
  background: var(--accent);
  color: var(--accent-fg);
}

.server-icon.active {
  border-radius: 16px;
  background: var(--accent);
  color: var(--accent-fg);
}

.server-icon.add-server {
  background: var(--bg-primary);
  color: var(--accent);
  font-size: 1.5rem;
  border: 1px dashed var(--accent-line);
}

.server-icon.add-server:hover {
  background: var(--accent);
  color: var(--accent-fg);
  border-color: transparent;
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
  bottom: -1px;
  right: -1px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--accent);
  border: 2px solid var(--bg-secondary);
}

.unread-badge.mention {
  background: var(--danger);
}

.voice-indicator {
  position: absolute;
  top: -2px;
  right: -2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--green);
  color: oklch(0.15 0.05 145);
  display: flex;
  align-items: center;
  justify-content: center;
  border: 2px solid var(--bg-secondary);
}

.notif-muted-indicator {
  position: absolute;
  top: -2px;
  left: -2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--bg-tertiary);
  color: var(--text-faint);
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
