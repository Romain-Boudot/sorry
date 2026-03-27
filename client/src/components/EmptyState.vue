<template>
  <div class="empty-state">
    <div class="empty-state-content">
      <ServerCrash :size="48" :stroke-width="1.2" />
      <h2>Aucun serveur</h2>
      <p>Tu n'as rejoint aucun serveur avec ce client.</p>
      <p>Clique sur <Plus :size="14" style="vertical-align: middle" /> a gauche pour en ajouter un.</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ServerCrash, Plus } from "lucide-vue-next";
</script>
