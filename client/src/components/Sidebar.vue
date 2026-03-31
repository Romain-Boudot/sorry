<template>
  <div class="sidebar">
    <div class="sidebar-header" @click="store.showServerSettingsModal = true">
      <h2>{{ server?.name }}</h2>
      <ChevronDown :size="16" class="sidebar-header-icon" />
    </div>

    <div class="channel-list" @contextmenu.prevent="onContextMenu">
      <!-- Header channels sans groupe -->
      <div v-if="ungrouped.length || canManage" class="channel-group-title ungrouped-header">
        <span>Channels</span>
        <button v-if="canManage" class="group-add-btn" @click.stop="onAddClick($event)" title="Creer un channel">
          <Plus :size="14" />
        </button>
      </div>

      <!-- Channels sans groupe -->
      <VueDraggable
        v-model="ungrouped"
        group="channels"
        :disabled="!canManage"
        data-group-id="ungrouped"
        @start="onChannelDragStart"
        @end="onChannelEnd"
      >
        <div v-for="ch in ungrouped" :key="ch.id" :data-channel-id="ch.id" class="channel-wrapper">
          <ChannelItem :channel="ch" @contextmenu="onChannelContextMenu(ch, $event)" />
          <button v-if="canManage" class="channel-gear" @click.stop="openChannelSettings(ch.id)" title="Modifier">
            <Settings :size="14" />
          </button>
        </div>
      </VueDraggable>

      <!-- Groupes -->
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
            <div v-for="ch in groupChannels[group.id]" :key="ch.id" :data-channel-id="ch.id" class="channel-wrapper">
              <ChannelItem :channel="ch" @contextmenu="onChannelContextMenu(ch, $event)" />
              <button v-if="canManage" class="channel-gear" @click.stop="openChannelSettings(ch.id)" title="Modifier">
                <Settings :size="14" />
              </button>
            </div>
          </VueDraggable>
        </div>
      </VueDraggable>
    </div>

    <ContextMenu
      v-if="ctxMenu"
      :x="ctxMenu.x"
      :y="ctxMenu.y"
      :items="ctxMenu.items"
      @close="ctxMenu = null"
    />

  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { ChevronDown, ChevronRight, Plus, Settings, Hash, Volume2, FolderPlus, Pencil, Trash2 } from "lucide-vue-next";
import { VueDraggable } from "vue-draggable-plus";
import {
  store,
  activeState,
  activeServer,
} from "../store";
import { api } from "../api";
import * as perms from "../permissions";
import ChannelItem from "./ChannelItem.vue";
import ContextMenu, { type MenuItem } from "./ContextMenu.vue";
import type { Channel, ChannelGroup } from "../api";

const state = computed(() => activeState());
const server = computed(() => activeServer());

const collapsed = reactive(new Set<number>());

const canManage = computed(() =>
  perms.has(state.value?.permissions ?? 0, perms.MANAGE_CHANNELS)
);

// ── Local drag-friendly state ──
const ungrouped = ref<Channel[]>([]);
const groupChannels = ref<Record<number, Channel[]>>({});
const localGroups = ref<ChannelGroup[]>([]);

watch(
  [() => state.value?.channels, () => state.value?.groups],
  () => {
    const chs = state.value?.channels ?? [];
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
  st.channels.push(ch);
  store.channelSettingsId = ch.id;
}

async function quickCreateGroup() {
  const s = activeServer();
  const st = activeState();
  if (!s || !st) return;
  const group = await api.createGroup(s.url, s.token, "Nouveau groupe");
  st.groups.push(group);
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

function onChannelContextMenu(channel: Channel, e: MouseEvent) {
  if (!canManage.value) return;

  const s = activeServer();
  const st = activeState();

  const items: MenuItem[] = [
    { label: "Modifier le channel", icon: Pencil, action: () => openChannelSettings(channel.id) },
    {
      label: "Supprimer le channel",
      icon: Trash2,
      danger: true,
      action: async () => {
        if (!s || !st) return;
        await api.deleteChannel(s.url, s.token, channel.id);
        st.channels = st.channels.filter((c) => c.id !== channel.id);
      },
    },
  ];

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
  border-top: 1px solid var(--border);
  border-left: 1px solid var(--border);
  border-top-left-radius: 14px;
}

.sidebar-header {
  padding: 0 16px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  transition: background 0.1s;
}

.sidebar-header:hover {
  background: var(--bg-modifier-hover);
}

.sidebar-header h2 {
  font-size: 0.9375rem;
  font-weight: 600;
  color: var(--header-primary);
}

.sidebar-header-icon {
  color: var(--text-faint);
}

.channel-list {
  padding: 8px 0;
  overflow-y: auto;
  flex: 1;
}

.channel-group {
  margin-top: 8px;
}

.channel-group-title {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 4px 8px 4px 10px;
  font-size: 0.6875rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-faint);
  cursor: pointer;
  user-select: none;
  transition: color 0.1s;
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

.channel-wrapper {
  position: relative;
}

.channel-gear {
  position: absolute;
  right: 12px;
  top: 50%;
  transform: translateY(-50%);
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
  opacity: 0;
  transition: opacity 0.1s;
}

.channel-wrapper:hover .channel-gear { opacity: 1; }
.channel-gear:hover { color: var(--text-normal); background: var(--bg-modifier-hover); box-shadow: none; }

.channel-group-title.can-drag { cursor: grab; }
.channel-group-title.can-drag:active { cursor: grabbing; }
</style>
