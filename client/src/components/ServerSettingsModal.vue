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

        <!-- Serveur -->
        <div v-if="activeTab === 'server'" class="settings-body">
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

        <!-- Profil -->
        <div v-if="activeTab === 'profile'" class="settings-body">
          <div class="card">
            <div class="card-title">Avatar</div>
            <p class="card-hint">Ta photo de profil sur ce serveur.</p>
            <div class="avatar-setting">
              <div class="avatar-preview" @click="serverAvatarInput?.click()">
                <img v-if="currentAvatarUrl" :src="currentAvatarUrl" />
                <span v-else class="avatar-placeholder">{{ (state?.user?.display_name || '?')[0]?.toUpperCase() }}</span>
                <div class="avatar-overlay">
                  <Camera :size="16" />
                </div>
              </div>
              <div class="avatar-actions">
                <button class="btn-sm" @click="serverAvatarInput?.click()">Changer</button>
                <button v-if="currentAvatarUrl" class="btn-sm btn-danger-outline" @click="removeServerAvatar">Supprimer</button>
              </div>
              <input ref="serverAvatarInput" type="file" accept="image/png,image/jpeg,image/gif,image/webp" hidden @change="onServerAvatarSelect" />
            </div>
          </div>

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

          <div class="card">
            <div class="card-title">Mot de passe</div>
            <p class="card-hint">Change ton mot de passe de connexion.</p>
            <div class="password-fields">
              <input v-model="currentPassword" type="password" placeholder="Mot de passe actuel" />
              <input v-model="newPassword" type="password" placeholder="Nouveau mot de passe" />
              <input v-model="confirmPassword" type="password" placeholder="Confirmer" @keydown.enter="changePassword" />
            </div>
            <div class="password-actions">
              <button class="btn-sm" @click="changePassword" :disabled="savingPassword || !currentPassword || !newPassword || newPassword !== confirmPassword">
                {{ savingPassword ? '...' : 'Changer' }}
              </button>
              <span v-if="newPassword && confirmPassword && newPassword !== confirmPassword" class="password-error">Les mots de passe ne correspondent pas</span>
              <span v-if="passwordError" class="password-error">{{ passwordError }}</span>
              <span v-if="passwordSaved" class="toast-success">Mot de passe change !</span>
            </div>
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
            <!-- Sub-tabs: Members / Banned -->
            <div class="mod-tabs">
              <button
                class="mod-tab"
                :class="{ active: modSubTab === 'members' }"
                @click="modSubTab = 'members'"
              >
                Membres
                <span class="mod-tab-count">{{ filteredUsers.length }}</span>
              </button>
              <button
                class="mod-tab"
                :class="{ active: modSubTab === 'banned' }"
                @click="modSubTab = 'banned'; loadBannedUsers()"
              >
                Bannis
                <span class="mod-tab-count" v-if="bannedUsers.length">{{ bannedUsers.length }}</span>
              </button>
            </div>

            <!-- Search -->
            <div class="mod-search" v-if="modSubTab === 'members'">
              <Search :size="14" class="mod-search-icon" />
              <input
                v-model="modSearch"
                type="text"
                placeholder="Rechercher un membre..."
                class="mod-search-input"
              />
            </div>

            <!-- Filter -->
            <div class="mod-filters" v-if="modSubTab === 'members'">
              <button
                class="mod-filter"
                :class="{ active: modFilter === 'all' }"
                @click="modFilter = 'all'"
              >Tous</button>
              <button
                class="mod-filter"
                :class="{ active: modFilter === 'online' }"
                @click="modFilter = 'online'"
              >En ligne</button>
              <button
                class="mod-filter"
                :class="{ active: modFilter === 'offline' }"
                @click="modFilter = 'offline'"
              >Hors ligne</button>
            </div>

            <!-- Members list -->
            <div class="item-list mod-list" v-if="modSubTab === 'members'">
              <div
                v-for="user in filteredUsers"
                :key="user.id"
                class="item-row mod-user-row"
                :class="{ active: selectedUserId === user.id }"
                @click="selectUser(user.id)"
              >
                <div class="mod-avatar-wrapper small">
                  <div class="mod-user-avatar-small">
                    <img v-if="userAvatarUrl(user)" :src="userAvatarUrl(user)!" />
                    <span v-else>{{ user.display_name[0]?.toUpperCase() }}</span>
                  </div>
                  <div class="mod-online-dot" :class="{ online: isOnline(user.id) }"></div>
                </div>
                <div class="mod-user-info">
                  <span class="mod-user-name">{{ user.display_name }}</span>
                  <span class="mod-user-sub" v-if="user.username">@{{ user.username }}</span>
                </div>
                <span
                  v-if="topRole(user.id)"
                  class="mod-top-role"
                  :style="`color: ${topRole(user.id)!.color || 'var(--text-muted)'}`"
                >{{ topRole(user.id)!.name }}</span>
              </div>
              <div v-if="!filteredUsers.length" class="mod-empty-list">
                Aucun membre trouve
              </div>
            </div>

            <!-- Banned list -->
            <div class="item-list mod-list" v-if="modSubTab === 'banned'">
              <div
                v-for="user in bannedUsers"
                :key="user.id"
                class="item-row mod-user-row"
                :class="{ active: selectedBannedId === user.id }"
                @click="selectedBannedId = user.id; selectedUserId = null"
              >
                <div class="mod-avatar-wrapper small">
                  <div class="mod-user-avatar-small banned">
                    <img v-if="bannedAvatarUrl(user)" :src="bannedAvatarUrl(user)!" />
                    <span v-else>{{ user.display_name[0]?.toUpperCase() }}</span>
                  </div>
                </div>
                <div class="mod-user-info">
                  <span class="mod-user-name">{{ user.display_name }}</span>
                  <span class="mod-user-sub">@{{ user.username }}</span>
                </div>
              </div>
              <div v-if="!bannedUsers.length" class="mod-empty-list">
                <ShieldCheck :size="20" />
                <span>Aucun utilisateur banni</span>
              </div>
            </div>
          </div>

          <!-- User detail panel -->
          <div class="split-detail" v-if="selectedUserId && modSubTab === 'members'">
            <div class="mod-detail-header">
              <div class="mod-avatar-wrapper large">
                <div class="mod-detail-avatar">
                  <img v-if="userAvatarUrl(selectedUser!)" :src="userAvatarUrl(selectedUser!)!" />
                  <span v-else class="mod-detail-avatar-letter">{{ selectedUser?.display_name[0]?.toUpperCase() }}</span>
                </div>
                <div class="mod-online-dot large" :class="{ online: isOnline(selectedUserId) }"></div>
              </div>
              <div class="mod-detail-names">
                <span class="mod-detail-display">{{ selectedUser?.display_name }}</span>
                <span class="mod-detail-username" v-if="selectedUser?.username">@{{ selectedUser.username }}</span>
              </div>
            </div>

            <div class="mod-detail-meta" v-if="selectedUser?.created_at">
              <Calendar :size="12" />
              <span>Membre depuis {{ formatDate(selectedUser.created_at) }}</span>
            </div>

            <div class="card-title" style="margin-top: 16px;">Roles</div>
            <div class="mod-role-badges">
              <span
                v-for="role in userRoleBadges(selectedUserId)"
                :key="role.id"
                class="mod-role-badge"
              >
                <div class="dot" :style="`background:${role.color || 'var(--text-muted)'}`"></div>
                {{ role.name }}
                <button v-if="canManageUser(selectedUserId)" class="mod-role-remove" @click="toggleUserRole(role.id)" title="Retirer">
                  <X :size="10" />
                </button>
              </span>
              <div class="mod-role-add-wrapper" v-if="canManageUser(selectedUserId) && availableRoles.length">
                <button class="mod-role-add" @click.stop="showRoleMenu = !showRoleMenu">
                  <Plus :size="12" />
                </button>
                <div v-if="showRoleMenu" class="mod-role-menu">
                  <div
                    v-for="role in availableRoles"
                    :key="role.id"
                    class="mod-role-menu-item"
                    @click="toggleUserRole(role.id); showRoleMenu = false"
                  >
                    <div class="dot" :style="`background:${role.color || 'var(--text-muted)'}`"></div>
                    {{ role.name }}
                  </div>
                </div>
              </div>
              <span v-if="!userRoleBadges(selectedUserId).length && !availableRoles.length" class="mod-no-roles">Aucun role</span>
            </div>

            <div class="card-title" style="margin-top: 16px;">Activite</div>
            <div class="mod-audit-placeholder">
              <ScrollText :size="20" />
              <span>L'historique d'activite sera disponible prochainement</span>
            </div>

            <!-- Ban action -->
            <div
              v-if="canManageUser(selectedUserId)"
              class="mod-danger-zone"
            >
              <button class="btn-sm mod-ban-btn" @click="showBanConfirm = true">
                <Ban :size="14" />
                Bannir {{ selectedUser?.display_name }}
              </button>
            </div>
          </div>

          <!-- Banned user detail panel -->
          <div class="split-detail" v-else-if="selectedBannedId && modSubTab === 'banned'">
            <div class="mod-detail-header">
              <div class="mod-avatar-wrapper large">
                <div class="mod-detail-avatar banned">
                  <img v-if="bannedAvatarUrl(selectedBanned!)" :src="bannedAvatarUrl(selectedBanned!)!" />
                  <span v-else class="mod-detail-avatar-letter">{{ selectedBanned?.display_name[0]?.toUpperCase() }}</span>
                </div>
              </div>
              <div class="mod-detail-names">
                <span class="mod-detail-display">{{ selectedBanned?.display_name }}</span>
                <span class="mod-detail-username">@{{ selectedBanned?.username }}</span>
              </div>
            </div>

            <div class="mod-detail-meta">
              <Ban :size="12" />
              <span>Banni le {{ formatTimestamp(selectedBanned!.banned_at) }}</span>
            </div>

            <div class="mod-danger-zone">
              <button class="btn-sm mod-unban-btn" @click="unbanUser(selectedBannedId)">
                <ShieldCheck :size="14" />
                Debannir {{ selectedBanned?.display_name }}
              </button>
            </div>
          </div>

          <div class="split-detail empty" v-else>
            <div class="empty-state">
              <UserRound :size="32" />
              <p>Selectionne un membre</p>
            </div>
          </div>
        </div>

        <!-- Ban confirmation dialog -->
        <Teleport to="body">
          <div v-if="showBanConfirm" class="modal-overlay ban-confirm-overlay" @click.self="showBanConfirm = false">
            <div class="ban-confirm-modal">
              <h3>Bannir {{ selectedUser?.display_name }} ?</h3>
              <p class="ban-confirm-desc">Cette action deconnectera l'utilisateur et l'empechera de se reconnecter.</p>
              <div class="ban-confirm-actions">
                <button class="btn-sm" style="background: transparent; color: var(--text-muted); border: 1px solid var(--border);" @click="showBanConfirm = false">Annuler</button>
                <button class="btn-sm mod-ban-btn" @click="confirmBan">
                  <Ban :size="14" />
                  Bannir
                </button>
              </div>
            </div>
          </div>
        </Teleport>

        <!-- Invitations -->
        <div v-if="activeTab === 'invites'" class="settings-body">
          <div class="card">
            <div class="card-title">Creer une invitation</div>
            <p class="card-hint">Genere un code pour inviter quelqu'un sur le serveur.</p>
            <div class="invite-create-row">
              <div class="invite-field">
                <label>Utilisations max</label>
                <input v-model.number="newInviteMaxUses" type="number" min="1" placeholder="Illimite" />
              </div>
              <div class="invite-field">
                <label>Expiration (heures)</label>
                <input v-model.number="newInviteExpireHours" type="number" min="1" placeholder="Jamais" />
              </div>
              <button class="btn-sm invite-create-btn" @click="createInvite">
                <TicketPlus :size="14" />
                Creer
              </button>
            </div>
          </div>

          <div class="card" v-if="invites.length">
            <div class="card-title">Invitations actives</div>
            <div class="invite-list">
              <div v-for="inv in invites" :key="inv.code" class="invite-row">
                <div class="invite-info">
                  <div class="invite-code-row">
                    <code class="invite-code">{{ inv.code }}</code>
                    <button class="invite-copy" @click="copyInvite(inv.code)" :title="copiedCode === inv.code ? 'Copie !' : 'Copier le code'">
                      <Check v-if="copiedCode === inv.code" :size="12" />
                      <Copy v-else :size="12" />
                    </button>
                    <button class="invite-copy" @click="copyInviteLink(inv.code)" :title="copiedLink === inv.code ? 'Copie !' : 'Copier le lien'">
                      <Check v-if="copiedLink === inv.code" :size="12" />
                      <Link2 v-else :size="12" />
                    </button>
                  </div>
                  <span class="invite-meta">
                    par {{ resolveUser(inv.created_by) }}
                    · {{ inv.uses }}{{ inv.max_uses ? `/${inv.max_uses}` : '' }} utilisations
                    <template v-if="inv.expires_at"> · {{ formatExpiry(inv.expires_at) }}</template>
                  </span>
                </div>
                <button class="invite-revoke" @click="revokeInvite(inv.code)" title="Revoquer">
                  <Trash2 :size="14" />
                </button>
              </div>
            </div>
          </div>
          <div v-else class="card">
            <div class="invite-empty">
              <TicketPlus :size="24" />
              <p>Aucune invitation</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { X, Trash2, ShieldCheck, Ban, UserRound, Gavel, Lock, GripVertical, Camera, Server, TicketPlus, Copy, Check, Link2, Search, Calendar, Plus, ScrollText } from "lucide-vue-next";
import { VueDraggable } from "vue-draggable-plus";
import { store, activeState, activeServer, persistServers, resolveUser } from "../store";
import { api, type Invite, type BannedUser } from "../api";
import * as perms from "../permissions";

const state = computed(() => activeState());
const activeTab = ref(store.serverSettingsTab || "profile");

const allTabs = [
  { id: "server", label: "Serveur", icon: Server, permission: perms.MANAGE_SERVER },
  { id: "profile", label: "Profil", icon: UserRound, permission: 0 },
  { id: "roles", label: "Roles", icon: ShieldCheck, permission: perms.MANAGE_ROLES },
  { id: "moderation", label: "Moderation", icon: Gavel, permission: perms.BAN_MEMBERS },
  { id: "invites", label: "Invitations", icon: TicketPlus, permission: perms.CREATE_INVITE },
];

const visibleTabs = computed(() => {
  const p = state.value?.permissions ?? 0;
  return allTabs.filter((t) => t.permission === 0 || perms.has(p, t.permission));
});

const activeTabLabel = computed(() => allTabs.find((t) => t.id === activeTab.value)?.label ?? "");

// Server
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
    // Update the saved server name locally
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

// Profile
const displayName = ref(state.value?.user?.display_name || "");
const saving = ref(false);
const saved = ref(false);
const serverAvatarInput = ref<HTMLInputElement>();

const currentAvatarUrl = computed(() => {
  const s = activeServer();
  const avatarPath = state.value?.user?.avatar_url;
  if (!s || !avatarPath) return null;
  return `${s.url}${avatarPath}`;
});

async function onServerAvatarSelect(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  const s = activeServer();
  const st = activeState();
  if (!s || !st) return;
  try {
    const user = await api.uploadAvatar(s.url, s.token, file);
    st.user = user;
    st.users.set(user.id, user);
  } catch {}
  input.value = "";
}

async function removeServerAvatar() {
  const s = activeServer();
  const st = activeState();
  if (!s || !st) return;
  try {
    await api.deleteAvatar(s.url, s.token);
    if (st.user) {
      st.user.avatar_url = null;
      st.users.set(st.user.id, { ...st.user });
    }
  } catch {}
}

// Password
const currentPassword = ref("");
const newPassword = ref("");
const confirmPassword = ref("");
const savingPassword = ref(false);
const passwordError = ref("");
const passwordSaved = ref(false);

async function changePassword() {
  const s = activeServer();
  if (!s || !currentPassword.value || !newPassword.value) return;
  if (newPassword.value !== confirmPassword.value) return;

  savingPassword.value = true;
  passwordError.value = "";
  passwordSaved.value = false;
  try {
    await api.changePassword(s.url, s.token, currentPassword.value, newPassword.value);
    passwordSaved.value = true;
    currentPassword.value = "";
    newPassword.value = "";
    confirmPassword.value = "";
    setTimeout(() => (passwordSaved.value = false), 3000);
  } catch (e: any) {
    if (e.message === "401") {
      passwordError.value = "Mot de passe actuel incorrect";
    } else {
      passwordError.value = "Erreur";
    }
  } finally {
    savingPassword.value = false;
  }
}

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

// Custom roles (exclude Owner ID=1 and Membre ID=2)
const customRoles = computed({
  get: () => roles.value.filter(r => r.id > 2),
  set: (val) => {
    // Keep Owner and Membre in place, replace custom roles
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
  loadInvites();
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
  // Update positions locally (only custom roles)
  const custom = customRoles.value;
  custom.forEach((r, i) => r.position = i);
  st.roles = [...roles.value];
  await api.reorderRoles(s.url, s.token, custom.map((r) => r.id));
}

// Moderation
const modSubTab = ref<"members" | "banned">("members");
const modSearch = ref("");
const modFilter = ref<"all" | "online" | "offline">("all");
const showBanConfirm = ref(false);
const bannedUsers = ref<BannedUser[]>([]);
const selectedBannedId = ref<number | null>(null);

const allUsers = computed(() => {
  if (!state.value) return [];
  return [...state.value.users.values()];
});

const filteredUsers = computed(() => {
  let users = allUsers.value;
  const search = modSearch.value.toLowerCase().trim();
  if (search) {
    users = users.filter(u =>
      u.display_name.toLowerCase().includes(search) ||
      (u.username && u.username.toLowerCase().includes(search))
    );
  }
  if (modFilter.value === "online") {
    users = users.filter(u => isOnline(u.id));
  } else if (modFilter.value === "offline") {
    users = users.filter(u => !isOnline(u.id));
  }
  return users;
});

function isOnline(userId: number): boolean {
  return state.value?.onlineUsers.has(userId) ?? false;
}

function getHighestPosition(userId: number): number {
  const roleIds = state.value?.userRoles.get(userId) ?? [];
  const positions = roles.value.filter(r => roleIds.includes(r.id)).map(r => r.position);
  return positions.length ? Math.min(...positions) : Infinity;
}

function canManageUser(targetId: number): boolean {
  const myId = state.value?.user?.id;
  if (!myId) return false;
  if (myId === 1) return targetId !== 1; // Owner can manage everyone except self
  if (targetId === 1) return false; // Nobody can manage owner
  if (myId === targetId) return false; // Can't manage self
  return getHighestPosition(myId) < getHighestPosition(targetId);
}

function userAvatarUrl(user: { avatar_url: string | null }): string | null {
  const s = activeServer();
  if (!s || !user.avatar_url) return null;
  return `${s.url}${user.avatar_url}`;
}

function bannedAvatarUrl(user: BannedUser): string | null {
  const s = activeServer();
  if (!s || !user.avatar_url) return null;
  return `${s.url}${user.avatar_url}`;
}

function userRoleBadges(userId: number) {
  const roleIds = state.value?.userRoles.get(userId) ?? [];
  return roles.value.filter(r => r.id > 2 && roleIds.includes(r.id));
}

function topRole(userId: number) {
  const badges = userRoleBadges(userId);
  return badges.length ? badges[0] : null;
}

const showRoleMenu = ref(false);

const availableRoles = computed(() => {
  if (!selectedUserId.value) return [];
  const assigned = selectedUserRoleIds();
  return roles.value.filter(r => r.id > 2 && !assigned.includes(r.id));
});

function formatDate(dateStr: string): string {
  try {
    const d = new Date(dateStr);
    return d.toLocaleDateString("fr-FR", { day: "numeric", month: "long", year: "numeric" });
  } catch {
    return dateStr;
  }
}

function formatTimestamp(ts: number): string {
  const d = new Date(ts * 1000);
  return d.toLocaleDateString("fr-FR", { day: "numeric", month: "long", year: "numeric" });
}

const selectedUserId = ref<number | null>(null);

const selectedUser = computed(() => {
  if (!selectedUserId.value) return null;
  return allUsers.value.find(u => u.id === selectedUserId.value) ?? null;
});

const selectedBanned = computed(() => {
  if (!selectedBannedId.value) return null;
  return bannedUsers.value.find(u => u.id === selectedBannedId.value) ?? null;
});

function selectedUserRoleIds(): number[] {
  if (!selectedUserId.value) return [];
  return state.value?.userRoles.get(selectedUserId.value) ?? [];
}

function selectUser(userId: number) {
  selectedUserId.value = userId;
  selectedBannedId.value = null;
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

async function confirmBan() {
  const s = activeServer();
  const uid = selectedUserId.value;
  if (!s || !uid) return;

  await api.banUser(s.url, s.token, uid);
  showBanConfirm.value = false;
  selectedUserId.value = null;
}

async function loadBannedUsers() {
  const s = activeServer();
  if (!s) return;
  try {
    bannedUsers.value = await api.listBanned(s.url, s.token);
  } catch {}
}

async function unbanUser(userId: number) {
  const s = activeServer();
  if (!s) return;
  await api.unbanUser(s.url, s.token, userId);
  bannedUsers.value = bannedUsers.value.filter(u => u.id !== userId);
  if (selectedBannedId.value === userId) selectedBannedId.value = null;
}

// Invites
const invites = ref<Invite[]>([]);
const newInviteMaxUses = ref<number | null>(null);
const newInviteExpireHours = ref<number | null>(null);

async function loadInvites() {
  const s = activeServer();
  if (!s) return;
  try {
    invites.value = await api.listInvites(s.url, s.token);
  } catch {}
}

async function createInvite() {
  const s = activeServer();
  if (!s) return;
  const expiresAt = newInviteExpireHours.value
    ? Math.floor(Date.now() / 1000) + newInviteExpireHours.value * 3600
    : null;
  const inv = await api.createInvite(s.url, s.token, {
    max_uses: newInviteMaxUses.value || null,
    expires_at: expiresAt,
  });
  invites.value.unshift(inv);
  newInviteMaxUses.value = null;
  newInviteExpireHours.value = null;
}

const copiedCode = ref("");
const copiedLink = ref("");

async function copyInvite(code: string) {
  await navigator.clipboard.writeText(code);
  copiedCode.value = code;
  setTimeout(() => { if (copiedCode.value === code) copiedCode.value = ""; }, 2000);
}

async function copyInviteLink(code: string) {
  const s = activeServer();
  if (!s) return;
  const link = `https://sorry.boudot.codes/#invite=${code}&server=${encodeURIComponent(s.url)}`;
  await navigator.clipboard.writeText(link);
  copiedLink.value = code;
  setTimeout(() => { if (copiedLink.value === code) copiedLink.value = ""; }, 2000);
}

async function revokeInvite(code: string) {
  const s = activeServer();
  if (!s) return;
  await api.deleteInvite(s.url, s.token, code);
  invites.value = invites.value.filter((i) => i.code !== code);
}

function formatExpiry(ts: number): string {
  const now = Date.now() / 1000;
  const diff = ts - now;
  if (diff <= 0) return "expire";
  if (diff < 3600) return `dans ${Math.ceil(diff / 60)} min`;
  if (diff < 86400) return `dans ${Math.ceil(diff / 3600)} h`;
  return `dans ${Math.ceil(diff / 86400)} j`;
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
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
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

.btn-danger-outline {
  background: transparent !important;
  color: var(--danger) !important;
  border: 1px solid var(--danger) !important;
}

.btn-danger-outline:hover {
  background: rgba(208, 80, 80, 0.15) !important;
  box-shadow: none !important;
}

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

.password-fields {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 10px;
}

.password-fields input {
  padding: 8px 10px;
  border-radius: 6px;
  border: none;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.875rem;
  font-family: inherit;
  outline: none;
}

.password-fields input::placeholder {
  color: var(--text-faint);
}

.password-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.password-error {
  font-size: 0.75rem;
  color: var(--danger);
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

.invite-create-row {
  display: flex;
  align-items: flex-end;
  gap: 10px;
}

.invite-field {
  flex: 1;
}

.invite-field label {
  display: block;
  font-size: 0.6875rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-faint);
  margin-bottom: 4px;
}

.invite-field input {
  width: 100%;
  padding: 8px 10px;
  border-radius: 6px;
  border: none;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.875rem;
  font-family: inherit;
  outline: none;
  -moz-appearance: textfield;
}

.invite-field input::placeholder { color: var(--text-faint); }
.invite-field input::-webkit-inner-spin-button,
.invite-field input::-webkit-outer-spin-button { -webkit-appearance: none; }

.invite-create-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  white-space: nowrap;
  flex-shrink: 0;
}

.invite-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 8px;
}

.invite-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 6px;
  background: var(--bg-tertiary);
}

.invite-info {
  flex: 1;
  min-width: 0;
}

.invite-code-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.invite-code {
  font-size: 0.9375rem;
  font-weight: 600;
  color: var(--accent);
  background: none;
  padding: 0;
  letter-spacing: 0.05em;
}

.invite-copy {
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
  border: none;
}
.invite-copy:hover { color: var(--text-normal); background: var(--bg-modifier-hover); box-shadow: none; }

.invite-meta {
  font-size: 0.6875rem;
  color: var(--text-faint);
  margin-top: 2px;
}

.invite-revoke {
  width: 28px;
  height: 28px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: transparent;
  color: var(--text-faint);
  cursor: pointer;
  border: none;
  flex-shrink: 0;
}
.invite-revoke:hover { color: var(--danger); background: rgba(208, 80, 80, 0.1); box-shadow: none; }

.invite-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 24px;
  color: var(--text-faint);
  font-size: 0.8125rem;
}

/* ── Item list (channels, roles, users) ── */
.item-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
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

.role-separator {
  height: 1px;
  background: var(--border);
  margin: 8px 4px;
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

/* ── Moderation ── */
.mod-tabs {
  display: flex;
  gap: 0;
  background: var(--bg-tertiary);
  border-radius: 8px;
  padding: 3px;
  margin-bottom: 10px;
}

.mod-tab {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 5px 8px;
  margin: 0;
  border-radius: 6px;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-muted);
  background: transparent;
  cursor: pointer;
  border: none;
  transition: background 0.15s, color 0.15s;
}

.mod-tab:hover { color: var(--text-normal); }
.mod-tab.active { background: var(--bg-primary); color: var(--text-normal); box-shadow: 0 1px 3px rgba(0,0,0,0.15); }

.mod-tab-count {
  font-size: 0.625rem;
  background: var(--bg-modifier-hover);
  padding: 1px 5px;
  border-radius: 8px;
  color: var(--text-faint);
}

.mod-tab.active .mod-tab-count {
  background: var(--accent);
  color: #fff;
}

.mod-search {
  position: relative;
  margin-bottom: 8px;
}

.mod-search-icon {
  position: absolute;
  left: 8px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--text-faint);
  pointer-events: none;
}

.mod-search-input {
  width: 100%;
  padding: 6px 8px 6px 28px;
  border-radius: 6px;
  border: none;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.8125rem;
  font-family: inherit;
  outline: none;
}

.mod-search-input::placeholder { color: var(--text-faint); }

.mod-filters {
  display: flex;
  gap: 4px;
  margin-bottom: 8px;
}

.mod-filter {
  padding: 3px 8px;
  border-radius: 10px;
  font-size: 0.6875rem;
  font-weight: 500;
  color: var(--text-faint);
  background: transparent;
  cursor: pointer;
  border: none;
  transition: background 0.1s, color 0.1s;
}

.mod-filter:hover { color: var(--text-muted); background: var(--bg-modifier-hover); box-shadow: none; }
.mod-filter.active { color: var(--text-normal); background: var(--bg-modifier-active); }

.mod-list {
  max-height: 100%;
  overflow-y: auto;
}

.mod-user-row {
  gap: 8px;
  padding: 6px 8px;
}

.mod-avatar-wrapper {
  position: relative;
  flex-shrink: 0;
}

.mod-avatar-wrapper.small {
  width: 28px;
  height: 28px;
}

.mod-avatar-wrapper.large {
  width: 56px;
  height: 56px;
}

.mod-user-avatar-small {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--text-muted);
  overflow: hidden;
}

.mod-user-avatar-small img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.mod-user-avatar-small.banned {
  opacity: 0.5;
}

.mod-online-dot {
  position: absolute;
  bottom: -1px;
  right: -1px;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--text-faint);
  border: 2px solid var(--bg-primary);
  transition: background 0.2s;
}

.mod-online-dot.online {
  background: var(--green);
}

.mod-online-dot.large {
  width: 14px;
  height: 14px;
  bottom: 0;
  right: 0;
  border-width: 3px;
}

.mod-user-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.mod-user-name {
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-normal);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mod-user-sub {
  font-size: 0.6875rem;
  color: var(--text-faint);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mod-top-role {
  font-size: 0.6875rem;
  font-weight: 500;
  flex-shrink: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 80px;
}

.mod-empty-list {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 20px 8px;
  color: var(--text-faint);
  font-size: 0.8125rem;
}

/* Detail panel */
.mod-detail-header {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 12px;
}

.mod-detail-avatar {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  background: var(--bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}

.mod-detail-avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.mod-detail-avatar.banned {
  opacity: 0.5;
}

.mod-detail-avatar-letter {
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--text-faint);
}

.mod-detail-names {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.mod-detail-display {
  font-size: 1rem;
  font-weight: 700;
  color: var(--header-primary);
}

.mod-detail-username {
  font-size: 0.8125rem;
  color: var(--text-faint);
}

.mod-detail-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.75rem;
  color: var(--text-faint);
  padding: 8px 10px;
  background: var(--bg-tertiary);
  border-radius: 6px;
}

.mod-role-badges {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 6px;
}

.mod-role-badge {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 3px 22px 3px 8px;
  border-radius: 10px;
  font-size: 0.6875rem;
  font-weight: 500;
  color: var(--text-normal);
  background: var(--bg-tertiary);
  position: relative;
}

.mod-role-remove {
  position: absolute;
  right: 4px;
  top: 50%;
  transform: translateY(-50%);
  width: 14px;
  height: 14px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: transparent;
  color: var(--text-faint);
  cursor: pointer;
  border: none;
  opacity: 0;
  transition: opacity 0.1s, color 0.1s;
}

.mod-role-badge:hover .mod-role-remove {
  opacity: 1;
}

.mod-role-remove:hover {
  color: var(--danger);
  box-shadow: none;
}

.mod-role-add-wrapper {
  position: relative;
}

.mod-role-add {
  width: 24px;
  height: 24px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: var(--bg-tertiary);
  color: var(--text-faint);
  cursor: pointer;
  border: 1px dashed var(--border);
  transition: color 0.1s, border-color 0.1s;
}

.mod-role-add:hover {
  color: var(--text-normal);
  border-color: var(--text-muted);
  box-shadow: none;
}

.mod-role-menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  min-width: 160px;
  background: var(--bg-floating);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 4px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  z-index: 10;
}

.mod-role-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 4px;
  font-size: 0.8125rem;
  color: var(--text-normal);
  cursor: pointer;
  transition: background 0.1s;
}

.mod-role-menu-item:hover {
  background: var(--bg-modifier-hover);
}

.mod-no-roles {
  font-size: 0.75rem;
  color: var(--text-faint);
  font-style: italic;
}

.mod-audit-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 24px 16px;
  color: var(--text-faint);
  font-size: 0.8125rem;
  text-align: center;
  background: var(--bg-tertiary);
  border-radius: 6px;
  margin-top: 6px;
}

.mod-danger-zone {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
}

.mod-ban-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--danger) !important;
  color: #fff !important;
}

.mod-ban-btn:hover {
  opacity: 0.9;
  box-shadow: none !important;
}

.mod-unban-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--green) !important;
  color: #fff !important;
}

.mod-unban-btn:hover {
  opacity: 0.9;
  box-shadow: none !important;
}

/* Ban confirmation */
.ban-confirm-overlay {
  z-index: 200;
}

.ban-confirm-modal {
  background: var(--bg-primary);
  padding: 24px;
  border-radius: 8px;
  width: 380px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.ban-confirm-modal h3 {
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--header-primary);
  margin-bottom: 8px;
}

.ban-confirm-desc {
  font-size: 0.8125rem;
  color: var(--text-muted);
  margin-bottom: 20px;
}

.ban-confirm-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
</style>
