<template>
  <div class="user-list">
    <div class="user-list-header">
      En ligne — {{ state?.onlineUsers.size ?? 0 }}
    </div>
    <div v-for="uid in onlineList" :key="uid" class="user-list-item">
      <span class="status-dot online"></span>
      <span>{{ resolveUser(uid) }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { activeState, resolveUser } from "../store";

const state = computed(() => activeState());

const onlineList = computed(() =>
  state.value ? [...state.value.onlineUsers] : []
);
</script>
