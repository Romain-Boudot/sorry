<template>
  <div class="empty-state">
    <div class="empty-state-content">
      <ServerCrash :size="48" :stroke-width="1.2" />
      <h2>Aucun serveur</h2>
      <p>Tu n'as rejoint aucun serveur avec ce client.</p>
      <button class="add-btn" @click="store.showAddServerModal = true">
        <Plus :size="16" />
        Ajouter un serveur
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ServerCrash, Plus } from "lucide-vue-next";
import { store } from "../store";
</script>

<style scoped>
.empty-state {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-primary);
}

.empty-state-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  color: var(--text-muted);
  text-align: center;
}

.empty-state-content h2 {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--header-primary);
}

.add-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  width: auto;
  margin-top: 8px;
  padding: 10px 20px;
  font-size: 0.875rem;
  border-radius: 4px;
}
</style>
