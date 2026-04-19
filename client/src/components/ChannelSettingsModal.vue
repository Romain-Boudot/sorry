<template>
  <div class="modal-overlay" @click.self="close">
    <div class="settings">
      <div class="settings-sidebar">
        <div
          v-for="tab in tabs"
          :key="tab.id"
          class="settings-tab"
          :class="{ active: activeTab === tab.id }"
          @click="activeTab = tab.id"
        >
          <component :is="tab.icon" :size="16" />
          <span>{{ tab.label }}</span>
        </div>

        <div class="sidebar-spacer"></div>

        <div class="settings-tab danger" @click="handleDelete">
          <Trash2 :size="16" />
          <span>Supprimer</span>
        </div>
      </div>

      <div class="settings-content">
        <div class="settings-header">
          <h2>
            <Hash v-if="channel?.kind === 'text'" :size="18" />
            <Volume2 v-else :size="18" />
            {{ channel?.name }}
          </h2>
          <button class="settings-close" @click="close">
            <X :size="20" />
          </button>
        </div>

        <!-- General -->
        <div v-if="activeTab === 'general'" class="settings-body">
          <div class="card">
            <div class="card-title">Nom du channel</div>
            <div class="input-row">
              <BaseInput v-model="channelName" placeholder="Nom" @keydown.enter="saveName" />
              <SaveButton :loading="savingName" :saved="nameSaved" :disabled="!channelName.trim() || channelName === channel?.name" @click="saveName" />
            </div>
          </div>
          <div class="card">
            <div class="card-title">Description / Topic</div>
            <div class="input-row">
              <BaseTextarea v-model="channelDescription" placeholder="Ajouter une description..." :rows="3" />
              <SaveButton :loading="savingDesc" :saved="descSaved" :disabled="channelDescription === (channel?.description ?? '')" @click="saveDescription" />
            </div>
            <p class="card-hint" style="margin-top: 6px;">Visible dans l'en-tete du channel.</p>
          </div>
          <div v-if="channel?.kind === 'voice'" class="card">
            <div class="card-title">Limite d'utilisateurs</div>
            <div class="input-row">
              <NumberStepper v-model="userLimit" :min="0" :max="99" />
              <SaveButton :loading="savingLimit" :saved="limitSaved" :disabled="userLimit === (channel?.user_limit ?? 0)" @click="saveUserLimit" />
            </div>
            <p class="card-hint" style="margin-top: 6px;">0 = pas de limite.</p>
          </div>
        </div>

        <!-- Permissions -->
        <div v-if="activeTab === 'permissions'" class="settings-body">
          <p class="card-hint">Autorise ou refuse des permissions par role pour ce channel.</p>

          <div v-for="role in roles" :key="role.id" class="ow-role">
            <div class="ow-role-header" @click="toggleRole(role.id)">
              <div class="dot" :style="`background:${role.color || 'var(--text-muted)'}`"></div>
              <span class="ow-role-name">{{ role.name }}</span>
              <span v-if="hasOverwrite(role.id)" class="badge">modifie</span>
              <ChevronDown :size="14" class="ow-chevron" :class="{ open: expandedRole === role.id }" />
            </div>
            <div v-if="expandedRole === role.id" class="ow-body">
              <div v-for="p in permsList" :key="p.flag" class="ow-perm">
                <span class="ow-perm-name">{{ p.name }}</span>
                <PermToggle
                  :model-value="getState(role.id, p.flag)"
                  @update:model-value="setState(role.id, p.flag, $event)"
                />
              </div>
            </div>
          </div>
        </div>

        <!-- Webhooks -->
        <div v-if="activeTab === 'webhooks'" class="settings-body">
          <p class="card-hint">
            Les webhooks entrants permettent a des services externes (CI, bots, alertes) de poster des messages.
            Compatible avec les payloads <strong>Discord</strong> (URL principale) et <strong>Slack</strong> (suffixe <code>/slack</code>).
          </p>

          <div class="card">
            <div class="card-title">Nouveau webhook</div>
            <div class="input-row">
              <BaseInput v-model="newWebhookName" placeholder="Nom (ex: GitHub Actions)" :maxlength="80" />
              <SaveButton :loading="creatingWebhook" :disabled="!newWebhookName.trim()" @click="createWebhook">Creer</SaveButton>
            </div>
            <p class="card-hint" style="margin-top: 6px;">L'URL d'avatar peut etre definie ensuite via l'edition.</p>
          </div>

          <div v-if="loadingWebhooks" class="webhook-empty">Chargement...</div>
          <div v-else-if="webhooks.length === 0" class="webhook-empty">Aucun webhook pour ce channel.</div>

          <div v-for="wh in webhooks" :key="wh.id" class="webhook-item">
            <div class="webhook-header">
              <div class="webhook-avatar clickable" @click="triggerAvatarUpload(wh.id)" :title="wh.avatar_url ? 'Changer' : 'Uploader un avatar'">
                <img v-if="webhookAvatarSrc(wh)" :src="webhookAvatarSrc(wh)!" :alt="wh.name" />
                <span v-else>{{ wh.name[0]?.toUpperCase() }}</span>
                <div class="avatar-overlay"><Camera :size="14" /></div>
              </div>
              <div class="webhook-meta">
                <div class="webhook-name">{{ wh.name }}</div>
                <div class="webhook-sub">
                  <button v-if="wh.avatar_url" class="link-btn" @click="removeAvatar(wh.id)">Supprimer l'avatar</button>
                  <span v-else>Clique sur l'avatar pour uploader une image</span>
                </div>
              </div>
              <button class="webhook-action danger" @click="confirmDeleteId = wh.id" title="Supprimer le webhook">
                <Trash2 :size="14" />
              </button>
              <input
                type="file"
                accept="image/png,image/jpeg,image/gif,image/webp"
                hidden
                :ref="(el) => { if (el) avatarInputs[wh.id] = el as HTMLInputElement }"
                @change="(e) => onAvatarSelect(wh.id, e)"
              />
            </div>

            <div class="webhook-row">
              <label class="webhook-label">Nom</label>
              <BaseInput v-model="wh.name" :maxlength="80" />
            </div>
            <div class="webhook-row webhook-actions-row">
              <button class="btn-secondary" @click="saveWebhook(wh)" :disabled="savingWebhookId === wh.id">
                {{ savingWebhookId === wh.id ? "Sauvegarde..." : "Enregistrer" }}
              </button>
            </div>

            <div class="webhook-url">
              <label class="webhook-label">URL (a garder secrete)</label>
              <div class="webhook-url-row">
                <code class="webhook-url-code">{{ webhookUrl(wh) }}</code>
                <button class="btn-secondary btn-copy" @click="copyUrl(webhookUrl(wh))" :title="copiedId === wh.id ? 'Copie !' : 'Copier'">
                  <Check v-if="copiedId === wh.id" :size="14" />
                  <Copy v-else :size="14" />
                </button>
              </div>
              <div class="webhook-aliases">
                <span class="webhook-alias-label">Alias Slack :</span>
                <code class="webhook-alias-code">{{ webhookUrl(wh) }}/slack</code>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-if="confirmDeleteId !== null" class="modal-overlay" @click.self="confirmDeleteId = null">
      <div class="modal-small">
        <h3>Supprimer le webhook</h3>
        <p class="confirm-text">L'URL deviendra immediatement invalide. Cette action est irreversible.</p>
        <div class="modal-actions">
          <button type="button" class="btn-cancel" @click="confirmDeleteId = null">Annuler</button>
          <button class="btn-danger" @click="deleteWebhook(confirmDeleteId)">Supprimer</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { X, Hash, Volume2, Trash2, ChevronDown, Settings, Shield, Webhook as WebhookIcon, Copy, Check, Camera } from "lucide-vue-next";
import SaveButton from "./ui/SaveButton.vue";
import BaseInput from "./ui/BaseInput.vue";
import BaseTextarea from "./ui/BaseTextarea.vue";
import NumberStepper from "./ui/NumberStepper.vue";
import PermToggle from "./ui/PermToggle.vue";
import { store, activeState, activeServer } from "../store";
import { api, type Webhook } from "../api";
import * as perms from "../permissions";
import { showToast } from "../composables/useToast";

const state = computed(() => activeState());
const channelId = computed(() => store.channelSettingsId);
const channel = computed(() => state.value?.channels.find((c) => c.id === channelId.value));

const activeTab = ref("general");

const tabs = computed(() => {
  const list = [
    { id: "general", label: "General", icon: Settings },
    { id: "permissions", label: "Permissions", icon: Shield },
  ];
  if (channel.value?.kind === "text") {
    list.push({ id: "webhooks", label: "Webhooks", icon: WebhookIcon });
  }
  return list;
});

const permsList = [
  { name: "Voir le channel", flag: perms.VIEW_CHANNELS },
  { name: "Envoyer des messages", flag: perms.SEND_MESSAGES },
  { name: "Historique", flag: perms.READ_MESSAGE_HISTORY },
  { name: "Joindre des fichiers", flag: perms.ATTACH_FILES },
  { name: "Gerer les messages", flag: perms.MANAGE_MESSAGES },
  { name: "Se connecter (vocal)", flag: perms.CONNECT },
  { name: "Parler (vocal)", flag: perms.SPEAK },
];

// General
const channelName = ref("");
const savingName = ref(false);
const nameSaved = ref(false);
const channelDescription = ref("");
const savingDesc = ref(false);
const descSaved = ref(false);
const userLimit = ref(0);
const savingLimit = ref(false);
const limitSaved = ref(false);

async function saveName() {
  const s = activeServer();
  const st = activeState();
  const id = channelId.value;
  if (!s || !st || !id || !channelName.value.trim()) return;

  savingName.value = true;
  try {
    await api.updateChannel(s.url, s.token, id, { name: channelName.value.trim() });
    const ch = st.channels.find((c) => c.id === id);
    if (ch) ch.name = channelName.value.trim();
    nameSaved.value = true;
    setTimeout(() => (nameSaved.value = false), 2500);
  } finally {
    savingName.value = false;
  }
}

async function saveDescription() {
  const s = activeServer();
  const st = activeState();
  const id = channelId.value;
  if (!s || !st || !id) return;

  savingDesc.value = true;
  try {
    await api.updateChannel(s.url, s.token, id, { description: channelDescription.value });
    const ch = st.channels.find((c) => c.id === id);
    if (ch) ch.description = channelDescription.value || null;
    descSaved.value = true;
    setTimeout(() => (descSaved.value = false), 2500);
  } finally {
    savingDesc.value = false;
  }
}

async function saveUserLimit() {
  const s = activeServer();
  const st = activeState();
  const id = channelId.value;
  if (!s || !st || !id) return;

  savingLimit.value = true;
  try {
    const limit = userLimit.value > 0 ? userLimit.value : null;
    await api.updateChannel(s.url, s.token, id, { user_limit: limit });
    const ch = st.channels.find((c) => c.id === id);
    if (ch) ch.user_limit = limit;
    limitSaved.value = true;
    setTimeout(() => (limitSaved.value = false), 2500);
  } finally {
    savingLimit.value = false;
  }
}

// Permissions
const roles = ref<{ id: number; name: string; permissions: number; color: string | null; position: number }[]>([]);
const overwrites = ref<Map<number, { allow: number; deny: number }>>(new Map());
const expandedRole = ref<number | null>(null);

function toggleRole(roleId: number) {
  expandedRole.value = expandedRole.value === roleId ? null : roleId;
}

function hasOverwrite(roleId: number): boolean {
  const ow = overwrites.value.get(roleId);
  return !!ow && (ow.allow !== 0 || ow.deny !== 0);
}

function getState(roleId: number, flag: number): "inherit" | "allow" | "deny" {
  const ow = overwrites.value.get(roleId);
  if (!ow) return "inherit";
  if (ow.allow & flag) return "allow";
  if (ow.deny & flag) return "deny";
  return "inherit";
}

async function setState(roleId: number, flag: number, newState: "inherit" | "allow" | "deny") {
  const s = activeServer();
  const id = channelId.value;
  if (!s || !id) return;

  let ow = overwrites.value.get(roleId) || { allow: 0, deny: 0 };
  ow.allow &= ~flag;
  ow.deny &= ~flag;
  if (newState === "allow") ow.allow |= flag;
  if (newState === "deny") ow.deny |= flag;

  if (ow.allow === 0 && ow.deny === 0) {
    await api.deleteOverwrite(s.url, s.token, id, roleId);
    overwrites.value.delete(roleId);
  } else {
    await api.setOverwrite(s.url, s.token, id, roleId, ow.allow, ow.deny);
    overwrites.value.set(roleId, ow);
  }
  overwrites.value = new Map(overwrites.value);
}

// Webhooks
const webhooks = ref<Webhook[]>([]);
const loadingWebhooks = ref(false);
const newWebhookName = ref("");
const creatingWebhook = ref(false);
const savingWebhookId = ref<number | null>(null);
const confirmDeleteId = ref<number | null>(null);
const copiedId = ref<number | null>(null);
const avatarInputs: Record<number, HTMLInputElement> = {};

function webhookUrl(wh: Webhook): string {
  const s = activeServer();
  return `${s?.url ?? ""}/api/webhooks/${wh.id}/${wh.token}`;
}

/** Resolve a webhook avatar to a fully-qualified URL (server prefix for relative paths). */
function webhookAvatarSrc(wh: Webhook): string | null {
  const raw = wh.avatar_url;
  if (!raw) return null;
  if (raw.startsWith("http://") || raw.startsWith("https://")) return raw;
  const s = activeServer();
  return s ? `${s.url}${raw}` : raw;
}

function triggerAvatarUpload(id: number) {
  avatarInputs[id]?.click();
}

async function onAvatarSelect(id: number, e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  const s = activeServer();
  const cid = channelId.value;
  if (!s || !cid) return;
  try {
    const updated = await api.uploadWebhookAvatar(s.url, s.token, cid, id, file);
    const wh = webhooks.value.find((w) => w.id === id);
    if (wh) Object.assign(wh, updated);
  } catch {
    showToast("Echec de l'upload", "error");
  }
}

async function removeAvatar(id: number) {
  const s = activeServer();
  const cid = channelId.value;
  if (!s || !cid) return;
  try {
    await api.deleteWebhookAvatar(s.url, s.token, cid, id);
    const wh = webhooks.value.find((w) => w.id === id);
    if (wh) wh.avatar_url = null;
  } catch {
    showToast("Echec de la suppression", "error");
  }
}

async function loadWebhooks() {
  const s = activeServer();
  const id = channelId.value;
  if (!s || !id) return;
  loadingWebhooks.value = true;
  try {
    webhooks.value = await api.listWebhooks(s.url, s.token, id);
  } catch {
    webhooks.value = [];
  } finally {
    loadingWebhooks.value = false;
  }
}

async function createWebhook() {
  const s = activeServer();
  const id = channelId.value;
  const name = newWebhookName.value.trim();
  if (!s || !id || !name) return;
  creatingWebhook.value = true;
  try {
    const created = await api.createWebhook(s.url, s.token, id, { name });
    webhooks.value.push(created);
    newWebhookName.value = "";
  } catch {
    showToast("Echec de creation", "error");
  } finally {
    creatingWebhook.value = false;
  }
}

async function saveWebhook(wh: Webhook) {
  const s = activeServer();
  const id = channelId.value;
  if (!s || !id) return;
  savingWebhookId.value = wh.id;
  try {
    const updated = await api.updateWebhook(s.url, s.token, id, wh.id, {
      name: wh.name.trim() || wh.name,
    });
    Object.assign(wh, updated);
    showToast("Webhook mis a jour", "success", 1500);
  } catch {
    showToast("Echec de la mise a jour", "error");
  } finally {
    savingWebhookId.value = null;
  }
}

async function deleteWebhook(id: number) {
  const s = activeServer();
  const cid = channelId.value;
  if (!s || !cid) return;
  try {
    await api.deleteWebhook(s.url, s.token, cid, id);
    webhooks.value = webhooks.value.filter((w) => w.id !== id);
  } catch {
    showToast("Echec de la suppression", "error");
  } finally {
    confirmDeleteId.value = null;
  }
}

async function copyUrl(url: string) {
  try {
    await navigator.clipboard.writeText(url);
    const wh = webhooks.value.find((w) => webhookUrl(w) === url);
    if (wh) {
      copiedId.value = wh.id;
      setTimeout(() => { if (copiedId.value === wh.id) copiedId.value = null; }, 1500);
    }
  } catch {
    showToast("Impossible de copier", "error");
  }
}

watch(activeTab, (t) => {
  if (t === "webhooks" && webhooks.value.length === 0 && !loadingWebhooks.value) {
    loadWebhooks();
  }
});

// Delete
async function handleDelete() {
  const s = activeServer();
  const st = activeState();
  const id = channelId.value;
  if (!s || !st || !id) return;

  await api.deleteChannel(s.url, s.token, id);
  st.channels = st.channels.filter((c) => c.id !== id);
  close();
}

// Init
onMounted(async () => {
  const s = activeServer();
  const id = channelId.value;
  if (!s || !id) return;

  channelName.value = channel.value?.name ?? "";
  channelDescription.value = channel.value?.description ?? "";
  userLimit.value = channel.value?.user_limit ?? 0;

  try {
    const allRoles = await api.listRoles(s.url, s.token);
    // Filter out Owner (ID=1), sort custom roles by position, Membre (ID=2) at the end
    const custom = allRoles.filter(r => r.id > 2).sort((a, b) => a.position - b.position);
    const membre = allRoles.find(r => r.id === 2);
    roles.value = membre ? [...custom, { ...membre, name: "Permissions par defaut" }] : custom;
  } catch {}

  try {
    const list = await api.listOverwrites(s.url, s.token, id);
    overwrites.value = new Map(list.map((o) => [o.role_id, { allow: o.allow, deny: o.deny }]));
  } catch {}
});

function close() {
  store.channelSettingsId = null;
}
</script>

<style scoped>
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
  width: 860px;
  height: 600px;
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

.sidebar-spacer { flex: 1; }

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
.settings-tab.danger { color: var(--danger); }
.settings-tab.danger:hover { background: var(--danger-bg); }

/* ── Content ── */
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
  display: flex;
  align-items: center;
  gap: 8px;
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

/* ── Card ── */
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
  margin-bottom: 8px;
}

.card-hint {
  font-size: 0.75rem;
  color: var(--text-faint);
  margin-bottom: 12px;
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

.toast-success {
  font-size: 0.75rem;
  color: var(--green);
  margin-top: 6px;
}

/* ── Dot ── */
.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
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
  color: var(--text-bright);
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

/* ── Webhooks ── */
.webhook-empty {
  text-align: center;
  padding: 32px 16px;
  font-size: 0.8125rem;
  color: var(--text-faint);
}

.webhook-item {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 12px;
}

.webhook-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.webhook-avatar {
  position: relative;
  width: 44px;
  height: 44px;
  border-radius: 50%;
  background: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  color: var(--text-bright);
  overflow: hidden;
  flex-shrink: 0;
}

.webhook-avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.webhook-avatar.clickable {
  cursor: pointer;
}

.webhook-avatar .avatar-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.55);
  color: var(--text-bright);
  opacity: 0;
  transition: opacity 0.15s;
}

.webhook-avatar.clickable:hover .avatar-overlay {
  opacity: 1;
}

.link-btn {
  background: none;
  border: none;
  padding: 0;
  margin: 0;
  width: auto;
  font: inherit;
  font-size: 0.6875rem;
  color: var(--accent);
  cursor: pointer;
  text-decoration: underline;
  text-underline-offset: 2px;
}

.link-btn:hover {
  background: none;
  color: var(--text-bright);
}

.webhook-meta {
  flex: 1;
  min-width: 0;
}

.webhook-name {
  font-size: 0.9375rem;
  font-weight: 600;
  color: var(--header-primary);
}

.webhook-sub {
  font-size: 0.6875rem;
  color: var(--text-faint);
}

.webhook-action {
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
.webhook-action:hover { background: var(--bg-modifier-hover); color: var(--text-normal); }
.webhook-action.danger:hover { background: var(--danger); color: var(--text-bright); }

.webhook-row {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 8px;
}

.webhook-actions-row {
  flex-direction: row;
  justify-content: flex-end;
  margin-top: 12px;
}

.webhook-label {
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: var(--text-muted);
}

.btn-secondary {
  width: auto;
  padding: 6px 14px;
  font-size: 0.8125rem;
  border-radius: 6px;
  background: var(--bg-tertiary);
  color: var(--text-normal);
  border: none;
  cursor: pointer;
}
.btn-secondary:hover { background: var(--bg-modifier-hover); }
.btn-secondary:disabled { opacity: 0.5; cursor: not-allowed; }

.webhook-url {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--border);
}

.webhook-url-row {
  display: flex;
  gap: 6px;
  align-items: stretch;
  margin-top: 4px;
}

.webhook-url-code {
  flex: 1;
  min-width: 0;
  padding: 8px 10px;
  background: var(--bg-tertiary);
  border-radius: 6px;
  font-family: monospace;
  font-size: 0.75rem;
  color: var(--text-normal);
  overflow-x: auto;
  white-space: nowrap;
}

.btn-copy {
  width: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.webhook-aliases {
  margin-top: 6px;
  display: flex;
  align-items: baseline;
  gap: 6px;
  font-size: 0.75rem;
  color: var(--text-muted);
}

.webhook-alias-label {
  flex-shrink: 0;
}

.webhook-alias-code {
  font-family: monospace;
  font-size: 0.6875rem;
  color: var(--text-faint);
  overflow-x: auto;
  white-space: nowrap;
  min-width: 0;
}

/* ── Delete confirm modal ── */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: var(--overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}

.modal-small {
  background: var(--bg-primary);
  padding: 24px;
  border-radius: 8px;
  width: 360px;
}

.modal-small h3 {
  font-size: 1rem;
  font-weight: 700;
  color: var(--header-primary);
  margin-bottom: 12px;
}

.confirm-text {
  font-size: 0.875rem;
  color: var(--text-muted);
  margin-bottom: 16px;
}

.modal-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.modal-actions button {
  padding: 8px 16px;
  font-size: 0.8125rem;
  border-radius: 6px;
}

.btn-cancel {
  background: transparent;
  color: var(--text-muted);
}

.btn-cancel:hover {
  color: var(--text-normal);
  background: transparent;
}

.btn-danger {
  background: var(--danger);
  color: var(--text-bright);
}

.btn-danger:hover {
  opacity: 0.9;
}

</style>
