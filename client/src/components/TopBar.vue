<template>
  <div class="top-bar" v-if="server">
    <span class="top-bar-name">{{ server.name }}</span>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { activeServer } from "../store";

const server = computed(() => activeServer());
</script>

<style scoped>
.top-bar {
  height: 28px;
  background: var(--bg-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.top-bar-name {
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--text-muted);
}
</style>
