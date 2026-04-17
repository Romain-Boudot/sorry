<template>
  <SettingsBody class="logs-tab">
    <div class="logs-toolbar">
      <BaseTabs v-model="tab" :items="tabItems" />
      <button type="button" class="refresh-btn" @click="refresh" :disabled="loading" title="Rafraichir">
        <RotateCw :size="14" :class="{ spin: loading }" />
      </button>
    </div>

    <!-- Audit log -->
    <div v-if="tab === 'audit'" class="logs-body">
      <div v-if="loading && !audit.length" class="logs-empty">
        <Loader2 :size="16" class="spin" />
        <span>Chargement...</span>
      </div>
      <div v-else-if="!audit.length" class="logs-empty">
        <ScrollText :size="20" />
        <span>Aucune action enregistree</span>
      </div>
      <div v-else class="audit-list">
        <div v-for="entry in audit" :key="entry.id" class="audit-entry">
          <div class="audit-time">{{ formatDate(entry.created_at) }}</div>
          <div class="audit-main">
            <span class="audit-actor">{{ resolveUser(entry.actor_id) }}</span>
            <span class="audit-action" :class="actionClass(entry.action)">{{ actionLabel(entry.action) }}</span>
            <span v-if="entry.target_user_id" class="audit-target">
              <UserRound :size="12" /> {{ resolveUser(entry.target_user_id) }}
            </span>
            <span v-if="entry.target_channel_id" class="audit-target">
              <Hash :size="12" /> {{ channelName(entry.target_channel_id) }}
            </span>
            <span v-if="entry.target_role_id" class="audit-target">
              <ShieldCheck :size="12" /> {{ roleName(entry.target_role_id) }}
            </span>
            <span v-if="entry.details" class="audit-details">"{{ entry.details }}"</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Server logs -->
    <div v-else class="logs-body">
      <div v-if="loading && !serverLogs.length" class="logs-empty">
        <Loader2 :size="16" class="spin" />
        <span>Chargement...</span>
      </div>
      <div v-else-if="!serverLogs.length" class="logs-empty">
        <Terminal :size="20" />
        <span>Aucun log (WARN / ERROR uniquement)</span>
      </div>
      <div v-else class="server-log-list">
        <div v-for="(entry, i) in serverLogs" :key="i" class="log-entry" :class="entry.level.toLowerCase()">
          <span class="log-time">{{ formatTime(entry.timestamp) }}</span>
          <span class="log-level">{{ entry.level }}</span>
          <span class="log-target">{{ entry.target }}</span>
          <span class="log-message">{{ entry.message }}</span>
        </div>
      </div>
    </div>
  </SettingsBody>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { Loader2, RotateCw, UserRound, Hash, ShieldCheck, ScrollText, Terminal } from "lucide-vue-next";
import { activeServer, activeState, resolveUser } from "../../store";
import { api, type AuditLog, type ServerLogEntry } from "../../api";
import SettingsBody from "../ui/SettingsBody.vue";
import BaseTabs from "../ui/BaseTabs.vue";

const tab = ref<"audit" | "server">("audit");
const audit = ref<AuditLog[]>([]);
const serverLogs = ref<ServerLogEntry[]>([]);
const loading = ref(false);

const tabItems = computed(() => [
  { id: "audit", label: "Audit", icon: ScrollText, count: audit.value.length },
  { id: "server", label: "Serveur", icon: Terminal, count: serverLogs.value.length },
]);

const state = computed(() => activeState());

function channelName(id: number): string {
  return state.value?.channels.find(c => c.id === id)?.name ?? `#${id}`;
}

function roleName(id: number): string {
  return state.value?.roles.find(r => r.id === id)?.name ?? `role #${id}`;
}

async function load() {
  const s = activeServer();
  if (!s) return;
  loading.value = true;
  try {
    if (tab.value === "audit") {
      audit.value = await api.listAudit(s.url, s.token, 100);
    } else {
      serverLogs.value = await api.listServerLogs(s.url, s.token, 500);
    }
  } catch {
    // ignore
  } finally {
    loading.value = false;
  }
}

async function refresh() {
  await load();
}

watch(tab, load);
onMounted(load);

function formatDate(ts: string): string {
  try {
    const date = new Date(ts.includes("Z") ? ts : ts + "Z");
    return date.toLocaleDateString("fr-FR", { day: "numeric", month: "short" })
      + " " + date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  } catch { return ts; }
}

function formatTime(ts: string): string {
  try {
    const date = new Date(ts);
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  } catch { return ts; }
}

const ACTION_LABELS: Record<string, string> = {
  "user.ban": "a banni",
  "user.unban": "a debanni",
  "role.create": "a cree le role",
  "role.update": "a modifie le role",
  "role.delete": "a supprime le role",
  "role.assign": "a attribue un role a",
  "role.remove": "a retire un role de",
  "channel.create": "a cree le channel",
  "channel.update": "a modifie le channel",
  "channel.delete": "a supprime le channel",
  "channel.overwrite.set": "a modifie les permissions du channel",
  "channel.overwrite.delete": "a retire un overwrite du channel",
  "voice.force_mute": "a force mute",
  "voice.force_unmute": "a retire le force mute de",
  "voice.force_deafen": "a force deafen",
  "voice.force_undeafen": "a retire le force deafen de",
  "voice.kick": "a deconnecte du vocal",
  "voice.move": "a deplace",
  "message.delete": "a supprime un message de",
  "message.pin": "a epingle un message dans",
  "message.unpin": "a desepingle un message dans",
  "server.update": "a modifie le serveur",
};

function actionLabel(action: string): string {
  return ACTION_LABELS[action] ?? action;
}

function actionClass(action: string): string {
  if (action.endsWith("delete") || action === "user.ban" || action === "voice.kick" || action === "message.delete") return "danger";
  if (action.endsWith("create")) return "success";
  return "";
}
</script>

<style scoped>
.logs-tab {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.logs-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.refresh-btn {
  margin: 0 0 0 auto;
  width: 32px;
  height: 32px;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border);
  background: var(--bg-tertiary);
  color: var(--text-muted);
  border-radius: 8px;
  cursor: pointer;
  transition: color 0.12s, background 0.12s;
  box-shadow: none;
}
.refresh-btn:hover:not(:disabled) {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
}
.refresh-btn:disabled { opacity: 0.5; cursor: default; }

.spin { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.logs-body {
  flex: 1;
  overflow-y: auto;
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 6px;
  min-height: 200px;
}

.logs-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 48px 16px;
  font-size: 0.8125rem;
  color: var(--text-faint);
}
.logs-empty :deep(svg) { opacity: 0.6; }

/* ── Audit entries ── */
.audit-list {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.audit-entry {
  display: flex;
  gap: 12px;
  padding: 8px 12px;
  font-size: 0.8125rem;
  border-radius: 6px;
  transition: background 0.1s;
}

.audit-entry:hover {
  background: var(--bg-modifier-hover);
}

.audit-time {
  flex-shrink: 0;
  color: var(--text-faint);
  font-size: 0.75rem;
  width: 110px;
  padding-top: 2px;
  font-variant-numeric: tabular-nums;
}

.audit-main {
  flex: 1;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.audit-actor {
  font-weight: 600;
  color: var(--header-primary);
}

.audit-action {
  color: var(--text-muted);
}

.audit-action.danger { color: var(--danger); }
.audit-action.success { color: var(--green); }

.audit-target {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  background: var(--bg-tertiary);
  border-radius: 999px;
  color: var(--text-normal);
  font-size: 0.75rem;
  font-weight: 500;
}
.audit-target :deep(svg) { opacity: 0.7; }

.audit-details {
  color: var(--text-faint);
  font-style: italic;
  font-size: 0.75rem;
}

/* ── Server logs ── */
.server-log-list {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.log-entry {
  display: flex;
  gap: 10px;
  padding: 6px 10px;
  border-radius: 6px;
  border-left: 3px solid transparent;
  align-items: baseline;
  transition: background 0.1s;
}
.log-entry:hover {
  background: var(--bg-modifier-hover);
}

.log-entry.warn {
  border-left-color: var(--yellow, #e3b341);
  background: rgba(227, 179, 65, 0.05);
}
.log-entry.warn:hover { background: rgba(227, 179, 65, 0.1); }

.log-entry.error {
  border-left-color: var(--danger);
  background: rgba(240, 71, 71, 0.08);
}
.log-entry.error:hover { background: rgba(240, 71, 71, 0.13); }

.log-time {
  color: var(--text-faint);
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
}

.log-level {
  flex-shrink: 0;
  font-weight: 700;
  width: 48px;
  letter-spacing: 0.02em;
}

.log-entry.warn .log-level { color: var(--yellow, #e3b341); }
.log-entry.error .log-level { color: var(--danger); }

.log-target {
  color: var(--text-muted);
  flex-shrink: 0;
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.log-message {
  color: var(--text-normal);
  word-break: break-word;
  flex: 1;
  line-height: 1.4;
}
</style>
