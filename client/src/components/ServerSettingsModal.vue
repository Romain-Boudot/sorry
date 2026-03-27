<template>
  <div class="modal-overlay" @click.self="close">
    <div class="settings">
      <div class="settings-sidebar">
        <div
          v-for="tab in visibleTabs"
          :key="tab.id"
          class="settings-tab"
          :class="{ active: activeTab === tab.id }"
          @click="activeTab = tab.id"
        >
          <component :is="tab.icon" :size="16" />
          <span>{{ tab.label }}</span>
        </div>
      </div>

      <div class="settings-content">
        <div class="settings-header">
          <h2>{{ activeTabLabel }}</h2>
          <button class="settings-close" @click="close">
            <X :size="20" />
          </button>
        </div>

        <!-- Profil -->
        <div v-if="activeTab === 'profile'" class="settings-body">
          <div class="settings-section">
            <label>Display name</label>
            <p class="settings-hint">Ton nom visible sur ce serveur.</p>
            <div class="settings-input-row">
              <input v-model="displayName" type="text" placeholder="Mon pseudo" maxlength="32" />
              <button class="settings-save-btn" @click="saveDisplayName" :disabled="saving">
                {{ saving ? '...' : 'Sauvegarder' }}
              </button>
            </div>
            <p class="settings-success" v-if="saved">Sauvegarde !</p>
          </div>
        </div>

        <!-- Channels -->
        <div v-if="activeTab === 'channels'" class="settings-body">
          <div class="settings-section">
            <label>Creer un groupe</label>
            <div class="settings-input-row">
              <input v-model="newGroupName" type="text" placeholder="Nom du groupe" @keydown.enter="createGroup" />
              <button class="settings-save-btn" @click="createGroup" :disabled="!newGroupName.trim()">
                Creer
              </button>
            </div>
          </div>

          <div class="settings-section">
            <label>Creer un channel</label>
            <div class="settings-input-row">
              <input v-model="newChannelName" type="text" placeholder="Nom du channel" />
              <div class="dropdown-wrapper">
                <Dropdown v-model="newChannelKind" :options="channelKindOptions" />
              </div>
              <div class="dropdown-wrapper-wide">
                <Dropdown v-model="newChannelGroup" :options="groupOptions" placeholder="Aucun groupe" />
              </div>
              <button class="settings-save-btn" @click="createChannel" :disabled="!newChannelName.trim()">
                Creer
              </button>
            </div>
          </div>

          <div class="settings-section">
            <label>Channels</label>

            <!-- Ungrouped -->
            <div class="channel-list">
              <div v-for="(ch, i) in ungroupedChannels" :key="ch.id" class="channel-row">
                <Hash v-if="ch.kind === 'text'" :size="16" class="channel-row-icon" />
                <Volume2 v-else :size="16" class="channel-row-icon" />
                <span class="channel-row-name">{{ ch.name }}</span>
                <div class="channel-row-actions">
                  <button class="channel-action-btn" @click="moveChannelUp(ch.id)" :disabled="i === 0" title="Monter">
                    <ChevronUp :size="14" />
                  </button>
                  <button class="channel-action-btn" @click="moveChannelDown(ch.id)" :disabled="i === ungroupedChannels.length - 1" title="Descendre">
                    <ChevronDown :size="14" />
                  </button>
                  <button class="channel-delete-btn" @click="deleteChannel(ch.id)" title="Supprimer">
                    <Trash2 :size="14" />
                  </button>
                </div>
              </div>
            </div>

            <!-- Groups -->
            <div v-for="(group, gi) in state?.groups" :key="group.id" class="settings-group">
              <div class="settings-group-header">
                <span class="settings-group-name">{{ group.name }}</span>
                <div class="channel-row-actions">
                  <button class="channel-action-btn" @click="moveGroupUp(group.id)" :disabled="gi === 0" title="Monter">
                    <ChevronUp :size="14" />
                  </button>
                  <button class="channel-action-btn" @click="moveGroupDown(group.id)" :disabled="gi === (state?.groups.length ?? 0) - 1" title="Descendre">
                    <ChevronDown :size="14" />
                  </button>
                  <button class="channel-delete-btn" @click="deleteGroup(group.id)" title="Supprimer le groupe">
                    <Trash2 :size="14" />
                  </button>
                </div>
              </div>
              <div class="channel-list">
                <div v-for="(ch, i) in getGroupChannels(group.id)" :key="ch.id" class="channel-row">
                  <Hash v-if="ch.kind === 'text'" :size="16" class="channel-row-icon" />
                  <Volume2 v-else :size="16" class="channel-row-icon" />
                  <span class="channel-row-name">{{ ch.name }}</span>
                  <div class="channel-row-actions">
                    <button class="channel-action-btn" @click="moveChannelUp(ch.id)" :disabled="i === 0" title="Monter">
                      <ChevronUp :size="14" />
                    </button>
                    <button class="channel-action-btn" @click="moveChannelDown(ch.id)" :disabled="i === getGroupChannels(group.id).length - 1" title="Descendre">
                      <ChevronDown :size="14" />
                    </button>
                    <button class="channel-delete-btn" @click="deleteChannel(ch.id)" title="Supprimer">
                      <Trash2 :size="14" />
                    </button>
                  </div>
                </div>
                <p v-if="!getGroupChannels(group.id).length" class="settings-hint" style="padding-left: 8px;">Aucun channel</p>
              </div>
            </div>
          </div>
        </div>

        <!-- Roles -->
        <div v-if="activeTab === 'roles'" class="settings-body">
          <div class="settings-section">
            <label>Creer un role</label>
            <div class="settings-input-row">
              <input v-model="newRoleName" type="text" placeholder="Nom du role" />
              <button class="settings-save-btn" @click="createRole" :disabled="!newRoleName.trim()">
                Creer
              </button>
            </div>
          </div>

          <div class="settings-section">
            <label>Roles existants</label>
            <div class="channel-list">
              <div v-for="role in roles" :key="role.id" class="channel-row">
                <Shield :size="16" class="channel-row-icon" />
                <span class="channel-row-name">{{ role.name }}</span>
              </div>
              <p v-if="!roles.length" class="settings-hint">Aucun role.</p>
            </div>
          </div>
        </div>

        <!-- Moderation -->
        <div v-if="activeTab === 'moderation'" class="settings-body">
          <div class="settings-section">
            <label>Membres</label>
            <div class="channel-list">
              <div v-for="user in allUsers" :key="user.id" class="channel-row">
                <span class="channel-row-name">{{ user.display_name }}</span>
                <button
                  v-if="user.id !== state?.user?.id"
                  class="channel-delete-btn"
                  @click="kickUser(user.id)"
                  title="Kick"
                >
                  <UserX :size="14" />
                </button>
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
import { X, Hash, Volume2, Trash2, Shield, UserX, UserRound, LayoutList, ShieldCheck, Gavel, ChevronUp, ChevronDown } from "lucide-vue-next";
import Dropdown from "./Dropdown.vue";
import { store, activeState, activeServer } from "../store";
import { api, type Channel } from "../api";
import * as perms from "../permissions";

const state = computed(() => activeState());
const server = computed(() => activeServer());
const activeTab = ref("profile");

const allTabs = [
  { id: "profile", label: "Profil", icon: UserRound, permission: 0 },
  { id: "channels", label: "Channels", icon: LayoutList, permission: perms.MANAGE_CHANNELS },
  { id: "roles", label: "Roles", icon: ShieldCheck, permission: perms.MANAGE_ROLES },
  { id: "moderation", label: "Moderation", icon: Gavel, permission: perms.KICK_MEMBERS },
];

const visibleTabs = computed(() => {
  const p = state.value?.permissions ?? 0;
  return allTabs.filter((t) => t.permission === 0 || perms.has(p, t.permission));
});

const activeTabLabel = computed(() => allTabs.find((t) => t.id === activeTab.value)?.label ?? "");

// Profile
const displayName = ref(state.value?.user?.display_name || "");
const saving = ref(false);
const saved = ref(false);

async function saveDisplayName() {
  const s = activeServer();
  const st = activeState();
  if (!s || !st) return;

  saving.value = true;
  saved.value = false;
  try {
    const user = await api.updateDisplayName(s.url, s.token, displayName.value.trim());
    st.user = user;
    st.users.set(user.id, user);
    saved.value = true;
    setTimeout(() => (saved.value = false), 2000);
  } finally {
    saving.value = false;
  }
}

// Groups
const newGroupName = ref("");

const groupOptions = computed(() => [
  { value: "", label: "Aucun groupe" },
  ...(state.value?.groups.map((g) => ({ value: String(g.id), label: g.name })) ?? []),
]);

const ungroupedChannels = computed(() =>
  state.value?.channels.filter((c) => !c.group_id) ?? []
);

function getGroupChannels(groupId: number) {
  return state.value?.channels.filter((c) => c.group_id === groupId) ?? [];
}

async function createGroup() {
  const s = activeServer();
  const st = activeState();
  if (!s || !st || !newGroupName.value.trim()) return;

  const group = await api.createGroup(s.url, s.token, newGroupName.value.trim());
  st.groups.push(group);
  newGroupName.value = "";
}

async function deleteGroup(id: number) {
  const s = activeServer();
  const st = activeState();
  if (!s || !st) return;

  await api.deleteGroup(s.url, s.token, id);
  st.groups = st.groups.filter((g) => g.id !== id);
}

async function moveGroupUp(id: number) {
  const st = activeState();
  const s = activeServer();
  if (!st || !s) return;
  const i = st.groups.findIndex((g) => g.id === id);
  if (i <= 0) return;
  [st.groups[i - 1], st.groups[i]] = [st.groups[i], st.groups[i - 1]];
  await api.reorderGroups(s.url, s.token, st.groups.map((g) => g.id));
}

async function moveGroupDown(id: number) {
  const st = activeState();
  const s = activeServer();
  if (!st || !s) return;
  const i = st.groups.findIndex((g) => g.id === id);
  if (i < 0 || i >= st.groups.length - 1) return;
  [st.groups[i], st.groups[i + 1]] = [st.groups[i + 1], st.groups[i]];
  await api.reorderGroups(s.url, s.token, st.groups.map((g) => g.id));
}

// Channels
const newChannelName = ref("");
const newChannelKind = ref("text");
const newChannelGroup = ref("");
const channelKindOptions = [
  { value: "text", label: "Texte" },
  { value: "voice", label: "Vocal" },
];

async function createChannel() {
  const s = activeServer();
  const st = activeState();
  if (!s || !st || !newChannelName.value.trim()) return;

  const groupId = newChannelGroup.value ? Number(newChannelGroup.value) : undefined;
  const ch = await api.createChannel(s.url, s.token, newChannelName.value.trim(), newChannelKind.value as "text" | "voice", groupId);
  st.channels.push(ch);
  newChannelName.value = "";
}

async function deleteChannel(id: number) {
  const s = activeServer();
  const st = activeState();
  if (!s || !st) return;

  await fetch(`${s.url}/api/channels/${id}`, {
    method: "DELETE",
    headers: { Authorization: `Bearer ${s.token}` },
  });
  st.channels = st.channels.filter((c) => c.id !== id);
}

async function moveChannelUp(id: number) {
  const st = activeState();
  const s = activeServer();
  if (!st || !s) return;
  const i = st.channels.findIndex((c) => c.id === id);
  if (i <= 0) return;
  [st.channels[i - 1], st.channels[i]] = [st.channels[i], st.channels[i - 1]];
  await api.reorderChannels(s.url, s.token, st.channels.map((c) => c.id));
}

async function moveChannelDown(id: number) {
  const st = activeState();
  const s = activeServer();
  if (!st || !s) return;
  const i = st.channels.findIndex((c) => c.id === id);
  if (i < 0 || i >= st.channels.length - 1) return;
  [st.channels[i], st.channels[i + 1]] = [st.channels[i + 1], st.channels[i]];
  await api.reorderChannels(s.url, s.token, st.channels.map((c) => c.id));
}

// Roles
const roles = ref<{ id: number; name: string; permissions: number }[]>([]);
const newRoleName = ref("");

onMounted(async () => {
  const s = activeServer();
  if (!s) return;
  try {
    const res = await fetch(`${s.url}/api/roles`, {
      headers: { Authorization: `Bearer ${s.token}` },
    });
    if (res.ok) roles.value = await res.json();
  } catch {}
});

async function createRole() {
  const s = activeServer();
  if (!s || !newRoleName.value.trim()) return;

  const res = await fetch(`${s.url}/api/roles`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${s.token}`,
    },
    body: JSON.stringify({ name: newRoleName.value.trim(), permissions: 0 }),
  });
  if (res.ok) {
    const role = await res.json();
    roles.value.push(role);
    newRoleName.value = "";
  }
}

// Moderation
const allUsers = computed(() => {
  if (!state.value) return [];
  return [...state.value.users.values()];
});

async function kickUser(userId: number) {
  // TODO: implement kick endpoint
}

function close() {
  store.showServerSettingsModal = false;
}
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.settings {
  background: var(--bg-primary);
  border-radius: 8px;
  width: 800px;
  height: 600px;
  display: flex;
  overflow: hidden;
}

.settings-sidebar {
  width: 190px;
  background: var(--bg-secondary);
  padding: 16px 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.settings-tab {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 4px;
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-muted);
  cursor: pointer;
  transition: background 0.1s, color 0.1s;
}

.settings-tab:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
}

.settings-tab.active {
  background: var(--bg-modifier-active);
  color: var(--header-primary);
}

.settings-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px 12px;
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
  border-radius: 4px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}

.settings-close:hover {
  color: var(--text-normal);
  background: var(--bg-modifier-hover);
  box-shadow: none;
}

.settings-body {
  padding: 0 24px 24px;
}

.settings-section {
  margin-bottom: 24px;
}

.settings-section label {
  display: block;
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-muted);
  margin-bottom: 4px;
}

.settings-hint {
  font-size: 0.75rem;
  color: var(--text-faint);
  margin-bottom: 8px;
}

.settings-input-row {
  display: flex;
  gap: 8px;
}

.dropdown-wrapper {
  width: 110px;
  flex-shrink: 0;
}

.dropdown-wrapper-wide {
  width: 150px;
  flex-shrink: 0;
}

.settings-input-row input {
  flex: 1;
  padding: 8px 10px;
  border-radius: 4px;
  border: none;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.875rem;
  font-family: inherit;
  outline: none;
}

.settings-input-row input::placeholder {
  color: var(--text-faint);
}

.settings-save-btn {
  width: auto;
  padding: 8px 16px;
  margin: 0;
  font-size: 0.8125rem;
  border-radius: 4px;
}

.settings-success {
  font-size: 0.75rem;
  color: var(--green);
  margin-top: 6px;
}

.channel-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-top: 8px;
}

.channel-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 4px;
  transition: background 0.1s;
}

.channel-row:hover {
  background: var(--bg-modifier-hover);
}

.channel-row-icon {
  color: var(--text-faint);
  flex-shrink: 0;
}

.channel-row-name {
  flex: 1;
  font-size: 0.875rem;
  color: var(--text-normal);
}

.settings-group {
  margin-top: 12px;
}

.settings-group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 8px;
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-faint);
  border-bottom: 1px solid var(--border);
  margin-bottom: 4px;
}

.settings-group-name {
  flex: 1;
}

.channel-row-actions {
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.1s;
}

.channel-row:hover .channel-row-actions,
.settings-group-header:hover .channel-row-actions {
  opacity: 1;
}

.channel-action-btn {
  width: 24px;
  height: 24px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 3px;
  background: transparent;
  color: var(--text-faint);
  cursor: pointer;
}

.channel-action-btn:hover {
  color: var(--text-normal);
  background: var(--bg-modifier-hover);
  box-shadow: none;
}

.channel-action-btn:disabled {
  opacity: 0.2;
  cursor: default;
}

.channel-delete-btn {
  width: 24px;
  height: 24px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 3px;
  background: transparent;
  color: var(--text-faint);
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.1s;
}

.channel-row:hover .channel-delete-btn,
.settings-group-header:hover .channel-delete-btn {
  opacity: 1;
}

.channel-delete-btn:hover {
  color: var(--danger);
  background: rgba(208, 80, 80, 0.1);
  box-shadow: none;
}
</style>
