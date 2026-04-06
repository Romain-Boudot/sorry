<template>
  <div class="modal-overlay" @click.self="close">
    <div class="settings">
      <div class="settings-sidebar">
        <div class="settings-tab active">
          <Settings :size="16" />
          <span>General</span>
        </div>

        <div class="sidebar-spacer"></div>

        <div class="settings-tab danger" @click="handleDelete">
          <Trash2 :size="16" />
          <span>Supprimer</span>
        </div>
      </div>

      <div class="settings-content">
        <div class="settings-header">
          <h2>{{ group?.name }}</h2>
          <button class="settings-close" @click="close">
            <X :size="20" />
          </button>
        </div>

        <div class="settings-body">
          <div class="card">
            <div class="card-title">Nom du groupe</div>
            <div class="input-row">
              <input v-model="groupName" type="text" placeholder="Nom" @keydown.enter="saveName" />
              <button class="btn-sm" @click="saveName" :disabled="!groupName.trim() || groupName === group?.name">
                Sauvegarder
              </button>
            </div>
            <p class="toast-success" v-if="nameSaved">Sauvegarde !</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { X, Trash2, Settings } from "lucide-vue-next";
import { store, activeState, activeServer } from "../store";
import { api } from "../api";

const state = computed(() => activeState());
const groupId = computed(() => store.groupSettingsId);
const group = computed(() => state.value?.groups.find((g) => g.id === groupId.value));

const groupName = ref("");
const nameSaved = ref(false);

onMounted(() => {
  groupName.value = group.value?.name ?? "";
});

async function saveName() {
  const s = activeServer();
  const st = activeState();
  const id = groupId.value;
  if (!s || !st || !id || !groupName.value.trim()) return;

  await api.updateGroup(s.url, s.token, id, groupName.value.trim());
  const g = st.groups.find((g) => g.id === id);
  if (g) g.name = groupName.value.trim();
  nameSaved.value = true;
  setTimeout(() => (nameSaved.value = false), 2000);
}

async function handleDelete() {
  const s = activeServer();
  const st = activeState();
  const id = groupId.value;
  if (!s || !st || !id) return;

  await api.deleteGroup(s.url, s.token, id);
  st.groups = st.groups.filter((g) => g.id !== id);
  st.channels = st.channels.map((c) =>
    c.group_id === id ? { ...c, group_id: null } : c
  );
  close();
}

function close() {
  store.groupSettingsId = null;
}
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: var(--overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.settings {
  background: var(--bg-primary);
  border-radius: 12px;
  width: 860px;
  height: 600px;
  display: flex;
  overflow: hidden;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
}

.settings-sidebar {
  width: 190px;
  background: var(--bg-secondary);
  padding: 16px 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.sidebar-spacer { flex: 1; }

.settings-tab {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-muted);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.settings-tab:hover { background: var(--bg-modifier-hover); color: var(--text-normal); }
.settings-tab.active { background: var(--bg-modifier-active); color: var(--header-primary); }
.settings-tab.danger { color: var(--danger); }
.settings-tab.danger:hover { background: var(--danger-bg); }

.settings-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px 12px;
  flex-shrink: 0;
}

.settings-header h2 {
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--header-primary);
}

.settings-close {
  width: 32px;
  height: 32px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}
.settings-close:hover { color: var(--text-normal); background: var(--bg-modifier-hover); box-shadow: none; }

.settings-body {
  padding: 0 24px 24px;
  overflow-y: auto;
  flex: 1;
}

.card {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 16px;
}

.card-title {
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-muted);
  margin-bottom: 8px;
}

.input-row {
  display: flex;
  gap: 8px;
}

.input-row input[type="text"] {
  flex: 1;
  padding: 8px 10px;
  border-radius: 6px;
  border: none;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.875rem;
  font-family: inherit;
  outline: none;
}
.input-row input::placeholder { color: var(--text-faint); }

.btn-sm {
  width: auto;
  padding: 8px 16px;
  margin: 0;
  font-size: 0.8125rem;
  border-radius: 6px;
  flex-shrink: 0;
}

.toast-success {
  font-size: 0.75rem;
  color: var(--green);
  margin-top: 6px;
}
</style>
