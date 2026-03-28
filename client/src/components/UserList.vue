<template>
  <div class="user-list">
    <div class="user-list-header">
      En ligne — {{ state?.onlineUsers.size ?? 0 }}
    </div>
    <div
      v-for="uid in onlineList"
      :key="uid"
      class="user-list-item"
      @click="openCard(uid, $event)"
    >
      <Circle class="status-dot online" :size="8" fill="currentColor" />
      <span>{{ resolveUser(uid) }}</span>
    </div>

    <div class="user-list-header offline-header">
      Hors ligne — {{ offlineList.length }}
    </div>
    <div
      v-for="user in offlineList"
      :key="user.id"
      class="user-list-item offline"
      @click="openCard(user.id, $event)"
    >
      <Circle class="status-dot" :size="8" fill="currentColor" />
      <span>{{ user.display_name }}</span>
    </div>

    <UserCard
      v-if="cardUser"
      :user="cardUser"
      :x="cardX"
      :y="cardY"
      @close="cardUser = null"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { Circle } from "lucide-vue-next";
import { activeState, resolveUser } from "../store";
import type { User } from "../api";
import UserCard from "./UserCard.vue";

const state = computed(() => activeState());

const onlineList = computed(() =>
  state.value ? [...state.value.onlineUsers] : []
);

const offlineList = computed(() => {
  if (!state.value) return [];
  const online = state.value.onlineUsers;
  return [...state.value.users.values()].filter((u) => !online.has(u.id));
});

const cardUser = ref<User | null>(null);
const cardX = ref(0);
const cardY = ref(0);

function openCard(userId: number, e: MouseEvent) {
  const user = state.value?.users.get(userId);
  if (!user) return;

  const el = e.currentTarget as HTMLElement;
  const rect = el.getBoundingClientRect();
  cardX.value = rect.left - 308;
  cardY.value = rect.top;
  cardUser.value = user;
}
</script>

<style scoped>
.user-list {
  width: 240px;
  background: var(--bg-primary);
  flex-shrink: 0;
  padding: 16px 8px;
  overflow-y: auto;
  border-left: 1px solid var(--border);
}

.user-list-header {
  font-size: 0.6875rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-faint);
  padding: 0 8px 8px;
}

.offline-header {
  margin-top: 16px;
}

.user-list-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  border-radius: 8px;
  color: var(--text-muted);
  font-size: 0.8125rem;
  font-weight: 500;
  cursor: pointer;
}

.user-list-item:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
}

.user-list-item.offline {
  opacity: 0.45;
}

.user-list-item.offline .status-dot {
  color: var(--text-faint);
}

.status-dot {
  color: var(--green);
  flex-shrink: 0;
}
</style>
