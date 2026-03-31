<template>
  <Teleport to="body">
    <div class="user-card-overlay" @click="close" @contextmenu.prevent="close" />
    <div class="user-card" :style="cardStyle" ref="cardEl">
      <div class="user-card-banner" />

      <div class="user-card-avatar" :style="avatarColor">
        {{ user.display_name[0]?.toUpperCase() }}
      </div>

      <div class="user-card-body">
        <div class="user-card-name">{{ user.display_name }}</div>
        <div class="user-card-status" :class="{ online: isOnline }">
          <Circle :size="8" fill="currentColor" />
          {{ isOnline ? 'En ligne' : 'Hors ligne' }}
        </div>

        <div class="user-card-separator" />

        <div class="user-card-section">
          <div class="user-card-section-title">Roles</div>
          <div class="user-card-roles" v-if="userRoles.length">
            <div
              v-for="role in userRoles"
              :key="role.id"
              class="role-badge"
              :style="roleBadgeStyle(role)"
            >
              <Circle :size="8" fill="currentColor" />
              <span>{{ role.name }}</span>
              <button
                v-if="canManageRoles && role.id > 2"
                class="role-remove-btn"
                @click.stop="onRemoveRole(role.id)"
                title="Retirer"
              >&times;</button>
            </div>
          </div>
          <span v-else class="user-card-no-roles">Aucun role</span>

          <div v-if="canManageRoles && availableRoles.length" class="role-add">
            <button class="role-add-btn" @click="showRoleDropdown = !showRoleDropdown">
              <Plus :size="14" /> Ajouter un role
            </button>
            <div v-if="showRoleDropdown" class="role-dropdown-overlay" @click.stop="showRoleDropdown = false"></div>
            <div v-if="showRoleDropdown" class="role-dropdown">
              <div
                v-for="role in availableRoles"
                :key="role.id"
                class="role-dropdown-item"
                @click="onAssignRole(role.id)"
              >
                <Circle :size="8" fill="currentColor" :style="{ color: role.color || 'var(--text-muted)' }" />
                {{ role.name }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { Circle, Plus } from "lucide-vue-next";
import { activeState, activeServer } from "../store";
import { api, type User, type Role } from "../api";
import * as perms from "../permissions";

const props = defineProps<{
  user: User;
  x: number;
  y: number;
}>();

const emit = defineEmits<{ close: [] }>();

const cardEl = ref<HTMLElement>();
const showRoleDropdown = ref(false);

const state = computed(() => activeState());
const server = computed(() => activeServer());

const userRoles = computed(() => {
  const roleIds = state.value?.userRoles.get(props.user.id) ?? [];
  const allRoles = state.value?.roles ?? [];
  return allRoles.filter((r) => roleIds.includes(r.id)).sort((a, b) => a.position - b.position);
});

const isOnline = computed(() =>
  state.value?.onlineUsers.has(props.user.id) ?? false
);

const canManageRoles = computed(() =>
  perms.has(state.value?.permissions ?? 0, perms.MANAGE_ROLES)
);

const availableRoles = computed(() =>
  (state.value?.roles ?? []).filter(
    (r) => !userRoles.value.some((ur) => ur.id === r.id)
  )
);

const topRole = computed(() =>
  userRoles.value.find((r) => r.color) ?? null
);

const avatarColor = computed(() => {
  const color = topRole.value?.color || "var(--accent)";
  return { background: color };
});

function roleBadgeStyle(role: Role) {
  const c = role.color || "var(--text-muted)";
  return {
    "--role-color": c,
  };
}

const cardStyle = computed(() => {
  const cardW = 300;
  const cardH = 400;
  let left = props.x;
  let top = props.y;

  if (left + cardW > window.innerWidth) left = left - cardW;
  if (top + cardH > window.innerHeight) top = Math.max(8, window.innerHeight - cardH - 8);
  if (left < 8) left = 8;

  return { top: `${top}px`, left: `${left}px` };
});

async function onAssignRole(roleId: number) {
  const s = server.value;
  if (!s) return;
  await api.assignRole(s.url, s.token, roleId, props.user.id);
  showRoleDropdown.value = false;
}

async function onRemoveRole(roleId: number) {
  const s = server.value;
  if (!s) return;
  await api.removeRole(s.url, s.token, roleId, props.user.id);
}

function close() {
  emit("close");
}
</script>

<style scoped>
.user-card-overlay {
  position: fixed;
  inset: 0;
  z-index: 300;
}

.user-card {
  position: fixed;
  width: 300px;
  background: var(--bg-tertiary);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  z-index: 301;
  overflow: hidden;
}

.user-card-banner {
  height: 60px;
  background: var(--bg-secondary);
}

.user-card-avatar {
  width: 52px;
  height: 52px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 1.25rem;
  color: #fff;
  margin: -26px 0 0 16px;
  border: 4px solid var(--bg-tertiary);
}

.user-card-body {
  padding: 8px 16px 16px;
}

.user-card-name {
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--header-primary);
  margin-top: 4px;
}



.user-card-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--text-faint);
  margin-top: 8px;
}

.user-card-status.online {
  color: var(--green);
}

.user-card-separator {
  height: 1px;
  background: var(--border);
  margin: 12px 0;
}

.user-card-section-title {
  font-size: 0.6875rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-muted);
  margin-bottom: 8px;
}

.user-card-roles {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.role-badge {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 6px;
  font-size: 0.75rem;
  font-weight: 500;
  background: var(--bg-secondary);
  color: var(--role-color);
}

.role-badge span {
  color: var(--text-normal);
}

.role-remove-btn {
  width: 14px;
  height: 14px;
  padding: 0;
  margin: 0 0 0 2px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: transparent;
  color: var(--text-faint);
  font-size: 0.75rem;
  cursor: pointer;
  border: none;
  line-height: 1;
}

.role-remove-btn:hover {
  color: var(--danger);
  background: rgba(208, 80, 80, 0.15);
  box-shadow: none;
}

.user-card-no-roles {
  font-size: 0.75rem;
  color: var(--text-faint);
}

.role-add {
  margin-top: 8px;
  position: relative;
}

.role-add-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  width: auto;
  margin: 0;
  border-radius: 6px;
  background: transparent;
  border: 1px dashed var(--border);
  color: var(--text-muted);
  font-size: 0.75rem;
  font-weight: 500;
  cursor: pointer;
}

.role-add-btn:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
  box-shadow: none;
}

.role-dropdown-overlay {
  position: fixed;
  inset: 0;
  z-index: 299;
}

.role-dropdown {
  z-index: 300;
  position: absolute;
  bottom: 100%;
  left: 0;
  margin-bottom: 4px;
  background: var(--bg-floating);
  border-radius: 8px;
  padding: 6px;
  min-width: 180px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
}

.role-dropdown-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 6px;
  font-size: 0.8125rem;
  color: var(--text-normal);
  cursor: pointer;
}

.role-dropdown-item:hover {
  background: var(--bg-modifier-hover);
}
</style>
