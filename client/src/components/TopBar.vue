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
