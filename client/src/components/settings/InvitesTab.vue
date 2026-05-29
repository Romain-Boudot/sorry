<template>
  <SettingsBody>
    <div class="card">
      <div class="card-title">Creer une invitation</div>
      <p class="card-hint">Genere un code pour inviter quelqu'un sur le serveur.</p>
      <div class="invite-create-row">
        <div class="invite-field">
          <label>Utilisations max</label>
          <NumberStepper :model-value="newInviteMaxUses ?? 0" @update:model-value="newInviteMaxUses = $event || null" :min="0" :max="999" />
        </div>
        <div class="invite-field">
          <label>Expiration (heures)</label>
          <NumberStepper :model-value="newInviteExpireHours ?? 0" @update:model-value="newInviteExpireHours = $event || null" :min="0" :max="720" />
        </div>
        <button class="btn-sm invite-create-btn" @click="createInvite" :disabled="!canCreateInvite">
          <TicketPlus :size="14" />
          Creer
        </button>
      </div>
      <div class="invite-options-row">
        <div class="invite-field invite-role-field" ref="roleDropdownRef">
          <label>Role attribue</label>
          <div class="role-select-trigger" @click="roleDropdownOpen = !roleDropdownOpen">
            <template v-if="selectedRole">
              <Circle :size="8" fill="currentColor" :style="{ color: selectedRole.color || 'var(--text-muted)' }" />
              <span>{{ selectedRole.name }}</span>
            </template>
            <span v-else class="role-select-placeholder">Aucun</span>
            <ChevronDown :size="14" class="role-select-arrow" :class="{ flipped: roleDropdownOpen }" />
          </div>
          <div v-if="roleDropdownOpen" class="role-select-menu">
            <div class="role-select-item" :class="{ active: !newInviteRoleId }" @click="newInviteRoleId = null; roleDropdownOpen = false">
              <span class="role-select-placeholder">Aucun</span>
            </div>
            <div
              v-for="role in assignableRoles"
              :key="role.id"
              class="role-select-item"
              :class="{ active: newInviteRoleId === role.id }"
              @click="newInviteRoleId = role.id; roleDropdownOpen = false"
            >
              <Circle :size="8" fill="currentColor" :style="{ color: role.color || 'var(--text-muted)' }" />
              <span>{{ role.name }}</span>
            </div>
            <div
              v-for="role in escalatingRoles"
              :key="role.id"
              class="role-select-item locked"
              title="Tu ne peux pas attribuer ce role : il accorde des permissions que tu ne possedes pas."
            >
              <Lock :size="10" />
              <span>{{ role.name }}</span>
            </div>
          </div>
        </div>
        <div class="invite-field invite-toggle-field">
          <label>Guest</label>
          <div class="invite-toggle-wrapper">
            <button class="invite-toggle" :class="{ active: newInviteGuest }" @click="toggleGuest">
              <span class="invite-toggle-knob" />
            </button>
          </div>
        </div>
      </div>
      <p v-if="newInviteGuest" class="card-hint" style="margin-top: 4px;">Les guests se connectent sans creer de compte (juste un pseudo). Ils n'ont pas les permissions de base (everyone), uniquement celles du role attribue. Un role est obligatoire.</p>
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
              <span v-if="inv.guest" class="guest-tag">Guest</span>
              par {{ resolveUser(inv.created_by) }}
              · {{ inv.uses }}{{ inv.max_uses ? `/${inv.max_uses}` : '' }} utilisations
              <template v-if="inv.expires_at"> · {{ formatExpiry(inv.expires_at) }}</template>
              <template v-if="inv.role_id"> · role: {{ getRoleName(inv.role_id) }}</template>
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
  </SettingsBody>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { Trash2, TicketPlus, Copy, Check, Link2, Circle, ChevronDown, Lock } from "lucide-vue-next";
import NumberStepper from "../ui/NumberStepper.vue";
import { showToast } from "../../composables/useToast";
import { activeState, activeServer, resolveUser } from "../../store";
import { api, type Invite } from "../../api";
import * as perms from "../../permissions";
import SettingsBody from "../ui/SettingsBody.vue";

const invites = ref<Invite[]>([]);
const newInviteMaxUses = ref<number | null>(null);
const newInviteExpireHours = ref<number | null>(null);
const newInviteGuest = ref(false);
const newInviteRoleId = ref<number | null>(null);

const assignableRoles = computed(() => {
  const st = activeState();
  if (!st) return [];
  const isAdmin = perms.has(st.permissions, perms.ADMINISTRATOR);
  if (isAdmin) return st.roles.filter((r) => r.id > 2);
  const userRoleIds = st.userRoles.get(st.user?.id ?? 0) ?? [];
  const userHighest = st.roles
    .filter((r) => userRoleIds.includes(r.id))
    .reduce((min, r) => Math.min(min, r.position), Infinity);
  // Mirror of `require_permission_subset`: cannot attach a role that grants
  // permissions the actor doesn't possess.
  return st.roles.filter((r) =>
    r.id > 2
    && r.position > userHighest
    && (r.permissions & ~st.permissions) === 0
  );
});

/** Roles excluded from `assignableRoles` because their permissions exceed the actor's. */
const escalatingRoles = computed(() => {
  const st = activeState();
  if (!st) return [];
  if (perms.has(st.permissions, perms.ADMINISTRATOR)) return [];
  const userRoleIds = st.userRoles.get(st.user?.id ?? 0) ?? [];
  const userHighest = st.roles
    .filter((r) => userRoleIds.includes(r.id))
    .reduce((min, r) => Math.min(min, r.position), Infinity);
  return st.roles.filter((r) =>
    r.id > 2
    && r.position > userHighest
    && (r.permissions & ~st.permissions) !== 0
  );
});

const selectedRole = computed(() =>
  newInviteRoleId.value ? assignableRoles.value.find((r) => r.id === newInviteRoleId.value) ?? null : null
);

const roleDropdownOpen = ref(false);
const roleDropdownRef = ref<HTMLElement>();

function onRoleClickOutside(e: MouseEvent) {
  if (roleDropdownRef.value && !roleDropdownRef.value.contains(e.target as Node)) {
    roleDropdownOpen.value = false;
  }
}

onMounted(() => {
  document.addEventListener("click", onRoleClickOutside);
  loadInvites();
});

onUnmounted(() => {
  document.removeEventListener("click", onRoleClickOutside);
});

function toggleGuest() {
  newInviteGuest.value = !newInviteGuest.value;
  if (newInviteGuest.value && !newInviteRoleId.value && assignableRoles.value.length) {
    newInviteRoleId.value = assignableRoles.value[0].id;
  }
}

const canCreateInvite = computed(() => {
  if (newInviteGuest.value && !newInviteRoleId.value) return false;
  return true;
});

function getRoleName(roleId: number): string {
  const role = activeState()?.roles.find((r) => r.id === roleId);
  return role?.name ?? `#${roleId}`;
}

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
    role_id: newInviteGuest.value ? newInviteRoleId.value : null,
    guest: newInviteGuest.value,
  });
  invites.value.unshift(inv);
  newInviteMaxUses.value = null;
  newInviteExpireHours.value = null;
  newInviteGuest.value = false;
  newInviteRoleId.value = null;
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
  showToast("Invitation revoquee");
}

function formatExpiry(ts: number): string {
  const now = Date.now() / 1000;
  const diff = ts - now;
  if (diff <= 0) return "expire";
  if (diff < 3600) return `dans ${Math.ceil(diff / 60)} min`;
  if (diff < 86400) return `dans ${Math.ceil(diff / 3600)} h`;
  return `dans ${Math.ceil(diff / 86400)} j`;
}
</script>

<style scoped>
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

.btn-sm {
  width: auto;
  padding: 8px 16px;
  margin: 0;
  font-size: 0.8125rem;
  border-radius: 6px;
  flex-shrink: 0;
}

.invite-create-row {
  display: flex;
  align-items: flex-end;
  gap: 10px;
  margin-bottom: 8px;
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


.invite-options-row {
  display: flex;
  align-items: flex-end;
  gap: 12px;
}

.invite-role-field {
  position: relative;
  flex: 1;
}

.role-select-trigger {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 8px;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.875rem;
  cursor: pointer;
  transition: background 0.1s;
}

.role-select-trigger:hover {
  background: var(--bg-modifier-hover);
}

.role-select-placeholder {
  color: var(--text-faint);
}

.role-select-arrow {
  margin-left: auto;
  color: var(--text-faint);
  flex-shrink: 0;
  transition: transform 0.15s;
}

.role-select-arrow.flipped {
  transform: rotate(180deg);
}

.role-select-menu {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  background: var(--bg-tertiary);
  border-radius: 8px;
  padding: 4px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
  z-index: 50;
  max-height: 180px;
  overflow-y: auto;
}

.role-select-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  font-size: 0.8125rem;
  color: var(--text-muted);
  cursor: pointer;
  transition: background 0.08s, color 0.08s;
}

.role-select-item:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
}

.role-select-item.active {
  color: var(--header-primary);
  background: var(--bg-modifier-active);
}

.role-select-item.locked {
  opacity: 0.45;
  cursor: not-allowed;
}
.role-select-item.locked:hover {
  background: transparent;
  color: var(--text-muted);
}

.invite-toggle-field {
  flex: 0 0 auto;
}

.invite-toggle-wrapper {
  height: 34px;
  display: flex;
  align-items: center;
}

.invite-toggle {
  width: 36px;
  height: 20px;
  border-radius: 10px;
  background: var(--bg-modifier-hover);
  border: none;
  padding: 2px;
  margin: 0;
  cursor: pointer;
  display: flex;
  align-items: center;
  transition: background 0.15s;
}

.invite-toggle:hover { box-shadow: none; }

.invite-toggle.active {
  background: var(--accent);
}

.invite-toggle-knob {
  display: block;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--text-faint);
  transition: transform 0.15s, background 0.15s;
}

.invite-toggle.active .invite-toggle-knob {
  transform: translateX(16px);
  background: var(--accent-fg);
}

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
.invite-revoke:hover { color: var(--danger); background: var(--danger-bg); box-shadow: none; }

.invite-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 24px;
  color: var(--text-faint);
  font-size: 0.8125rem;
}

.guest-tag {
  font-family: var(--font-mono);
  font-size: 0.5625rem;
  font-weight: 600;
  background: var(--accent);
  color: var(--accent-fg);
  padding: 1px 5px;
  border-radius: 4px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  margin-right: 4px;
}
</style>
