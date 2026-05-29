<template>
  <div class="sidebar">
    <div class="sidebar-header" @click="state?.connected && (store.showServerSettingsModal = true)">
      <h2 v-if="server?.name">{{ server.name }}</h2>
      <div v-else class="skeleton skeleton-text" style="width: 120px; height: 16px;"></div>
      <ChevronDown v-if="state?.connected" :size="16" class="sidebar-header-icon" />
    </div>

    <!-- Skeleton while connecting -->
    <div v-if="!state?.connected" class="channel-list skeleton-channels">
      <div class="skeleton-group-title"><div class="skeleton skeleton-text" style="width: 70px; height: 10px;"></div></div>
      <div v-for="i in 5" :key="i" class="skeleton-channel">
        <div class="skeleton skeleton-icon"></div>
        <div class="skeleton skeleton-text" :style="{ width: (60 + Math.random() * 80) + 'px' }"></div>
      </div>
      <div class="skeleton-group-title" style="margin-top: 16px;"><div class="skeleton skeleton-text" style="width: 90px; height: 10px;"></div></div>
      <div v-for="i in 3" :key="'v'+i" class="skeleton-channel">
        <div class="skeleton skeleton-icon"></div>
        <div class="skeleton skeleton-text" :style="{ width: (60 + Math.random() * 80) + 'px' }"></div>
      </div>
    </div>

    <div v-else class="sidebar-body">
      <div class="sidebar-tabs-wrap">
        <BaseTabs
          v-model="activeTab"
          :items="tabItems"
          variant="stretched"
        />
      </div>

      <!-- Onglet Channels -->
      <div
        v-if="activeTab === 'channels'"
        class="channel-list"
        @contextmenu.prevent="onContextMenu"
      >
        <div v-if="ungrouped.length || canManage" class="channel-group-title ungrouped-header">
          <span>Channels</span>
          <button v-if="canManage" class="group-add-btn" @click.stop="onAddClick($event)" title="Creer un channel">
            <Plus :size="14" />
          </button>
        </div>

        <VueDraggable
          v-model="ungrouped"
          group="channels"
          :disabled="!canManage"
          data-group-id="ungrouped"
          @start="onChannelDragStart"
          @end="onChannelEnd"
        >
          <div v-for="ch in ungrouped" :key="ch.id" :data-channel-id="ch.id">
            <ChannelItem :channel="ch" @contextmenu="onChannelContextMenu(ch, $event)">
              <template #actions v-if="canManage">
                <button class="channel-gear" @click.stop="openChannelSettings(ch.id)" title="Modifier">
                  <Settings :size="14" />
                </button>
              </template>
            </ChannelItem>
          </div>
        </VueDraggable>

        <VueDraggable
          v-model="localGroups"
          group="groups"
          :disabled="!canManage"
          handle=".channel-group-title"
          @end="onGroupEnd"
        >
          <div v-for="group in localGroups" :key="group.id" class="channel-group">
            <div
              class="channel-group-title"
              :class="{ 'can-drag': canManage }"
              @click="toggleGroup(group.id)"
              @contextmenu.prevent.stop="onGroupContextMenu(group, $event)"
            >
              <ChevronRight :size="12" class="group-arrow" :class="{ expanded: !collapsed.has(group.id) }" />
              <span>{{ group.name }}</span>
              <button v-if="canManage" class="group-add-btn" @click.stop="onAddChannelClick($event)" title="Creer un channel">
                <Plus :size="14" />
              </button>
            </div>
            <VueDraggable
              v-if="!collapsed.has(group.id)"
              v-model="groupChannels[group.id]"
              group="channels"
              :disabled="!canManage"
              :data-group-id="group.id"
              @start="onChannelDragStart"
              @end="onChannelEnd"
            >
              <div v-for="ch in groupChannels[group.id]" :key="ch.id" :data-channel-id="ch.id">
                <ChannelItem :channel="ch" @contextmenu="onChannelContextMenu(ch, $event)">
                  <template #actions v-if="canManage">
                    <button class="channel-gear" @click.stop="openChannelSettings(ch.id)" title="Modifier">
                      <Settings :size="14" />
                    </button>
                  </template>
                </ChannelItem>
              </div>
            </VueDraggable>
          </div>
        </VueDraggable>
      </div>

      <!-- Onglet Messages privés -->
      <div v-else class="channel-list dm-tab">
        <div class="channel-group-title ungrouped-header">
          <span>Conversations</span>
          <button class="group-add-btn" @click.stop="openNewDmModal" title="Nouveau message prive">
            <Plus :size="14" />
          </button>
        </div>
        <div class="dm-list">
          <div v-if="!dmPeers.length" class="dm-empty-hint">
            Aucune conversation pour l'instant.
            <button class="dm-empty-cta" @click="openNewDmModal">Commencer un DM</button>
          </div>
          <div
            v-for="peerId in dmPeers"
            :key="peerId"
            class="dm-item"
            :class="{ active: state?.activeDmUserId === peerId }"
            @click="onOpenDm(peerId)"
          >
            <div class="dm-item-avatar">
              <img v-if="dmAvatar(peerId)" :src="dmAvatar(peerId)!" />
              <span v-else>{{ dmInitial(peerId) }}</span>
            </div>
            <span class="dm-item-name">{{ dmName(peerId) }}</span>
            <span v-if="(state?.dmUnread.get(peerId) ?? 0) > 0" class="dm-item-badge">
              {{ state?.dmUnread.get(peerId) }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <ContextMenu
      v-if="ctxMenu"
      :x="ctxMenu.x"
      :y="ctxMenu.y"
      :items="ctxMenu.items"
      @close="ctxMenu = null"
    />

    <ModalSmall v-if="confirmDeleteChannel" title="Supprimer le channel" @close="confirmDeleteChannel = null">
      <p class="confirm-text">Es-tu sur de vouloir supprimer <strong>#{{ confirmDeleteChannel.name }}</strong> ? Cette action est irreversible.</p>
      <div class="modal-actions">
        <button class="btn-cancel" @click="confirmDeleteChannel = null">Annuler</button>
        <button class="btn-danger" @click="doDeleteChannel">Supprimer</button>
      </div>
    </ModalSmall>

    <NewDmModal v-if="showNewDmModal" @close="showNewDmModal = false" />
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { ChevronDown, ChevronRight, Plus, Settings, Hash, Volume2, FolderPlus, Pencil, Trash2, BellOff, Bell, BellMinus, MessageCircle } from "lucide-vue-next";
import { VueDraggable } from "vue-draggable-plus";
import {
  store,
  activeState,
  activeServer,
  setNotificationPref,
  removeNotificationPref,
  openDmWith,
  resolveAvatarUrl,
} from "../store";
import { api } from "../api";
import * as perms from "../permissions";
import ChannelItem from "./ChannelItem.vue";
import ContextMenu, { type MenuItem } from "./ui/ContextMenu.vue";
import ModalSmall from "./ui/ModalSmall.vue";
import BaseTabs, { type TabItem } from "./ui/BaseTabs.vue";
import NewDmModal from "./NewDmModal.vue";
import type { Channel, ChannelGroup } from "../api";

const state = computed(() => activeState());
const server = computed(() => activeServer());

// ── Tabs ──
// L'onglet actif vit dans l'état du serveur (non persisté) pour qu'App.vue puisse router la vue.
const activeTab = computed<"channels" | "dms">({
  get: () => state.value?.activeTab ?? "channels",
  set: (v) => {
    const st = state.value;
    if (!st) return;
    st.activeTab = v;
    // En passant sur l'onglet DMs : si aucun DM n'est déjà ouvert, ouvrir automatiquement
    // le dernier utilisé (mémorisé via activeDmUserId) ou sinon le premier de la liste.
    if (v === "dms" && st.activeDmUserId == null) {
      const first = st.dmConversations[0];
      if (first != null) openDmWith(first);
    }
  },
});

const totalDmUnread = computed(() => {
  let sum = 0;
  for (const n of state.value?.dmUnread.values() ?? []) sum += n;
  return sum;
});

const tabItems = computed<TabItem[]>(() => [
  { id: "channels", label: "Channels", icon: Hash },
  { id: "dms", label: "DMs", icon: MessageCircle, count: totalDmUnread.value },
]);

// ── DMs ──
const dmPeers = computed(() => state.value?.dmConversations ?? []);
function dmName(peerId: number): string {
  return state.value?.users.get(peerId)?.display_name ?? `User #${peerId}`;
}
function dmInitial(peerId: number): string {
  return (state.value?.users.get(peerId)?.display_name ?? "?")[0]?.toUpperCase() ?? "?";
}
function dmAvatar(peerId: number): string | null {
  return resolveAvatarUrl(peerId);
}
async function onOpenDm(peerId: number) {
  await openDmWith(peerId);
}
const showNewDmModal = ref(false);
function openNewDmModal() {
  showNewDmModal.value = true;
}

const collapsed = reactive(new Set<number>());

const canManage = computed(() =>
  perms.has(state.value?.permissions ?? 0, perms.MANAGE_CHANNELS)
);

// ── Local drag-friendly state ──
const ungrouped = ref<Channel[]>([]);
const groupChannels = ref<Record<number, Channel[]>>({});
const localGroups = ref<ChannelGroup[]>([]);

function canViewChannel(channelId: number): boolean {
  const st = state.value;
  if (!st?.user) return true;
  // Admins see everything
  if (perms.has(st.permissions, perms.ADMINISTRATOR)) return true;
  const userRoleIds = st.userRoles.get(st.user.id) ?? [];
  const channelOws = st.channelOverwrites.filter(o => o.channel_id === channelId);
  if (channelOws.length === 0) return perms.has(st.permissions, perms.VIEW_CHANNELS);
  const computed = perms.computeChannel(userRoleIds, st.roles, channelOws);
  return perms.has(computed, perms.VIEW_CHANNELS);
}

watch(
  [() => state.value?.channels, () => state.value?.groups, () => state.value?.channelOverwrites, () => state.value?.userRoles, () => state.value?.roles, () => state.value?.voiceChannelId],
  () => {
    const chs = (state.value?.channels ?? []).filter(c =>
      canManage.value || canViewChannel(c.id) || c.id === state.value?.voiceChannelId
    );
    const grs = state.value?.groups ?? [];
    localGroups.value = [...grs];
    ungrouped.value = chs.filter((c) => !c.group_id);
    const map: Record<number, Channel[]> = {};
    for (const g of grs) {
      map[g.id] = chs.filter((c) => c.group_id === g.id);
    }
    groupChannels.value = map;
  },
  { immediate: true, deep: true }
);

async function quickCreateChannel(kind: "text" | "voice") {
  const s = activeServer();
  const st = activeState();
  if (!s || !st) return;
  const name = kind === "text" ? "nouveau-channel" : "Nouveau vocal";
  const ch = await api.createChannel(s.url, s.token, name, kind);
  // L'event WS ChannelCreate peut être arrivé avant la réponse REST — dédup pour éviter un doublon.
  if (!st.channels.find((c) => c.id === ch.id)) st.channels.push(ch);
  store.channelSettingsId = ch.id;
}

async function quickCreateGroup() {
  const s = activeServer();
  const st = activeState();
  if (!s || !st) return;
  const group = await api.createGroup(s.url, s.token, "Nouveau groupe");
  if (!st.groups.find((g) => g.id === group.id)) st.groups.push(group);
  store.groupSettingsId = group.id;
}

function onAddChannelClick(e: MouseEvent) {
  ctxMenu.value = {
    x: e.clientX,
    y: e.clientY,
    items: [
      { label: "Channel texte", icon: Hash, action: () => quickCreateChannel("text") },
      { label: "Channel vocal", icon: Volume2, action: () => quickCreateChannel("voice") },
    ],
  };
}

function onAddClick(e: MouseEvent) {
  ctxMenu.value = {
    x: e.clientX,
    y: e.clientY,
    items: [
      { label: "Channel texte", icon: Hash, action: () => quickCreateChannel("text") },
      { label: "Channel vocal", icon: Volume2, action: () => quickCreateChannel("voice") },
      { label: "Groupe", icon: FolderPlus, action: () => quickCreateGroup() },
    ],
  };
}

function openChannelSettings(channelId: number) {
  store.channelSettingsId = channelId;
}

function onChannelDragStart() {
  collapsed.clear();
}

async function onChannelEnd(evt: { from: HTMLElement; to: HTMLElement; item: HTMLElement }) {
  const fromRaw = evt.from.dataset.groupId;
  const toRaw = evt.to.dataset.groupId;
  if (!fromRaw || !toRaw) return;

  const fromGroupId = fromRaw === "ungrouped" ? null : Number(fromRaw);
  const toGroupId = toRaw === "ungrouped" ? null : Number(toRaw);
  const movedId = Number(evt.item.dataset.channelId);

  const toList = toGroupId === null ? ungrouped.value : (groupChannels.value[toGroupId] ?? []);
  const movedCh = toList.find((c) => c.id === movedId);
  if (movedCh) movedCh.group_id = toGroupId;

  const allChannels: Channel[] = [
    ...ungrouped.value,
    ...localGroups.value.flatMap((g) => groupChannels.value[g.id] ?? []),
  ];

  const s = activeServer();
  const st = activeState();
  if (!s || !st) return;

  st.channels = allChannels;

  if (fromGroupId !== toGroupId) {
    await api.moveChannel(s.url, s.token, movedId, toGroupId);
  }
  await api.reorderChannels(s.url, s.token, allChannels.map((c) => c.id));
}

async function onGroupEnd() {
  const s = activeServer();
  const st = activeState();
  if (!s || !st) return;

  st.groups = [...localGroups.value];
  await api.reorderGroups(s.url, s.token, localGroups.value.map((g) => g.id));
}

// ── Delete confirmation ──
const confirmDeleteChannel = ref<Channel | null>(null);

async function doDeleteChannel() {
  const s = activeServer();
  const st = activeState();
  const ch = confirmDeleteChannel.value;
  if (!s || !st || !ch) return;
  await api.deleteChannel(s.url, s.token, ch.id);
  st.channels = st.channels.filter((c) => c.id !== ch.id);
  confirmDeleteChannel.value = null;
}

// ── Context menu ──
const ctxMenu = ref<{ x: number; y: number; items: MenuItem[] } | null>(null);


function onGroupContextMenu(group: ChannelGroup, e: MouseEvent) {
  if (!canManage.value) return;

  const s = activeServer();
  const st = activeState();

  const items: MenuItem[] = [
    { label: "Modifier le groupe", icon: Pencil, action: () => { store.groupSettingsId = group.id; } },
    {
      label: "Supprimer le groupe",
      icon: Trash2,
      danger: true,
      action: async () => {
        if (!s || !st) return;
        await api.deleteGroup(s.url, s.token, group.id);
        st.groups = st.groups.filter((g) => g.id !== group.id);
        st.channels = st.channels.map((c) =>
          c.group_id === group.id ? { ...c, group_id: null } : c
        );
      },
    },
  ];

  ctxMenu.value = { x: e.clientX, y: e.clientY, items };
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

function buildDurationItems(scope: "channel" | "server", targetId: number, level: "mentions" | "nothing"): MenuItem[] {
  return muteDurations.map((d) => ({
    label: d.label,
    action: () => setNotificationPref(scope, targetId, level, getMuteUntil(d.value)),
  }));
}

function buildMuteItems(scope: "channel" | "server", targetId: number, x: number, y: number): MenuItem[] {
  const st = activeState();
  if (!st) return [];
  const currentPref = st.notificationPrefs.find(
    (p) => p.scope === scope && p.target_id === targetId
  );
  const currentLevel = currentPref?.level ?? "all";

  const items: MenuItem[] = [];

  if (currentLevel !== "all") {
    items.push({
      label: "Reactiver les notifications",
      icon: Bell,
      action: () => removeNotificationPref(scope, targetId),
    });
  }

  if (currentLevel !== "mentions") {
    items.push({
      label: "Mentions uniquement",
      icon: BellMinus,
      keepOpen: true,
      action: () => {
        ctxMenu.value = { x, y, items: buildDurationItems(scope, targetId, "mentions") };
      },
    });
  }

  if (currentLevel !== "nothing") {
    items.push({
      label: "Aucune notification",
      icon: BellOff,
      keepOpen: true,
      action: () => {
        ctxMenu.value = { x, y, items: buildDurationItems(scope, targetId, "nothing") };
      },
    });
  }

  return items;
}

function onChannelContextMenu(channel: Channel, e: MouseEvent) {
  const items: MenuItem[] = buildMuteItems("channel", channel.id, e.clientX, e.clientY);

  // Admin-only items
  if (canManage.value) {
    items.push({ label: "Modifier le channel", icon: Pencil, action: () => openChannelSettings(channel.id) });
    items.push({
      label: "Supprimer le channel",
      icon: Trash2,
      danger: true,
      action: () => { confirmDeleteChannel.value = channel; },
    });
  }

  ctxMenu.value = { x: e.clientX, y: e.clientY, items };
}

function onContextMenu(e: MouseEvent) {
  if (!canManage.value) return;

  const target = (e.target as HTMLElement).closest("[data-channel-id]");
  if (target) return;

  const items: MenuItem[] = [
    { label: "Creer un channel texte", icon: Hash, action: () => quickCreateChannel("text") },
    { label: "Creer un channel vocal", icon: Volume2, action: () => quickCreateChannel("voice") },
    { label: "Creer un groupe", icon: FolderPlus, action: () => quickCreateGroup() },
  ];

  ctxMenu.value = { x: e.clientX, y: e.clientY, items };
}

function toggleGroup(groupId: number) {
  if (collapsed.has(groupId)) {
    collapsed.delete(groupId);
  } else {
    collapsed.add(groupId);
  }
}
</script>

<style scoped>
.sidebar {
  grid-area: channels;
  background: var(--bg-secondary);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-top: 1px solid var(--border-soft);
  border-left: 1px solid var(--border-soft);
  border-top-left-radius: 14px;
}

.sidebar-header {
  padding: 0 16px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--border-soft);
  cursor: pointer;
  transition: background 0.12s;
}

.sidebar-header:hover {
  background: var(--bg-modifier-hover);
}

.sidebar-header h2 {
  font-size: 0.9375rem;
  font-weight: 600;
  color: var(--header-primary);
  letter-spacing: -0.005em;
}

.sidebar-header-icon {
  color: var(--text-faint);
}

.sidebar-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.sidebar-tabs-wrap {
  padding: 8px 10px 4px;
  flex-shrink: 0;
}

.channel-list {
  padding: 8px 0;
  overflow-y: auto;
  flex: 1;
}
.channel-list.dm-tab { padding: 4px 0; }

.channel-group {
  margin-top: 8px;
}

.channel-group-title {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 8px 8px 6px 10px;
  font-size: 0.65rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-faint);
  cursor: pointer;
  user-select: none;
  transition: color 0.12s;
}

.channel-group-title:hover {
  color: var(--text-muted);
}

.ungrouped-header {
  cursor: default;
}

.group-add-btn {
  width: 18px;
  height: 18px;
  padding: 0;
  margin: 0 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  background: transparent;
  color: var(--text-faint);
  border: none;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.1s, color 0.1s;
}

.channel-group-title:hover .group-add-btn,
.ungrouped-header:hover .group-add-btn {
  opacity: 1;
}

.group-add-btn:hover {
  color: var(--text-normal);
  background: var(--bg-modifier-hover);
  box-shadow: none;
}

.group-arrow {
  transition: transform 0.15s;
  flex-shrink: 0;
}

.group-arrow.expanded {
  transform: rotate(90deg);
}

/* ── DM list ── */
.dm-list {
  display: flex;
  flex-direction: column;
  padding: 0 4px;
  margin-bottom: 12px;
}
.dm-empty-hint {
  padding: 20px 14px;
  font-size: 0.8125rem;
  color: var(--text-faint);
  text-align: center;
  display: flex;
  flex-direction: column;
  gap: 10px;
  align-items: center;
}
.dm-empty-cta {
  width: auto;
  margin: 0;
  padding: 6px 14px;
  font-size: 0.75rem;
  font-weight: 600;
  background: var(--accent);
  color: var(--accent-fg);
  border: none;
  border-radius: 6px;
  cursor: pointer;
}
.dm-empty-cta:hover { background: var(--accent-hover); box-shadow: none; }
.dm-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 4px;
  cursor: pointer;
  color: var(--text-muted);
  font-size: 0.875rem;
}
.dm-item:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
}
.dm-item.active {
  background: var(--bg-modifier-selected);
  color: var(--text-normal);
}
.dm-item-avatar {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: var(--accent);
  color: var(--accent-fg);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.625rem;
  font-weight: 600;
  overflow: hidden;
  flex-shrink: 0;
}
.dm-item-avatar img { width: 100%; height: 100%; object-fit: cover; }
.dm-item-name {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.dm-item-badge {
  background: var(--accent);
  color: var(--accent-fg);
  font-size: 0.6875rem;
  font-family: var(--font-mono);
  padding: 1px 6px;
  border-radius: 10px;
  font-weight: 600;
}

.channel-gear {
  width: 24px;
  height: 24px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  background: transparent;
  color: var(--text-faint);
  cursor: pointer;
}
.channel-gear:hover { color: var(--text-normal); background: var(--bg-modifier-hover); box-shadow: none; }

.channel-group-title.can-drag { cursor: grab; }
.channel-group-title.can-drag:active { cursor: grabbing; }

/* ── Skeleton ── */
.skeleton-channels {
  padding: 8px 0;
  flex: 1;
}

.skeleton-group-title {
  padding: 4px 10px;
  margin-bottom: 4px;
}

.skeleton-channel {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
}

.skeleton-icon {
  width: 18px;
  height: 18px;
  border-radius: 4px;
}

.skeleton {
  background: var(--bg-modifier-hover);
  border-radius: 4px;
  animation: skeleton-pulse 1.5s ease-in-out infinite;
}

.skeleton-text {
  height: 14px;
}

@keyframes skeleton-pulse {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 0.8; }
}
</style>
