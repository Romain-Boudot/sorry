<template>
  <div class="chat-header">
    <Hash class="channel-icon" :size="18" />
    <span>{{ activeChannel?.name }}</span>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Hash } from "lucide-vue-next";
import { activeState } from "../store";

const state = computed(() => activeState());
const activeChannel = computed(() =>
  state.value?.channels.find((c) => c.id === state.value?.activeChannelId)
);
</script>
