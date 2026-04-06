<template>
  <div class="settings-body">
    <div class="card">
      <div class="card-title">Icone du serveur</div>
      <p class="card-hint">L'icone affichee dans la liste des serveurs.</p>
      <div class="avatar-setting">
        <div class="avatar-preview" @click="iconInput?.click()">
          <img v-if="serverIconUrl" :src="serverIconUrl" />
          <span v-else class="avatar-placeholder">
            <Server :size="24" />
          </span>
          <div class="avatar-overlay">
            <Camera :size="16" />
          </div>
        </div>
        <div class="avatar-actions">
          <button class="btn-sm" @click="iconInput?.click()">Changer</button>
          <button v-if="serverIconUrl" class="btn-sm btn-danger-outline" @click="removeServerIcon">Supprimer</button>
        </div>
        <input ref="iconInput" type="file" accept="image/png,image/jpeg,image/gif,image/webp" hidden @change="onIconSelect" />
      </div>
    </div>

    <div class="card">
      <div class="card-title">Nom du serveur</div>
      <div class="input-row">
        <input v-model="serverName" type="text" placeholder="Mon serveur" maxlength="64" />
        <button class="btn-sm" @click="saveServerInfo" :disabled="savingServer">
          {{ savingServer ? '...' : 'Sauvegarder' }}
        </button>
      </div>
    </div>

    <div class="card">
      <div class="card-title">Description</div>
      <p class="card-hint">Une courte description de ton serveur (max 256 caracteres).</p>
      <textarea v-model="serverDescription" class="server-desc-input" placeholder="Description du serveur..." maxlength="256" rows="3"></textarea>
      <button class="btn-sm" style="margin-top: 8px;" @click="saveServerInfo" :disabled="savingServer">
        {{ savingServer ? '...' : 'Sauvegarder' }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { Server, Camera } from "lucide-vue-next";
import { store, activeServer, persistServers } from "../../store";
import { api } from "../../api";

const serverName = ref("");
const serverDescription = ref("");
const serverIconUrl = ref<string | null>(null);
const iconInput = ref<HTMLInputElement>();
const savingServer = ref(false);

onMounted(async () => {
  const s = activeServer();
  if (!s) return;
  try {
    const info = await api.serverInfo(s.url);
    serverName.value = info.name;
    serverDescription.value = info.description || "";
    serverIconUrl.value = info.icon_url ? `${s.url}${info.icon_url}` : null;
  } catch {}
});

async function saveServerInfo() {
  const s = activeServer();
  if (!s) return;
  savingServer.value = true;
  try {
    await api.updateServer(s.url, s.token, {
      name: serverName.value,
      description: serverDescription.value,
    });
    const saved = store.savedServers.find((sv) => sv.id === store.activeServerId);
    if (saved) {
      saved.name = serverName.value;
      persistServers();
    }
  } finally {
    savingServer.value = false;
  }
}

async function onIconSelect(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  const s = activeServer();
  if (!s) return;
  try {
    const info = await api.uploadServerIcon(s.url, s.token, file);
    serverIconUrl.value = info.icon_url ? `${s.url}${info.icon_url}` : null;
  } catch {}
  input.value = "";
}

async function removeServerIcon() {
  const s = activeServer();
  if (!s) return;
  try {
    await api.deleteServerIcon(s.url, s.token);
    serverIconUrl.value = null;
  } catch {}
}
</script>

<style scoped>
.settings-body {
  padding: 0 24px 24px;
  overflow-y: auto;
  flex: 1;
}

.card {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 16px;
}

.card-title {
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-muted);
  margin-bottom: 4px;
}

.card-hint {
  font-size: 0.75rem;
  color: var(--text-faint);
  margin-bottom: 8px;
}

.avatar-setting {
  display: flex;
  align-items: center;
  gap: 16px;
}

.avatar-preview {
  width: 72px;
  height: 72px;
  border-radius: 50%;
  background: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  position: relative;
  overflow: hidden;
  flex-shrink: 0;
}

.avatar-preview img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.avatar-placeholder {
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--text-faint);
}

.avatar-overlay {
  position: absolute;
  inset: 0;
  background: var(--overlay-light);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-bright);
  opacity: 0;
  transition: opacity 0.15s;
}

.avatar-preview:hover .avatar-overlay {
  opacity: 1;
}

.avatar-actions {
  display: flex;
  gap: 8px;
}

.btn-sm {
  width: auto;
  padding: 8px 16px;
  margin: 0;
  font-size: 0.8125rem;
  border-radius: 6px;
  flex-shrink: 0;
}

.btn-danger-outline {
  background: transparent !important;
  color: var(--danger) !important;
  border: 1px solid var(--danger) !important;
}

.btn-danger-outline:hover {
  background: var(--danger-bg-hover) !important;
  box-shadow: none !important;
}

.input-row {
  display: flex;
  gap: 8px;
}

.input-row input[type="text"] {
  flex: 1;
  min-width: 0;
  padding: 8px 10px;
  border-radius: 6px;
  border: none;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.875rem;
  font-family: inherit;
  outline: none;
}
.input-row input[type="text"]::placeholder { color: var(--text-faint); }

.server-desc-input {
  width: 100%;
  padding: 8px 10px;
  border-radius: 6px;
  border: none;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.875rem;
  font-family: inherit;
  outline: none;
  resize: vertical;
}

.server-desc-input::placeholder {
  color: var(--text-faint);
}
</style>
