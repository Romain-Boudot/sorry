<template>
  <SettingsBody class="split-view">
    <div class="split-list">
      <BaseTabs
        :model-value="modSubTab"
        :items="modSubTabItems"
        variant="stretched"
        class="mod-sub-tabs"
        @update:model-value="onSubTabChange"
      />

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

      <BaseTabs
        v-model="activityTab"
        :items="activityTabItems"
        variant="stretched"
        class="mod-activity-tabs"
      />

      <!-- Messages tab -->
      <div v-if="activityTab === 'messages'" class="mod-messages" ref="messagesContainer" @scroll="onMessagesScroll">
        <div v-if="loadingMessages && !userMessages.length" class="mod-messages-status">
          <Loader2 :size="14" class="spinner" /> Chargement...
        </div>
        <div v-else-if="!userMessages.length" class="mod-messages-status">
          Aucun message
        </div>
        <template v-else>
          <div v-for="msg in userMessages" :key="msg.id" class="mod-msg">
            <div class="mod-msg-header">
              <span class="mod-msg-channel">#{{ channelName(msg.channel_id) }}</span>
              <span class="mod-msg-time">{{ formatMsgDate(msg.created_at) }}</span>
            </div>
            <div class="mod-msg-content">{{ truncate(msg.content, 200) }}</div>
            <div v-if="msg.attachments.length" class="mod-msg-attachments">
              {{ msg.attachments.length }} fichier{{ msg.attachments.length > 1 ? 's' : '' }}
            </div>
          </div>
          <div v-if="loadingMessages" class="mod-messages-status">
            <Loader2 :size="14" class="spinner" /> Chargement...
          </div>
        </template>
      </div>

      <!-- Actions tab -->
      <div v-if="activityTab === 'actions'" class="mod-messages">
        <div v-if="loadingAudit && !userAudit.length" class="mod-messages-status">
          <Loader2 :size="14" class="spinner" /> Chargement...
        </div>
        <div v-else-if="!userAudit.length" class="mod-messages-status" style="flex-direction: column; padding: 24px;">
          <ScrollText :size="20" />
          <span>Aucune action enregistree</span>
        </div>
        <template v-else>
          <div v-for="entry in userAudit" :key="entry.id" class="mod-audit-entry">
            <div class="mod-audit-header">
              <span class="mod-audit-action">{{ actionLabel(entry.action) }}</span>
              <span class="mod-audit-time">{{ formatMsgDate(entry.created_at) }}</span>
            </div>
            <div class="mod-audit-targets">
              <span v-if="entry.target_user_id" class="mod-audit-target">
                <UserRound :size="12" /> {{ targetUserName(entry.target_user_id) }}
              </span>
              <span v-if="entry.target_channel_id" class="mod-audit-target">
                <Hash :size="12" /> {{ channelName(entry.target_channel_id) }}
              </span>
              <span v-if="entry.target_role_id" class="mod-audit-target">
                <ShieldCheck :size="12" /> {{ roleName(entry.target_role_id) }}
              </span>
              <span v-if="entry.details" class="mod-audit-details">"{{ entry.details }}"</span>
            </div>
          </div>
        </template>
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
  </SettingsBody>

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
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { X, ShieldCheck, Ban, UserRound, Search, Calendar, Plus, Loader2, ScrollText, Hash } from "lucide-vue-next";
import { showToast } from "../../composables/useToast";
import { activeState, activeServer } from "../../store";
import { api, type BannedUser, type Message, type AuditLog } from "../../api";
import SettingsBody from "../ui/SettingsBody.vue";
import BaseTabs from "../ui/BaseTabs.vue";

const state = computed(() => activeState());

const modSubTab = ref<"members" | "banned">("members");
const activityTab = ref<"messages" | "actions">("messages");

const modSubTabItems = computed(() => [
  { id: "members", label: "Membres", count: filteredUsers.value.length },
  { id: "banned", label: "Bannis", count: bannedUsers.value.length },
]);

const activityTabItems = computed(() => [
  { id: "messages", label: "Messages" },
  { id: "actions", label: "Actions" },
]);

function onSubTabChange(id: string) {
  modSubTab.value = id as "members" | "banned";
  if (id === "banned") loadBannedUsers();
}

const modSearch = ref("");
const modFilter = ref<"all" | "online" | "offline">("all");
const showBanConfirm = ref(false);
const bannedUsers = ref<BannedUser[]>([]);
const selectedBannedId = ref<number | null>(null);
const showRoleMenu = ref(false);

const roles = ref<{ id: number; name: string; permissions: number; color: string | null; position: number }[]>([]);

onMounted(async () => {
  const s = activeServer();
  if (!s) return;
  try {
    roles.value = await api.listRoles(s.url, s.token);
  } catch {}
});

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
  if (myId === 1) return targetId !== 1;
  if (targetId === 1) return false;
  if (myId === targetId) return false;
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

const userMessages = ref<Message[]>([]);
const loadingMessages = ref(false);
const hasMoreMessages = ref(false);
const messagesContainer = ref<HTMLElement>();

const userAudit = ref<AuditLog[]>([]);
const loadingAudit = ref(false);

async function loadUserAudit(userId: number) {
  const s = activeServer();
  if (!s) return;
  loadingAudit.value = true;
  try {
    userAudit.value = await api.userAudit(s.url, s.token, userId, 50);
  } catch {
    userAudit.value = [];
  } finally {
    loadingAudit.value = false;
  }
}

const ACTION_LABELS: Record<string, string> = {
  "user.ban": "Ban",
  "user.unban": "Unban",
  "role.create": "Creation role",
  "role.update": "Modification role",
  "role.delete": "Suppression role",
  "role.assign": "Role attribue",
  "role.remove": "Role retire",
  "channel.create": "Creation channel",
  "channel.update": "Modification channel",
  "channel.delete": "Suppression channel",
  "channel.overwrite.set": "Modification permissions",
  "channel.overwrite.delete": "Suppression overwrite",
  "voice.force_mute": "Force mute",
  "voice.force_unmute": "Retrait force mute",
  "voice.force_deafen": "Force deafen",
  "voice.force_undeafen": "Retrait force deafen",
  "voice.kick": "Deconnexion vocal",
  "voice.move": "Deplacement vocal",
  "message.delete": "Suppression message",
  "message.pin": "Epinglage message",
  "message.unpin": "Desepinglage",
  "server.update": "Modification serveur",
};

function actionLabel(action: string): string {
  return ACTION_LABELS[action] ?? action;
}

function targetUserName(id: number): string {
  return state.value?.users.get(id)?.display_name ?? `User #${id}`;
}

watch([activityTab, selectedUserId], ([tab, uid]) => {
  if (tab === "actions" && uid && !userAudit.value.length) {
    loadUserAudit(uid);
  }
});

function roleName(id: number): string {
  return state.value?.roles.find(r => r.id === id)?.name ?? `role #${id}`;
}

function selectUser(userId: number) {
  selectedUserId.value = userId;
  selectedBannedId.value = null;
  activityTab.value = "messages";
  userAudit.value = [];
  loadUserMessages(userId);
}

async function loadUserMessages(userId: number, before?: number) {
  const s = activeServer();
  if (!s) return;
  loadingMessages.value = true;
  try {
    const msgs = await api.userMessages(s.url, s.token, userId, 20, before);
    if (before) {
      userMessages.value.push(...msgs);
    } else {
      userMessages.value = msgs;
    }
    hasMoreMessages.value = msgs.length >= 20;
  } catch {
    userMessages.value = [];
    hasMoreMessages.value = false;
  } finally {
    loadingMessages.value = false;
  }
}

function loadMoreMessages() {
  if (!selectedUserId.value || !userMessages.value.length || loadingMessages.value) return;
  const lastId = userMessages.value[userMessages.value.length - 1].id;
  loadUserMessages(selectedUserId.value, lastId);
}

function onMessagesScroll() {
  const el = messagesContainer.value;
  if (!el || !hasMoreMessages.value) return;
  if (el.scrollHeight - el.scrollTop - el.clientHeight < 80) {
    loadMoreMessages();
  }
}

function channelName(channelId: number): string {
  const ch = state.value?.channels.find(c => c.id === channelId);
  return ch?.name ?? "inconnu";
}

function truncate(s: string, max: number): string {
  return s.length > max ? s.slice(0, max) + "..." : s;
}

function formatMsgDate(ts: string): string {
  try {
    const d = new Date(ts + "Z");
    return d.toLocaleDateString("fr-FR", { day: "numeric", month: "short" })
      + " " + d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  } catch {
    return ts;
  }
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

  const name = selectedUser.value?.display_name;
  try {
    await api.banUser(s.url, s.token, uid);
    showBanConfirm.value = false;
    selectedUserId.value = null;
    showToast(`${name} a ete banni`);
  } catch {
    showToast("Erreur lors du bannissement", "error");
  }
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
  const name = bannedUsers.value.find(u => u.id === userId)?.display_name;
  try {
    await api.unbanUser(s.url, s.token, userId);
    bannedUsers.value = bannedUsers.value.filter(u => u.id !== userId);
    if (selectedBannedId.value === userId) selectedBannedId.value = null;
    showToast(`${name} a ete debanni`);
  } catch {
    showToast("Erreur lors du debannissement", "error");
  }
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

.card-title {
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-muted);
  margin-bottom: 4px;
}

.btn-sm {
  width: auto;
  padding: 8px 16px;
  margin: 0;
  font-size: 0.8125rem;
  border-radius: 6px;
  flex-shrink: 0;
}

.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

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

/* Moderation styles */
.mod-sub-tabs { margin-bottom: 10px; }

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

.mod-activity-tabs {
  margin-top: 16px;
}

.mod-messages {
  background: var(--bg-tertiary);
  border-radius: 0 0 6px 6px;
  max-height: 300px;
  overflow-y: auto;
}

.mod-messages-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 16px;
  font-size: 0.8125rem;
  color: var(--text-faint);
}

.spinner {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.mod-msg {
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
}

.mod-msg:last-child {
  border-bottom: none;
}

.mod-audit-entry {
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
}

.mod-audit-entry:last-child {
  border-bottom: none;
}

.mod-audit-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 4px;
}

.mod-audit-action {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--header-primary);
}

.mod-audit-time {
  font-size: 0.625rem;
  color: var(--text-faint);
}

.mod-audit-targets {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
}

.mod-audit-target {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 1px 6px;
  background: var(--bg-tertiary);
  border-radius: 4px;
  color: var(--text-normal);
  font-size: 0.75rem;
}

.mod-audit-details {
  color: var(--text-faint);
  font-style: italic;
  font-size: 0.75rem;
}

.mod-msg-header {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-bottom: 2px;
}

.mod-msg-channel {
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--text-muted);
}

.mod-msg-time {
  font-size: 0.625rem;
  color: var(--text-faint);
}

.mod-msg-content {
  font-size: 0.8125rem;
  color: var(--text-normal);
  line-height: 1.4;
  word-break: break-word;
}

.mod-msg-attachments {
  font-size: 0.6875rem;
  color: var(--text-faint);
  margin-top: 2px;
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
  color: var(--text-bright) !important;
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
  color: var(--text-bright) !important;
}

.mod-unban-btn:hover {
  opacity: 0.9;
  box-shadow: none !important;
}

/* Ban confirmation */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: var(--overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

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

.guest-tag {
  font-size: 0.5625rem;
  font-weight: 600;
  background: var(--accent);
  color: var(--text-bright);
  padding: 1px 5px;
  border-radius: 3px;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  margin-right: 4px;
}
</style>
