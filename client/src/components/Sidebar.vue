<template>
  <div class="sidebar">
    <div class="sidebar-header" @click="store.showServerSettingsModal = true">
      <h2>{{ server?.name }}</h2>
      <ChevronDown :size="16" class="sidebar-header-icon" />
    </div>

    <div
      class="channel-list"
      @contextmenu.prevent="onContextMenu"
      @dragover.prevent="onDragOver"
      @drop="onDrop"
    >
      <!-- Header channels sans groupe -->
      <div v-if="ungroupedChannels.length || canManage" class="channel-group-title ungrouped-header">
        <span>Channels</span>
        <button v-if="canManage" class="group-add-btn" @click.stop="createInGroup = undefined; showCreateChannel = true" title="Creer un channel">
          <Plus :size="14" />
        </button>
      </div>

      <!-- Channels sans groupe -->
      <template v-for="ch in ungroupedChannels" :key="ch.id">
        <ChannelItem
          :channel="ch"
          :draggable="canManage"
          @dragstart="onDragStart('channel', ch.id, $event)"
          @contextmenu="onChannelContextMenu(ch, $event)"
          :class="{ 'drop-above': dropTarget?.id === ch.id && dropTarget?.position === 'above',
                     'drop-below': dropTarget?.id === ch.id && dropTarget?.position === 'below' }"
        />
      </template>

      <!-- Groupes -->
      <div
        v-for="group in state?.groups"
        :key="'g' + group.id"
        class="channel-group"
        :class="{ 'drop-into': dropTarget?.id === group.id && dropTarget?.type === 'group' }"
      >
        <div
          class="channel-group-title"
          :draggable="canManage"
          @click="toggleGroup(group.id)"
          @dragstart="onDragStart('group', group.id, $event)"
          @dragover.prevent.stop="onGroupDragOver(group.id, $event)"
          @drop.stop="onGroupDrop(group.id)"
        >
          <ChevronRight :size="12" class="group-arrow" :class="{ expanded: !collapsed.has(group.id) }" />
          <span>{{ group.name }}</span>
          <button v-if="canManage" class="group-add-btn" @click.stop="createInGroup = group.id; showCreateChannel = true" title="Creer un channel">
            <Plus :size="14" />
          </button>
        </div>
        <template v-if="!collapsed.has(group.id)">
          <template v-for="ch in getGroupChannels(group.id)" :key="ch.id">
            <ChannelItem
              :channel="ch"
              :draggable="canManage"
              @dragstart="onDragStart('channel', ch.id, $event)"
              @contextmenu="onChannelContextMenu(ch, $event)"
              :class="{ 'drop-above': dropTarget?.id === ch.id && dropTarget?.position === 'above',
                         'drop-below': dropTarget?.id === ch.id && dropTarget?.position === 'below' }"
            />
          </template>
        </template>
      </div>
    </div>

    <ContextMenu
      v-if="ctxMenu"
      :x="ctxMenu.x"
      :y="ctxMenu.y"
      :items="ctxMenu.items"
      @close="ctxMenu = null"
    />

    <CreateChannelModal v-if="showCreateChannel" :group-id="createInGroup" @close="showCreateChannel = false" />
    <CreateGroupModal v-if="showCreateGroup" @close="showCreateGroup = false" />
    <EditChannelModal v-if="editingChannel" :channel="editingChannel" @close="editingChannel = null" />
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from "vue";
import { ChevronDown, ChevronRight, Plus } from "lucide-vue-next";
import {
  store,
  activeState,
  activeServer,
} from "../store";
import { api } from "../api";
import * as perms from "../permissions";
import ChannelItem from "./ChannelItem.vue";
import ContextMenu, { type MenuItem } from "./ContextMenu.vue";
import CreateChannelModal from "./CreateChannelModal.vue";
import CreateGroupModal from "./CreateGroupModal.vue";
import EditChannelModal from "./EditChannelModal.vue";
import type { Channel } from "../api";

const state = computed(() => activeState());
const server = computed(() => activeServer());

const collapsed = reactive(new Set<number>());

const canManage = computed(() =>
  perms.has(state.value?.permissions ?? 0, perms.MANAGE_CHANNELS)
);

const ungroupedChannels = computed(() =>
  state.value?.channels.filter((c) => !c.group_id) ?? []
);

function getGroupChannels(groupId: number) {
  return state.value?.channels.filter((c) => c.group_id === groupId) ?? [];
}

function toggleGroup(groupId: number) {
  if (collapsed.has(groupId)) {
    collapsed.delete(groupId);
  } else {
    collapsed.add(groupId);
  }
}

// ── Context menu ──
const ctxMenu = ref<{ x: number; y: number; items: MenuItem[] } | null>(null);
const showCreateChannel = ref(false);
const showCreateGroup = ref(false);
const createInGroup = ref<number | undefined>(undefined);
const editingChannel = ref<Channel | null>(null);

function onChannelContextMenu(channel: Channel, e: MouseEvent) {
  if (!canManage.value) return;

  const s = activeServer();
  const st = activeState();

  const items: MenuItem[] = [
    { label: "Modifier le channel", action: () => { editingChannel.value = channel; } },
    {
      label: "Supprimer le channel",
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

  // Si on a cliqué sur un channel, le menu channel sera affiché à la place
  const target = (e.target as HTMLElement).closest("[data-channel-id]");
  if (target) return;

  const items: MenuItem[] = [
    { label: "Creer un channel", action: () => { createInGroup.value = undefined; showCreateChannel.value = true; } },
    { label: "Creer un groupe", action: () => { showCreateGroup.value = true; } },
  ];

  ctxMenu.value = { x: e.clientX, y: e.clientY, items };
}

// ── Drag & drop ──
const dragItem = ref<{ type: "channel" | "group"; id: number } | null>(null);
const dropTarget = ref<{ type: string; id: number; position?: "above" | "below" } | null>(null);

function onDragStart(type: "channel" | "group", id: number, e: DragEvent) {
  dragItem.value = { type, id };
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", `${type}:${id}`);
  }
}

function onDragOver(e: DragEvent) {
  if (!dragItem.value || dragItem.value.type !== "channel") return;

  const target = (e.target as HTMLElement).closest("[data-channel-id]");
  if (!target) {
    dropTarget.value = null;
    return;
  }

  const id = Number(target.getAttribute("data-channel-id"));
  const rect = target.getBoundingClientRect();
  const position = e.clientY < rect.top + rect.height / 2 ? "above" : "below";
  dropTarget.value = { type: "channel", id, position };
}

function onGroupDragOver(groupId: number, _e: DragEvent) {
  if (!dragItem.value) return;

  if (dragItem.value.type === "channel") {
    dropTarget.value = { type: "group", id: groupId };
  }
}

async function onDrop() {
  if (!dragItem.value || !dropTarget.value) {
    resetDrag();
    return;
  }

  const s = activeServer();
  const st = activeState();
  if (!s || !st) { resetDrag(); return; }

  if (dragItem.value.type === "channel" && dropTarget.value.type === "channel") {
    const channels = [...st.channels];
    const fromIdx = channels.findIndex((c) => c.id === dragItem.value!.id);
    const toIdx = channels.findIndex((c) => c.id === dropTarget.value!.id);
    if (fromIdx < 0 || toIdx < 0) { resetDrag(); return; }

    // Move to same group as target
    channels[fromIdx].group_id = channels[toIdx].group_id;
    const [moved] = channels.splice(fromIdx, 1);
    const newToIdx = channels.findIndex((c) => c.id === dropTarget.value!.id);
    const insertIdx = dropTarget.value.position === "above" ? newToIdx : newToIdx + 1;
    channels.splice(insertIdx, 0, moved);

    st.channels = channels;
    await api.moveChannel(s.url, s.token, moved.id, moved.group_id);
    await api.reorderChannels(s.url, s.token, channels.map((c) => c.id));
  }

  resetDrag();
}

async function onGroupDrop(groupId: number) {
  if (!dragItem.value) { resetDrag(); return; }

  const s = activeServer();
  const st = activeState();
  if (!s || !st) { resetDrag(); return; }

  if (dragItem.value.type === "channel") {
    const ch = st.channels.find((c) => c.id === dragItem.value!.id);
    if (ch) {
      ch.group_id = groupId;
      await api.moveChannel(s.url, s.token, ch.id, groupId);
    }
  } else if (dragItem.value.type === "group") {
    const groups = [...st.groups];
    const fromIdx = groups.findIndex((g) => g.id === dragItem.value!.id);
    const toIdx = groups.findIndex((g) => g.id === groupId);
    if (fromIdx >= 0 && toIdx >= 0 && fromIdx !== toIdx) {
      const [moved] = groups.splice(fromIdx, 1);
      groups.splice(toIdx, 0, moved);
      st.groups = groups;
      await api.reorderGroups(s.url, s.token, groups.map((g) => g.id));
    }
  }

  resetDrag();
}

function resetDrag() {
  dragItem.value = null;
  dropTarget.value = null;
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
  transition: background 0.1s;
}

.channel-group.drop-into {
  background: var(--bg-modifier-hover);
  border-radius: 8px;
  margin-left: 4px;
  margin-right: 4px;
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

.channel-group-title[draggable="true"] {
  cursor: grab;
}

.channel-group-title[draggable="true"]:active {
  cursor: grabbing;
}

.group-arrow {
  transition: transform 0.15s;
  flex-shrink: 0;
}

.group-arrow.expanded {
  transform: rotate(90deg);
}

/* Drop indicators */
:deep(.drop-above) {
  border-top: 2px solid var(--accent) !important;
}

:deep(.drop-below) {
  border-bottom: 2px solid var(--accent) !important;
}
</style>
