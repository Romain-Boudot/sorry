<template>
  <div class="sidebar">
    <div class="sidebar-header" @click="store.showServerSettingsModal = true">
      <h2>{{ server?.name }}</h2>
      <ChevronDown :size="16" class="sidebar-header-icon" />
    </div>

    <div class="channel-list">
      <!-- Channels sans groupe -->
      <template v-for="ch in ungroupedChannels" :key="ch.id">
        <ChannelItem :channel="ch" />
      </template>

      <!-- Groupes -->
      <div v-for="group in state?.groups" :key="group.id" class="channel-group">
        <div class="channel-group-title" @click="toggleGroup(group.id)">
          <ChevronRight :size="12" class="group-arrow" :class="{ expanded: !collapsed.has(group.id) }" />
          <span>{{ group.name }}</span>
        </div>
        <template v-if="!collapsed.has(group.id)">
          <template v-for="ch in getGroupChannels(group.id)" :key="ch.id">
            <ChannelItem :channel="ch" />
          </template>
        </template>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive } from "vue";
import { ChevronDown, ChevronRight } from "lucide-vue-next";
import {
  store,
  activeState,
  activeServer,
} from "../store";
import ChannelItem from "./ChannelItem.vue";

const state = computed(() => activeState());
const server = computed(() => activeServer());

const collapsed = reactive(new Set<number>());

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
  transition: color 0.1s;
}

.channel-group-title:hover {
  color: var(--text-muted);
}

.group-arrow {
  transition: transform 0.15s;
  flex-shrink: 0;
}

.group-arrow.expanded {
  transform: rotate(90deg);
}
</style>
