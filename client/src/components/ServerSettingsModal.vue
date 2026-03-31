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
          <div class="card">
            <div class="card-title">Display name</div>
            <p class="card-hint">Ton nom visible sur ce serveur.</p>
            <div class="input-row">
              <input v-model="displayName" type="text" placeholder="Mon pseudo" maxlength="32" />
              <button class="btn-sm" @click="saveDisplayName" :disabled="saving">
                {{ saving ? '...' : 'Sauvegarder' }}
              </button>
            </div>
            <p class="toast-success" v-if="saved">Sauvegarde !</p>
          </div>
        </div>


        <!-- Roles -->
        <div v-if="activeTab === 'roles'" class="settings-body split-view">
          <div class="split-list">
            <div class="card-title">Roles</div>
            <div class="input-row" style="margin-top: 8px;">
              <input v-model="newRoleName" type="text" placeholder="Nouveau role..." @keydown.enter="createRole" />
              <button class="btn-sq" @click="createRole" :disabled="!newRoleName.trim()">+</button>
            </div>

            <VueDraggable
              v-model="roles"
              class="role-list"
              :animation="150"
              @end="onRoleDragEnd"
            >
              <div v-for="role in roles" :key="role.id">
                <div
                  class="item-row"
                  :class="{ active: editingRole?.id === role.id }"
                  @click="editRole(role)"
                >
                  <GripVertical :size="12" class="role-drag-handle" />
                  <div class="dot" :style="`background:${role.color || 'var(--text-muted)'}`"></div>
                  <span class="item-name">{{ role.name }}</span>
                  <div v-if="role.id <= 2" class="lock-wrapper">
                    <Lock :size="12" class="lock-icon" />
                  </div>
                  <button
                    v-else
                    class="btn-icon-danger"
                    @click.stop="deleteRole(role.id)"
                    title="Supprimer"
                  >
                    <Trash2 :size="14" />
                  </button>
                </div>
              </div>
            </VueDraggable>
            <p v-if="!roles.length" class="card-hint" style="margin-top: 8px;">Aucun role.</p>
          </div>

          <!-- Role editor panel -->
          <div class="split-detail" v-if="editingRole">
            <div class="card-title">{{ editingRole.name }}</div>

            <div class="input-row" style="margin-bottom: 16px;">
              <input v-model="editingRole.name" type="text" placeholder="Nom" />
              <label class="color-picker">
                <input type="color" v-model="editingRole.color" />
                <div class="color-preview" :style="`background:${editingRole.color}`"></div>
              </label>
            </div>

            <!-- Admin: no permission editing -->
            <div v-if="editingRole.id === 1" class="admin-notice">
              <ShieldCheck :size="28" />
              <p>Ce role possede toutes les permissions.</p>
              <p class="card-hint">Le role administrateur ne peut pas etre modifie.</p>
            </div>

            <!-- Other roles: full permission editing -->
            <template v-else>
              <div v-for="group in permissionGroups" :key="group.label" class="perm-section">
                <div class="perm-section-title">{{ group.label }}</div>
                <div v-for="p in group.perms" :key="p.flag" class="perm-row">
                  <span class="perm-label">{{ p.name }}</span>
                  <div
                    class="toggle"
                    :class="{ on: (editingRole.permissions & p.flag) !== 0 }"
                    @click="togglePerm(p.flag)"
                  >
                    <div class="toggle-knob"></div>
                  </div>
                </div>
              </div>
            </template>

            <div class="detail-actions">
              <button class="btn-sm" @click="saveRole">Sauvegarder</button>
            </div>
          </div>
          <div class="split-detail empty" v-else>
            <div class="empty-state">
              <ShieldCheck :size="32" />
              <p>Selectionne un role</p>
            </div>
          </div>
        </div>

        <!-- Moderation -->
        <div v-if="activeTab === 'moderation'" class="settings-body split-view">
          <div class="split-list">
            <div class="card-title">Membres</div>
            <div class="item-list" style="margin-top: 8px;">
              <div
                v-for="user in allUsers"
                :key="user.id"
                class="item-row"
                :class="{ active: selectedUserId === user.id }"
                @click="selectUser(user.id)"
              >
                <span class="item-name">{{ user.display_name }}</span>
                <button
                  v-if="user.id !== state?.user?.id"
                  class="btn-icon-danger"
                  @click.stop="kickUser(user.id)"
                  title="Kick"
                >
                  <UserX :size="14" />
                </button>
              </div>
            </div>
          </div>

          <!-- User role assignment panel -->
          <div class="split-detail" v-if="selectedUserId">
            <div class="card-title">Roles de {{ allUsers.find(u => u.id === selectedUserId)?.display_name }}</div>
            <div class="item-list" style="margin-top: 8px;">
              <div v-for="role in roles" :key="role.id" class="perm-row">
                <div class="dot" :style="`background:${role.color || 'var(--text-muted)'}`"></div>
                <span class="perm-label">{{ role.name }}</span>
                <div
                  class="toggle"
                  :class="{ on: selectedUserRoleIds().includes(role.id) }"
                  @click="toggleUserRole(role.id)"
                >
                  <div class="toggle-knob"></div>
                </div>
              </div>
            </div>
          </div>
          <div class="split-detail empty" v-else>
            <div class="empty-state">
              <UserRound :size="32" />
              <p>Selectionne un membre</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { X, Trash2, ShieldCheck, UserX, UserRound, Gavel, Lock, GripVertical } from "lucide-vue-next";
import { VueDraggable } from "vue-draggable-plus";
import { store, activeState, activeServer } from "../store";
import { api } from "../api";
import * as perms from "../permissions";

const state = computed(() => activeState());
const activeTab = ref(store.serverSettingsTab || "profile");

const allTabs = [
  { id: "profile", label: "Profil", icon: UserRound, permission: 0 },
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

// Roles
const roles = ref<{ id: number; name: string; permissions: number; color: string | null; position: number }[]>([]);
const newRoleName = ref("");
const editingRole = ref<{ id: number; name: string; permissions: number; color: string | null } | null>(null);

const permissionGroups = [
  {
    label: "General",
    perms: [
      { name: "Administrateur", flag: perms.ADMINISTRATOR },
      { name: "Gerer les channels", flag: perms.MANAGE_CHANNELS },
      { name: "Gerer les roles", flag: perms.MANAGE_ROLES },
      { name: "Gerer le serveur", flag: perms.MANAGE_SERVER },
      { name: "Kick", flag: perms.KICK_MEMBERS },
      { name: "Ban", flag: perms.BAN_MEMBERS },
      { name: "Creer des invitations", flag: perms.CREATE_INVITE },
      { name: "Changer son pseudo", flag: perms.CHANGE_NICKNAME },
      { name: "Gerer les pseudos", flag: perms.MANAGE_NICKNAMES },
    ],
  },
  {
    label: "Texte",
    perms: [
      { name: "Voir les channels", flag: perms.VIEW_CHANNELS },
      { name: "Envoyer des messages", flag: perms.SEND_MESSAGES },
      { name: "Gerer les messages", flag: perms.MANAGE_MESSAGES },
      { name: "Historique des messages", flag: perms.READ_MESSAGE_HISTORY },
      { name: "Joindre des fichiers", flag: perms.ATTACH_FILES },
      { name: "Mentionner @everyone", flag: perms.MENTION_EVERYONE },
      { name: "Reactions", flag: perms.ADD_REACTIONS },
      { name: "Liens embarques", flag: perms.EMBED_LINKS },
    ],
  },
  {
    label: "Vocal",
    perms: [
      { name: "Se connecter", flag: perms.CONNECT },
      { name: "Parler", flag: perms.SPEAK },
      { name: "Streamer", flag: perms.STREAM },
      { name: "Mute des membres", flag: perms.MUTE_MEMBERS },
      { name: "Deafen des membres", flag: perms.DEAFEN_MEMBERS },
      { name: "Deplacer des membres", flag: perms.MOVE_MEMBERS },
      { name: "Activite vocale", flag: perms.USE_VOICE_ACTIVITY },
      { name: "Priorite vocale", flag: perms.PRIORITY_SPEAKER },
    ],
  },
];

onMounted(async () => {
  const s = activeServer();
  if (!s) return;
  try {
    roles.value = await api.listRoles(s.url, s.token);
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
    body: JSON.stringify({ name: newRoleName.value.trim(), permissions: 0, position: roles.value.length }),
  });
  if (res.ok) {
    const role = await res.json();
    roles.value.push(role);
    newRoleName.value = "";
    editRole(role);
  }
}

function editRole(role: typeof roles.value[0]) {
  editingRole.value = { id: role.id, name: role.name, permissions: role.permissions, color: role.color || "#99aab5" };
}

function togglePerm(flag: number) {
  if (!editingRole.value) return;
  editingRole.value.permissions ^= flag;
}

async function saveRole() {
  const s = activeServer();
  const r = editingRole.value;
  if (!s || !r) return;

  await api.updateRole(s.url, s.token, r.id, {
    name: r.name,
    permissions: r.permissions,
    color: r.color,
  });

  const idx = roles.value.findIndex((x) => x.id === r.id);
  if (idx >= 0) {
    roles.value[idx].name = r.name;
    roles.value[idx].permissions = r.permissions;
    roles.value[idx].color = r.color;
  }
}

async function deleteRole(id: number) {
  const s = activeServer();
  if (!s) return;

  await api.deleteRole(s.url, s.token, id);
  roles.value = roles.value.filter((r) => r.id !== id);
  if (editingRole.value?.id === id) editingRole.value = null;
}

async function onRoleDragEnd() {
  const s = activeServer();
  const st = activeState();
  if (!s || !st) return;
  // Update positions locally
  roles.value.forEach((r, i) => r.position = i);
  st.roles = [...roles.value];
  await api.reorderRoles(s.url, s.token, roles.value.map((r) => r.id));
}

// Moderation
const allUsers = computed(() => {
  if (!state.value) return [];
  return [...state.value.users.values()];
});

const selectedUserId = ref<number | null>(null);

function selectedUserRoleIds(): number[] {
  if (!selectedUserId.value) return [];
  return state.value?.userRoles.get(selectedUserId.value) ?? [];
}

function selectUser(userId: number) {
  selectedUserId.value = userId;
}

async function toggleUserRole(roleId: number) {
  const s = activeServer();
  const uid = selectedUserId.value;
  if (!s || !uid) return;

  const has = selectedUserRoleIds().includes(roleId);
  if (has) {
    await api.removeRole(s.url, s.token, roleId, uid);
  } else {
    await api.assignRole(s.url, s.token, roleId, uid);
  }
}

async function kickUser(userId: number) {
  const s = activeServer();
  const st = activeState();
  if (!s || !st) return;

  await api.kickUser(s.url, s.token, userId);
  st.users.delete(userId);
  if (selectedUserId.value === userId) selectedUserId.value = null;
}

function close() {
  store.showServerSettingsModal = false;
}
</script>

<style scoped>
/* ── Modal shell ── */
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
  border-radius: 12px;
  width: 1040px;
  height: 700px;
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

/* ── Content area ── */
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

/* ── Split view (list + detail) ── */
.split-view {
  display: flex;
  gap: 16px;
  min-height: 0;
}

.split-list {
  width: 220px;
  flex-shrink: 0;
  overflow-y: auto;
  overflow-x: hidden;
}

.split-detail {
  flex: 1;
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 16px;
  overflow-y: auto;
}

.split-detail.empty {
  display: flex;
  align-items: center;
  justify-content: center;
}

.empty-state {
  text-align: center;
  color: var(--text-faint);
}
.empty-state p { margin-top: 8px; font-size: 0.8125rem; }

/* ── Cards & titles ── */
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

/* ── Inputs ── */
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

.btn-sm {
  width: auto;
  padding: 8px 16px;
  margin: 0;
  font-size: 0.8125rem;
  border-radius: 6px;
  flex-shrink: 0;
}

.btn-sq {
  width: 34px;
  height: 34px;
  padding: 0;
  margin: 0;
  font-size: 1rem;
  border-radius: 6px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.toast-success {
  font-size: 0.75rem;
  color: var(--green);
  margin-top: 6px;
}

/* ── Item list (channels, roles, users) ── */
.item-list {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.item-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 8px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.1s;
}
.item-row:hover { background: var(--bg-modifier-hover); }
.item-row.active { background: var(--bg-modifier-active); }

.item-icon { color: var(--text-faint); flex-shrink: 0; }
.item-name { flex: 1; font-size: 0.8125rem; color: var(--text-normal); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.btn-icon-danger {
  width: 24px;
  height: 24px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: transparent;
  color: var(--text-faint);
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.1s;
  flex-shrink: 0;
}
.item-row:hover .btn-icon-danger,
.group-header:hover .btn-icon-danger { opacity: 1; }
.btn-icon-danger:hover { color: var(--danger); background: rgba(208, 80, 80, 0.1); box-shadow: none; }

/* ── Groups ── */
.group-section {
  margin-top: 10px;
  background: var(--bg-tertiary);
  border-radius: 6px;
  padding: 4px;
}

.group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px;
  font-size: 0.6875rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-faint);
  margin-bottom: 2px;
}

/* ── Dot ── */
.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

/* ── Color picker ── */
.color-picker {
  position: relative;
  cursor: pointer;
  flex-shrink: 0;
}
.color-picker input[type="color"] {
  position: absolute;
  inset: 0;
  opacity: 0;
  cursor: pointer;
  width: 100%;
  height: 100%;
}
.color-preview {
  width: 36px;
  height: 36px;
  border-radius: 6px;
  border: 2px solid var(--border);
}

/* ── Toggle switch ── */
.toggle {
  width: 36px;
  height: 20px;
  border-radius: 10px;
  background: var(--bg-tertiary);
  cursor: pointer;
  position: relative;
  transition: background 0.2s;
  flex-shrink: 0;
}
.toggle.on { background: var(--green); }

.toggle-knob {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #fff;
  position: absolute;
  top: 2px;
  left: 2px;
  transition: transform 0.2s;
}
.toggle.on .toggle-knob { transform: translateX(16px); }

/* ── Permission rows ── */
.perm-section { margin-bottom: 16px; }

.perm-section-title {
  font-size: 0.6875rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-faint);
  margin-bottom: 6px;
  padding-bottom: 4px;
  border-bottom: 1px solid var(--border);
}

.perm-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 0;
}

.perm-label {
  font-size: 0.8125rem;
  color: var(--text-normal);
  flex: 1;
}

.detail-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid var(--border);
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
  color: #fff;
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

/* ── Tristate buttons ── */
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
.tri-btn.active.allow { background: rgba(95, 173, 95, 0.2); color: var(--green); }
.tri-btn.active.deny { background: rgba(208, 80, 80, 0.2); color: var(--danger); }

/* ── List header ── */
.list-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.list-header-actions {
  display: flex;
  gap: 4px;
}

.btn-icon-add {
  position: relative;
  width: 28px;
  height: 28px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: var(--bg-tertiary);
  color: var(--text-muted);
  cursor: pointer;
  transition: background 0.1s, color 0.1s;
}
.btn-icon-add:hover { background: var(--bg-modifier-hover); color: var(--text-normal); box-shadow: none; }

.plus-badge {
  position: absolute;
  bottom: 1px;
  right: 1px;
  background: var(--green);
  color: #fff;
  border-radius: 50%;
  padding: 1px;
}

.btn-icon-sm {
  width: 22px;
  height: 22px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  background: transparent;
  color: var(--text-faint);
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.1s;
  flex-shrink: 0;
}
.group-header:hover .btn-icon-sm,
.item-row:hover .btn-icon-sm { opacity: 1; }
.btn-icon-sm:hover { color: var(--text-normal); background: var(--bg-modifier-hover); box-shadow: none; }

.role-list {
  display: flex;
  flex-direction: column;
  gap: 3px;
  margin-top: 12px;
}

.role-list .item-row {
  padding: 5px 8px;
}

.role-drag-handle {
  color: var(--text-faint);
  opacity: 0.3;
  cursor: grab;
  flex-shrink: 0;
  transition: opacity 0.1s;
}
.item-row:hover .role-drag-handle { opacity: 0.7; }
.role-drag-handle:active { cursor: grabbing; }

.lock-wrapper {
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.lock-icon {
  color: var(--text-faint);
  opacity: 0.5;
}

.admin-notice {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 32px 16px;
  color: var(--accent);
  text-align: center;
}
.admin-notice p {
  font-size: 0.875rem;
  color: var(--text-normal);
  margin: 0;
}
.admin-notice .card-hint {
  color: var(--text-faint);
  margin: 0;
}

.inline-edit {
  flex: 1;
  padding: 2px 6px;
  border: 1px solid var(--accent);
  border-radius: 4px;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: inherit;
  font-family: inherit;
  font-weight: inherit;
  outline: none;
  min-width: 0;
}

/* ── Drag & drop ── */
.drag-zone { min-height: 8px; padding: 2px 0; }

.drag-handle, .group-drag-handle {
  color: var(--text-faint);
  opacity: 0.3;
  cursor: grab;
  flex-shrink: 0;
  transition: opacity 0.1s;
}
.item-row:hover .drag-handle,
.group-header:hover .group-drag-handle { opacity: 0.7; }
.drag-handle:active, .group-drag-handle:active { cursor: grabbing; }
.item-row.sortable-chosen { background: var(--bg-modifier-active); border-radius: 6px; }
.item-row.sortable-ghost { opacity: 0.3; }
</style>
