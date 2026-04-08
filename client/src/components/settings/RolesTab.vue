<template>
  <div class="settings-body split-view">
    <div class="split-list">
      <div class="card-title">Roles</div>
      <div class="input-row" style="margin-top: 8px;">
        <input v-model="newRoleName" type="text" placeholder="Nouveau role..." @keydown.enter="createRole" />
        <button class="btn-sq" @click="createRole" :disabled="!newRoleName.trim()">+</button>
      </div>

      <VueDraggable
        v-model="customRoles"
        class="role-list"
        :animation="150"
        @end="onRoleDragEnd"
      >
        <div v-for="role in customRoles" :key="role.id">
          <div
            class="item-row"
            :class="{ active: editingRole?.id === role.id }"
            @click="editRole(role)"
          >
            <GripVertical :size="12" class="role-drag-handle" />
            <div class="dot" :style="`background:${role.color || 'var(--text-muted)'}`"></div>
            <span class="item-name">{{ role.name }}</span>
            <button
              class="btn-icon-danger"
              @click.stop="deleteRole(role.id)"
              title="Supprimer"
            >
              <Trash2 :size="14" />
            </button>
          </div>
        </div>
      </VueDraggable>

      <!-- Membre role (always at bottom, not draggable) -->
      <div v-if="membreRole" class="role-separator"></div>
      <div
        v-if="membreRole"
        class="item-row"
        :class="{ active: editingRole?.id === 2 }"
        @click="editRole({ ...membreRole })"
      >
        <div class="dot" :style="`background: var(--text-faint)`"></div>
        <span class="item-name">Permissions par defaut</span>
        <Lock :size="12" class="lock-icon" />
      </div>
    </div>

    <!-- Role editor panel -->
    <div class="split-detail" v-if="editingRole">
      <div class="card-title">{{ editingRole.id === 2 ? 'Permissions par defaut' : editingRole.name }}</div>

      <!-- Custom roles: name + color editing -->
      <div v-if="editingRole.id > 2" class="input-row" style="margin-bottom: 16px;">
        <input v-model="editingRole.name" type="text" placeholder="Nom" />
        <label class="color-picker">
          <input type="color" v-model="editingRole.color" />
          <div class="color-preview" :style="`background:${editingRole.color || 'var(--text-muted)'}`"></div>
        </label>
        <button class="btn-color-reset" :disabled="!editingRole.color" @click="editingRole.color = null" title="Retirer la couleur">
          <X :size="12" />
        </button>
      </div>

      <p v-if="editingRole.id === 2" class="card-hint" style="margin-bottom: 12px;">
        Ces permissions s'appliquent a tous les membres du serveur.
      </p>

      <!-- Permission editing (all roles except Owner) -->
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

      <div class="detail-actions">
        <SaveButton :loading="savingRole" :saved="roleSaved" @click="saveRole" />
      </div>
    </div>
    <div class="split-detail empty" v-else>
      <div class="empty-state">
        <ShieldCheck :size="32" />
        <p>Selectionne un role</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { X, Trash2, ShieldCheck, Lock, GripVertical } from "lucide-vue-next";
import SaveButton from "../SaveButton.vue";
import { VueDraggable } from "vue-draggable-plus";
import { activeState, activeServer } from "../../store";
import { api } from "../../api";
import * as perms from "../../permissions";

const roles = ref<{ id: number; name: string; permissions: number; color: string | null; position: number }[]>([]);
const newRoleName = ref("");
const editingRole = ref<{ id: number; name: string; permissions: number; color: string | null } | null>(null);
const savingRole = ref(false);
const roleSaved = ref(false);

const customRoles = computed({
  get: () => roles.value.filter(r => r.id > 2),
  set: (val) => {
    const fixed = roles.value.filter(r => r.id <= 2);
    roles.value = [...fixed, ...val];
  },
});

const membreRole = computed(() => roles.value.find(r => r.id === 2) ?? null);

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
      { name: "Deplacer des membres (kick)", flag: perms.MOVE_MEMBERS },
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
    body: JSON.stringify({ name: newRoleName.value.trim(), permissions: 0 }),
  });
  if (res.ok) {
    const role = await res.json();
    roles.value.push(role);
    newRoleName.value = "";
    editRole(role);
  }
}

function editRole(role: typeof roles.value[0]) {
  editingRole.value = { id: role.id, name: role.name, permissions: role.permissions, color: role.color };
}

function togglePerm(flag: number) {
  if (!editingRole.value) return;
  editingRole.value.permissions ^= flag;
}

async function saveRole() {
  const s = activeServer();
  const r = editingRole.value;
  if (!s || !r) return;

  savingRole.value = true;
  try {
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
    roleSaved.value = true;
    setTimeout(() => (roleSaved.value = false), 2500);
  } finally {
    savingRole.value = false;
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
  const custom = customRoles.value;
  custom.forEach((r, i) => r.position = i);
  st.roles = [...roles.value];
  await api.reorderRoles(s.url, s.token, custom.map((r) => r.id));
}
</script>

<style scoped>
.settings-body {
  padding: 0 24px 24px;
  overflow-y: auto;
  flex: 1;
}

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
  padding: 16px 16px 0;
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

.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
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
.item-row:hover .btn-icon-danger { opacity: 1; }
.btn-icon-danger:hover { color: var(--danger); background: var(--danger-bg); box-shadow: none; }

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

.lock-icon {
  color: var(--text-faint);
  opacity: 0.5;
}

.role-separator {
  height: 1px;
  background: var(--border);
  margin: 8px 4px;
}

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

.btn-color-reset {
  width: 28px;
  height: 28px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: var(--bg-tertiary);
  color: var(--text-faint);
  cursor: pointer;
  flex-shrink: 0;
  align-self: center;
}
.btn-color-reset:not(:disabled):hover { color: var(--text-normal); background: var(--bg-modifier-hover); box-shadow: none; }
.btn-color-reset:disabled { opacity: 0.3; cursor: not-allowed; }

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
  position: sticky;
  bottom: 0;
  margin-top: 16px;
  padding: 12px 0;
  border-top: 1px solid var(--border);
  background: var(--bg-secondary);
  z-index: 1;
}

.item-row.sortable-chosen { background: var(--bg-modifier-active); border-radius: 6px; }
.item-row.sortable-ghost { opacity: 0.3; }
</style>
