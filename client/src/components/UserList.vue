<template>
  <div class="user-list">
    <div class="user-list-header">
      En ligne — {{ state?.onlineUsers.size ?? 0 }}
    </div>
    <div v-for="uid in onlineList" :key="uid" class="user-list-item">
      <Circle class="status-dot online" :size="8" fill="currentColor" />
      <span>{{ resolveUser(uid) }}</span>
    </div>

    <div class="user-list-header offline-header">
      Hors ligne — {{ offlineList.length }}
    </div>
    <div v-for="user in offlineList" :key="user.id" class="user-list-item offline">
      <Circle class="status-dot" :size="8" fill="currentColor" />
      <span>{{ user.display_name }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Circle } from "lucide-vue-next";
import { activeState, resolveUser } from "../store";

const state = computed(() => activeState());

const onlineList = computed(() =>
  state.value ? [...state.value.onlineUsers] : []
);

const offlineList = computed(() => {
  if (!state.value) return [];
  const online = state.value.onlineUsers;
  return [...state.value.users.values()].filter((u) => !online.has(u.id));
});
</script>
