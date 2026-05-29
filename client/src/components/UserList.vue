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
      <div class="user-avatar-small">
        <img v-if="resolveAvatarUrl(uid)" :src="resolveAvatarUrl(uid)!" />
        <span v-else>{{ resolveUser(uid)[0]?.toUpperCase() }}</span>
      </div>
      <span :style="resolveUserColor(uid) ? `color:${resolveUserColor(uid)}` : ''">{{ resolveUser(uid) }}</span>
      <span v-if="isGuest(uid)" class="guest-tag">Guest</span>
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
      <div class="user-avatar-small">
        <img v-if="resolveAvatarUrl(user.id)" :src="resolveAvatarUrl(user.id)!" />
        <span v-else>{{ user.display_name[0]?.toUpperCase() }}</span>
      </div>
      <span :style="resolveUserColor(user.id) ? `color:${resolveUserColor(user.id)}` : ''">{{ user.display_name }}</span>
      <span v-if="user.guest" class="guest-tag">Guest</span>
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
// no icons needed
import { activeState, resolveUser, resolveUserColor, resolveAvatarUrl, isGuest } from "../store";
import type { User } from "../api";
import UserCard from "./UserCard.vue";

const state = computed(() => activeState());

const onlineList = computed(() =>
  state.value ? [...state.value.onlineUsers] : []
);

const offlineList = computed(() => {
  if (!state.value) return [];
  const online = state.value.onlineUsers;
  return [...state.value.users.values()].filter((u) => !online.has(u.id) && !u.guest);
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
  background: var(--bg-secondary);
  flex-shrink: 0;
  padding: 16px 12px;
  overflow-y: auto;
  border-left: 1px solid var(--border-soft);
}

.user-list-header {
  font-family: var(--font-mono);
  font-size: 0.625rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-faint);
  padding: 0 8px 8px;
}

.offline-header {
  margin-top: 16px;
}

.user-avatar-small {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  overflow: hidden;
  font-size: 0.625rem;
  font-weight: 700;
  color: var(--text-faint);
}

.user-avatar-small img {
  width: 100%;
  height: 100%;
  object-fit: cover;
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
  user-select: none;
}

.user-list-item:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
}

.user-list-item.offline {
  opacity: 0.45;
}

</style>
