<template>
  <div class="modal-overlay" @click.self="close">
    <div class="settings">
      <div class="settings-sidebar">
        <div
          v-for="tab in tabs"
          :key="tab.id"
          class="settings-tab"
          :class="{ active: activeTab === tab.id }"
          @click="activeTab = tab.id"
        >
          <component :is="tab.icon" :size="16" />
          <span>{{ tab.label }}</span>
        </div>

        <div class="sidebar-spacer"></div>

        <div class="settings-tab danger" @click="handleDelete">
          <Trash2 :size="16" />
          <span>Supprimer</span>
        </div>
      </div>

      <div class="settings-content">
        <div class="settings-header">
          <h2>
            <Hash v-if="channel?.kind === 'text'" :size="18" />
            <Volume2 v-else :size="18" />
            {{ channel?.name }}
          </h2>
          <button class="settings-close" @click="close">
            <X :size="20" />
          </button>
        </div>

        <!-- General -->
        <div v-if="activeTab === 'general'" class="settings-body">
          <div class="card">
            <div class="card-title">Nom du channel</div>
            <div class="input-row">
              <input v-model="channelName" type="text" placeholder="Nom" @keydown.enter="saveName" />
              <SaveButton :loading="savingName" :saved="nameSaved" :disabled="!channelName.trim() || channelName === channel?.name" @click="saveName" />
            </div>
          </div>
        </div>

        <!-- Permissions -->
        <div v-if="activeTab === 'permissions'" class="settings-body">
          <p class="card-hint">Autorise ou refuse des permissions par role pour ce channel.</p>

          <div v-for="role in roles" :key="role.id" class="ow-role">
            <div class="ow-role-header" @click="toggleRole(role.id)">
              <div class="dot" :style="`background:${role.color || 'var(--text-muted)'}`"></div>
              <span class="ow-role-name">{{ role.name }}</span>
              <span v-if="hasOverwrite(role.id)" class="badge">modifie</span>
              <ChevronDown :size="14" class="ow-chevron" :class="{ open: expandedRole === role.id }" />
            </div>
            <div v-if="expandedRole === role.id" class="ow-body">
              <div v-for="p in permsList" :key="p.flag" class="ow-perm">
                <span class="ow-perm-name">{{ p.name }}</span>
                <div class="tristate">
                  <button
                    v-for="s in tristateOptions"
                    :key="s.value"
                    class="tri-btn"
                    :class="[s.value, { active: getState(role.id, p.flag) === s.value }]"
                    @click="setState(role.id, p.flag, s.value)"
                    :title="s.label"
                  >{{ s.icon }}</button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { X, Hash, Volume2, Trash2, ChevronDown, Settings, Shield } from "lucide-vue-next";
import SaveButton from "./SaveButton.vue";
import { store, activeState, activeServer } from "../store";
import { api } from "../api";
import * as perms from "../permissions";

const state = computed(() => activeState());
const channelId = computed(() => store.channelSettingsId);
const channel = computed(() => state.value?.channels.find((c) => c.id === channelId.value));

const activeTab = ref("general");

const tabs = [
  { id: "general", label: "General", icon: Settings },
  { id: "permissions", label: "Permissions", icon: Shield },
];

const tristateOptions = [
  { value: "inherit" as const, label: "Heriter", icon: "/" },
  { value: "allow" as const, label: "Autoriser", icon: "\u2713" },
  { value: "deny" as const, label: "Refuser", icon: "\u2715" },
];

const permsList = [
  { name: "Voir le channel", flag: perms.VIEW_CHANNELS },
  { name: "Envoyer des messages", flag: perms.SEND_MESSAGES },
  { name: "Historique", flag: perms.READ_MESSAGE_HISTORY },
  { name: "Joindre des fichiers", flag: perms.ATTACH_FILES },
  { name: "Gerer les messages", flag: perms.MANAGE_MESSAGES },
  { name: "Se connecter (vocal)", flag: perms.CONNECT },
  { name: "Parler (vocal)", flag: perms.SPEAK },
];

// General
const channelName = ref("");
const savingName = ref(false);
const nameSaved = ref(false);

async function saveName() {
  const s = activeServer();
  const st = activeState();
  const id = channelId.value;
  if (!s || !st || !id || !channelName.value.trim()) return;

  savingName.value = true;
  try {
    await api.updateChannel(s.url, s.token, id, { name: channelName.value.trim() });
    const ch = st.channels.find((c) => c.id === id);
    if (ch) ch.name = channelName.value.trim();
    nameSaved.value = true;
    setTimeout(() => (nameSaved.value = false), 2500);
  } finally {
    savingName.value = false;
  }
}

// Permissions
const roles = ref<{ id: number; name: string; permissions: number; color: string | null; position: number }[]>([]);
const overwrites = ref<Map<number, { allow: number; deny: number }>>(new Map());
const expandedRole = ref<number | null>(null);

function toggleRole(roleId: number) {
  expandedRole.value = expandedRole.value === roleId ? null : roleId;
}

function hasOverwrite(roleId: number): boolean {
  const ow = overwrites.value.get(roleId);
  return !!ow && (ow.allow !== 0 || ow.deny !== 0);
}

function getState(roleId: number, flag: number): "inherit" | "allow" | "deny" {
  const ow = overwrites.value.get(roleId);
  if (!ow) return "inherit";
  if (ow.allow & flag) return "allow";
  if (ow.deny & flag) return "deny";
  return "inherit";
}

async function setState(roleId: number, flag: number, newState: "inherit" | "allow" | "deny") {
  const s = activeServer();
  const id = channelId.value;
  if (!s || !id) return;

  let ow = overwrites.value.get(roleId) || { allow: 0, deny: 0 };
  ow.allow &= ~flag;
  ow.deny &= ~flag;
  if (newState === "allow") ow.allow |= flag;
  if (newState === "deny") ow.deny |= flag;

  if (ow.allow === 0 && ow.deny === 0) {
    await api.deleteOverwrite(s.url, s.token, id, roleId);
    overwrites.value.delete(roleId);
  } else {
    await api.setOverwrite(s.url, s.token, id, roleId, ow.allow, ow.deny);
    overwrites.value.set(roleId, ow);
  }
  overwrites.value = new Map(overwrites.value);
}

// Delete
async function handleDelete() {
  const s = activeServer();
  const st = activeState();
  const id = channelId.value;
  if (!s || !st || !id) return;

  await api.deleteChannel(s.url, s.token, id);
  st.channels = st.channels.filter((c) => c.id !== id);
  close();
}

// Init
onMounted(async () => {
  const s = activeServer();
  const id = channelId.value;
  if (!s || !id) return;

  channelName.value = channel.value?.name ?? "";

  try {
    const allRoles = await api.listRoles(s.url, s.token);
    // Filter out Owner (ID=1), sort custom roles by position, Membre (ID=2) at the end
    const custom = allRoles.filter(r => r.id > 2).sort((a, b) => a.position - b.position);
    const membre = allRoles.find(r => r.id === 2);
    roles.value = membre ? [...custom, { ...membre, name: "Permissions par defaut" }] : custom;
  } catch {}

  try {
    const list = await api.listOverwrites(s.url, s.token, id);
    overwrites.value = new Map(list.map((o) => [o.role_id, { allow: o.allow, deny: o.deny }]));
  } catch {}
});

function close() {
  store.channelSettingsId = null;
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

/* ── Sidebar ── */
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

/* ── Content ── */
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
  display: flex;
  align-items: center;
  gap: 8px;
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

/* ── Card ── */
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
  margin-bottom: 8px;
}

.card-hint {
  font-size: 0.75rem;
  color: var(--text-faint);
  margin-bottom: 12px;
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

/* ── Dot ── */
.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

/* ── Overwrite roles ── */
.ow-role { margin-bottom: 2px; }

.ow-role-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 8px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.8125rem;
  color: var(--text-normal);
  transition: background 0.1s;
}
.ow-role-header:hover { background: var(--bg-modifier-hover); }

.ow-role-name { flex: 1; }

.badge {
  font-size: 0.5625rem;
  font-weight: 600;
  background: var(--accent);
  color: var(--text-bright);
  padding: 1px 6px;
  border-radius: 4px;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.ow-chevron {
  color: var(--text-faint);
  transition: transform 0.2s;
}
.ow-chevron.open { transform: rotate(180deg); }

.ow-body {
  padding: 4px 0 8px 26px;
}

.ow-perm {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 0;
}

.ow-perm-name {
  font-size: 0.8125rem;
  color: var(--text-normal);
}

/* ── Tristate ── */
.tristate {
  display: flex;
  border-radius: 6px;
  overflow: hidden;
  border: 1px solid var(--border);
}

.tri-btn {
  width: 28px;
  height: 24px;
  padding: 0;
  margin: 0;
  font-size: 0.75rem;
  background: var(--bg-tertiary);
  color: var(--text-faint);
  cursor: pointer;
  border: none;
  border-radius: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.1s, color 0.1s;
}
.tri-btn:not(:last-child) { border-right: 1px solid var(--border); }
.tri-btn:hover { background: var(--bg-modifier-hover); box-shadow: none; }

.tri-btn.active.inherit { background: var(--bg-modifier-active); color: var(--text-normal); }
.tri-btn.active.allow { background: var(--green-bg); color: var(--green); }
.tri-btn.active.deny { background: var(--danger-bg-hover); color: var(--danger); }
</style>
