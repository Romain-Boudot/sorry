<template>
  <div class="modal-overlay" @click.self="close">
    <div class="settings">
      <div class="settings-sidebar">
        <div class="sidebar-section-label">Mon compte</div>
        <div
          v-for="tab in visibleUserTabs"
          :key="tab.id"
          class="settings-tab"
          :class="{ active: activeTab === tab.id }"
          @click="activeTab = tab.id"
        >
          <component :is="tab.icon" :size="16" />
          <span>{{ tab.label }}</span>
        </div>

        <template v-if="visibleServerTabs.length">
          <div class="sidebar-separator"></div>
          <div class="sidebar-section-label">Serveur</div>
          <div
            v-for="tab in visibleServerTabs"
            :key="tab.id"
            class="settings-tab"
            :class="{ active: activeTab === tab.id }"
            @click="activeTab = tab.id"
          >
            <component :is="tab.icon" :size="16" />
            <span>{{ tab.label }}</span>
          </div>
        </template>
      </div>

      <div class="settings-content">
        <div class="settings-header">
          <h2>{{ activeTabLabel }}</h2>
          <button class="settings-close" @click="close">
            <X :size="20" />
          </button>
        </div>

        <ServerTab v-if="activeTab === 'server'" />
        <StatsTab v-else-if="activeTab === 'stats'" />
        <ProfileTab v-else-if="activeTab === 'profile'" />
        <RolesTab v-else-if="activeTab === 'roles'" />
        <ModerationTab v-else-if="activeTab === 'moderation'" />
        <InvitesTab v-else-if="activeTab === 'invites'" />
        <SecurityTab v-else-if="activeTab === 'security'" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { X, UserRound, KeyRound, Server, ShieldCheck, Gavel, TicketPlus, BarChart3 } from "lucide-vue-next";
import { store, activeState } from "../store";
import * as perms from "../permissions";

import ServerTab from "./settings/ServerTab.vue";
import ProfileTab from "./settings/ProfileTab.vue";
import RolesTab from "./settings/RolesTab.vue";
import ModerationTab from "./settings/ModerationTab.vue";
import InvitesTab from "./settings/InvitesTab.vue";
import SecurityTab from "./settings/SecurityTab.vue";
import StatsTab from "./settings/StatsTab.vue";

const state = computed(() => activeState());
const activeTab = ref(store.serverSettingsTab || "profile");

const userTabs = [
  { id: "profile", label: "Profil", icon: UserRound, permission: 0 },
  { id: "security", label: "Securite", icon: KeyRound, permission: 0 },
];

const serverTabs = [
  { id: "server", label: "Serveur", icon: Server, permission: perms.MANAGE_SERVER },
  { id: "stats", label: "Statistiques", icon: BarChart3, permission: perms.MANAGE_SERVER },
  { id: "roles", label: "Roles", icon: ShieldCheck, permission: perms.MANAGE_ROLES },
  { id: "moderation", label: "Moderation", icon: Gavel, permission: perms.BAN_MEMBERS },
  { id: "invites", label: "Invitations", icon: TicketPlus, permission: perms.CREATE_INVITE },
];

const allTabs = [...userTabs, ...serverTabs];

const visibleUserTabs = computed(() => {
  const p = state.value?.permissions ?? 0;
  return userTabs.filter((t) => t.permission === 0 || perms.has(p, t.permission));
});

const visibleServerTabs = computed(() => {
  const p = state.value?.permissions ?? 0;
  return serverTabs.filter((t) => t.permission === 0 || perms.has(p, t.permission));
});

const activeTabLabel = computed(() => allTabs.find((t) => t.id === activeTab.value)?.label ?? "");

function close() {
  store.showServerSettingsModal = false;
}
</script>

<style scoped>
/* ── Modal shell ── */
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

.sidebar-section-label {
  font-size: 0.6875rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.02em;
  color: var(--text-faint);
  padding: 4px 10px 4px;
}

.sidebar-separator {
  height: 1px;
  background: var(--border);
  margin: 8px 10px;
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
</style>
