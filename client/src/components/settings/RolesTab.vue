<template>
  <SettingsBody class="split-view">
    <div class="split-list">
      <div class="card-title">Roles</div>
      <div class="input-row" style="margin-top: 8px;">
        <BaseInput v-model="newRoleName" placeholder="Nouveau role..." @keydown.enter="createRole" />
        <button class="btn-sq" @click="createRole" :disabled="!newRoleName.trim()">+</button>
      </div>

      <!-- Protected roles (above my hierarchy) — not editable/draggable -->
      <div v-if="protectedRoles.length" class="role-list">
        <div
          v-for="role in protectedRoles"
          :key="role.id"
          class="item-row protected"
          :class="{ active: editingRole?.id === role.id }"
          @click="editRole(role)"
        >
          <span class="role-handle-slot"></span>
          <div class="dot" :style="`background:${role.color || 'var(--text-muted)'}`"></div>
          <span class="item-name">{{ role.name }}</span>
          <span class="role-action-slot"><Lock :size="12" class="lock-icon" /></span>
        </div>
      </div>

      <!-- Editable custom roles — draggable -->
      <VueDraggable
        v-model="editableRoles"
        class="role-list"
        :animation="150"
        handle=".role-drag-handle"
        @end="onRoleDragEnd"
      >
        <div v-for="role in editableRoles" :key="role.id">
          <div
            class="item-row"
            :class="{ active: editingRole?.id === role.id }"
            @click="editRole(role)"
          >
            <span class="role-handle-slot"><GripVertical :size="12" class="role-drag-handle" /></span>
            <div class="dot" :style="`background:${role.color || 'var(--text-muted)'}`"></div>
            <span class="item-name">{{ role.name }}</span>
            <span class="role-action-slot">
              <button
                class="btn-icon-danger"
                @click.stop="deleteRole(role.id)"
                title="Supprimer"
              >
                <Trash2 :size="14" />
              </button>
            </span>
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
        <span class="role-handle-slot"></span>
        <div class="dot" :style="`background: var(--text-faint)`"></div>
        <span class="item-name">Everyone</span>
        <span class="role-action-slot"></span>
      </div>
    </div>

    <!-- Role editor panel -->
    <div class="split-detail" v-if="editingRole">
      <div class="card-title">
        {{ editingRole.id === 2 ? 'Permissions par defaut' : editingRole.name }}
        <Lock v-if="!canEditEditingRole" :size="12" class="lock-icon" style="margin-left: 6px;" />
      </div>

      <p v-if="!canEditEditingRole && editingRole.id !== 2" class="card-hint" style="margin-bottom: 12px;">
        Ce role est au-dessus ou au meme niveau que les tiens — lecture seule.
      </p>

      <p v-else-if="canEditEditingRole && !isAdmin" class="card-hint" style="margin-bottom: 12px;">
        <Lock :size="11" class="lock-icon" style="vertical-align: -1px;" />
        Tu ne peux activer que les permissions que tu possedes toi-meme.
      </p>

      <!-- Custom roles: name + color editing -->
      <div v-if="editingRole.id > 2" class="input-row" style="margin-bottom: 16px;">
        <BaseInput v-model="editingRole.name" placeholder="Nom" :disabled="!canEditEditingRole" />
        <label class="color-picker" :class="{ disabled: !canEditEditingRole }">
          <input type="color" v-model="editingRole.color" :disabled="!canEditEditingRole" />
          <div class="color-preview" :style="`background:${editingRole.color || 'var(--text-muted)'}`"></div>
        </label>
        <button class="btn-color-reset" :disabled="!editingRole.color || !canEditEditingRole" @click="editingRole.color = null" title="Retirer la couleur">
          <X :size="12" />
        </button>
      </div>

      <p v-if="editingRole.id === 2" class="card-hint" style="margin-bottom: 12px;">
        Ces permissions s'appliquent a tous les membres du serveur.
      </p>

      <!-- Permission editing (all roles except Owner) -->
      <div v-for="group in permissionGroups" :key="group.label" class="perm-section">
        <div class="perm-section-title">{{ group.label }}</div>
        <div v-for="p in group.perms" :key="p.flag" class="perm-row" :class="{ 'perm-locked': !canTogglePerm(p.flag) }">
          <span class="perm-label">
            {{ p.name }}
            <Lock v-if="canEditEditingRole && !canTogglePerm(p.flag)" :size="10" class="perm-lock-icon" />
          </span>
          <PermToggle
            :model-value="(editingRole.permissions & p.flag) !== 0 ? 'allow' : 'deny'"
            mode="dual"
            :disabled="!canEditEditingRole || !canTogglePerm(p.flag)"
            @update:model-value="togglePerm(p.flag)"
          />
        </div>
      </div>

      <div v-if="canEditEditingRole" class="detail-actions">
        <SaveButton :loading="savingRole" :saved="roleSaved" @click="saveRole" />
      </div>
    </div>
    <div class="split-detail empty" v-else>
      <div class="empty-state">
        <ShieldCheck :size="32" />
        <p>Selectionne un role</p>
      </div>
    </div>
  </SettingsBody>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { X, Trash2, ShieldCheck, Lock, GripVertical } from "lucide-vue-next";
import SaveButton from "../ui/SaveButton.vue";
import BaseInput from "../ui/BaseInput.vue";
import PermToggle from "../ui/PermToggle.vue";
import SettingsBody from "../ui/SettingsBody.vue";
import { VueDraggable } from "vue-draggable-plus";
import { activeState, activeServer } from "../../store";
import { api } from "../../api";
import * as perms from "../../permissions";

const roles = ref<{ id: number; name: string; permissions: number; color: string | null; position: number }[]>([]);
const newRoleName = ref("");
const editingRole = ref<{ id: number; name: string; permissions: number; color: string | null } | null>(null);
const savingRole = ref(false);
const roleSaved = ref(false);

const OWNER_USER_ID = 1;

/** Lowest position number among my roles (owner bypasses → -Infinity). */
const myMaxPosition = computed(() => {
  const st = activeState();
  if (!st?.user) return Infinity; // no user → everything protected
  if (st.user.id === OWNER_USER_ID) return -Infinity; // owner bypass
  const myRoleIds = st.userRoles.get(st.user.id) ?? [];
  const myPositions = roles.value
    .filter(r => myRoleIds.includes(r.id))
    .map(r => r.position);
  return myPositions.length ? Math.min(...myPositions) : Infinity;
});

function canActOnRole(role: { id: number; position: number }): boolean {
  if (role.id === 1) return false; // Owner role: immutable
  return role.position > myMaxPosition.value;
}

const customRoles = computed(() => roles.value.filter(r => r.id > 2).sort((a, b) => a.position - b.position));

const protectedRoles = computed(() => customRoles.value.filter(r => !canActOnRole(r)));

const editableRoles = computed({
  get: () => customRoles.value.filter(r => canActOnRole(r)),
  set: (val) => {
    // Merge back: protected roles keep their order, editable roles get new order
    const protectedIds = new Set(protectedRoles.value.map(r => r.id));
    const fixed = roles.value.filter(r => r.id <= 2 || protectedIds.has(r.id));
    roles.value = [...fixed, ...val];
  },
});

const membreRole = computed(() => roles.value.find(r => r.id === 2) ?? null);
const canEditEditingRole = computed(() => {
  if (!editingRole.value) return false;
  const full = roles.value.find(r => r.id === editingRole.value!.id);
  if (!full) return false;
  return canActOnRole(full);
});

/** True if the current user holds ADMINISTRATOR (owner is implicit admin). */
const isAdmin = computed(() => {
  const st = activeState();
  if (!st?.user) return false;
  if (st.user.id === OWNER_USER_ID) return true;
  return perms.has(st.permissions, perms.ADMINISTRATOR);
});

/**
 * Mirror of `require_permission_subset` on the server: a non-admin user can
 * only toggle a permission if they hold it themselves. Bits already set in
 * the role can stay (a higher-up may have set them) but cannot be removed
 * and re-added by a non-holder.
 */
function canTogglePerm(flag: number): boolean {
  if (isAdmin.value) return true;
  const st = activeState();
  if (!st) return false;
  if ((st.permissions & flag) !== 0) return true;
  if (editingRole.value && (editingRole.value.permissions & flag) !== 0) return true;
  return false;
}

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
  // Full order: protected first (unchanged), then editable in new order
  const ids = [...protectedRoles.value.map(r => r.id), ...editableRoles.value.map(r => r.id)];
  st.roles = [...roles.value];
  await api.reorderRoles(s.url, s.token, ids);
}
</script>

<style scoped>
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

.role-list + .role-list { margin-top: 3px; }

.role-list .item-row,
.split-list > .item-row {
  padding: 5px 8px;
}

.role-handle-slot,
.role-action-slot {
  width: 16px;
  height: 16px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.role-drag-handle {
  color: var(--text-faint);
  opacity: 0.3;
  cursor: grab;
  transition: opacity 0.1s;
}
.item-row:hover .role-drag-handle { opacity: 0.7; }
.role-drag-handle:active { cursor: grabbing; }

.lock-icon {
  color: var(--text-faint);
  opacity: 0.5;
}

.item-row.protected {
  opacity: 0.65;
}

.role-list .btn-icon-danger {
  width: 16px;
  height: 16px;
}

.color-picker.disabled {
  opacity: 0.4;
  pointer-events: none;
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
  display: flex;
  align-items: center;
  gap: 6px;
}

.perm-lock-icon {
  color: var(--text-faint);
  opacity: 0.6;
}

.perm-row.perm-locked .perm-label {
  color: var(--text-muted);
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
