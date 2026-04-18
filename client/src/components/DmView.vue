<template>
  <div class="dm-view">
    <div class="dm-header">
      <div class="dm-header-user" @click="openPeerCard">
        <div class="dm-avatar" :style="avatarStyle">
          <img v-if="avatarSrc" :src="avatarSrc" />
          <span v-else>{{ peer?.display_name?.[0]?.toUpperCase() }}</span>
        </div>
        <div class="dm-header-meta">
          <div class="dm-header-name">{{ peer?.display_name ?? `User #${peerId}` }}</div>
          <div class="dm-header-sub">
            <ShieldCheck v-if="peerFingerprint" :size="12" class="fp-ok" />
            <KeyRound v-else :size="12" class="fp-warn" />
            <span v-if="peerFingerprint" class="dm-fingerprint" :title="'Empreinte de la cle : ' + peerFingerprint">
              {{ peerFingerprint }}
            </span>
            <span v-else class="dm-no-key">Pas de cle publique</span>
          </div>
        </div>
      </div>
      <div
        v-if="myFingerprint"
        class="dm-header-mine"
        :class="{ 'key-unsynced': myKeyStatus === 'unsynced' }"
        :title="myKeyTooltip"
      >
        <ShieldCheck v-if="myKeyStatus === 'ok'" :size="11" class="fp-ok" />
        <AlertTriangle v-else :size="11" class="fp-warn" />
        <span>Ma cle · {{ myFingerprint }}</span>
      </div>
      <button class="dm-close-btn" title="Fermer" @click="close">
        <X :size="16" />
      </button>
    </div>

    <div v-if="showWebWarning" class="dm-web-warning">
      <AlertTriangle class="dm-warn-icon" :size="16" />
      <p class="dm-warn-text">
        <strong>Stockage local</strong> — ta cle privee est gardee dans ce navigateur.
        Vider les donnees du site ou changer d'appareil la rendra inaccessible
        et <em>les anciens messages seront perdus</em>. Utilise l'app desktop pour plus de securite.
      </p>
      <button class="dm-warn-close" @click="dismissWebWarning" title="J'ai compris">
        <X :size="14" />
      </button>
    </div>

    <div class="dm-messages" ref="messagesEl" @scroll="onScroll">
      <div v-if="!peer?.public_key" class="dm-empty">
        <KeyRound :size="36" :stroke-width="1.2" />
        <p>En attente de la cle publique de {{ peer?.display_name ?? 'cet utilisateur' }}.</p>
        <p class="dm-empty-sub">Elle est generee automatiquement a sa prochaine connexion.</p>
        <p v-if="myKeyStatus === 'unsynced'" class="dm-empty-warn">
          <AlertTriangle :size="12" />
          Ta propre cle n'est pas publiee au serveur — le serveur est peut-etre pas a jour.
        </p>
        <p v-else-if="myKeyStatus === 'pending'" class="dm-empty-sub">
          (Initialisation de la crypto en cours...)
        </p>
      </div>
      <template v-else>
        <div v-if="loadingOlder" class="dm-loading-older">
          <Loader2 :size="16" class="spinner" /> Chargement...
        </div>
        <div v-else-if="!reachedTop && messages.length >= 50" class="dm-loading-older loaded-all">
          <button class="load-older-btn" @click="onLoadOlder">Charger les anciens messages</button>
        </div>
        <div v-if="!messages.length" class="dm-empty">
          <MessageSquare :size="36" :stroke-width="1.2" />
          <p>Debute la conversation avec {{ peer.display_name }}.</p>
          <p class="dm-empty-sub">Les messages sont chiffres de bout en bout.</p>
        </div>
        <div
          v-for="msg in messages"
          :key="msg.id"
          class="dm-message"
          :class="{ mine: msg.sender_id === myId, pending: msg.pending, undecryptable: msg.undecryptable, editing: editingId === msg.id }"
          :data-msg-id="msg.id"
        >
          <!-- Reply preview -->
          <div v-if="msg.reply_to_id" class="dm-reply-preview" @click="scrollToMessage(msg.reply_to_id!)">
            <CornerDownRight :size="10" />
            <span>{{ replyPreviewText(msg.reply_to_id!) }}</span>
          </div>

          <!-- Actions au hover -->
          <div v-if="editingId !== msg.id && !msg.pending && !msg.undecryptable" class="dm-message-actions">
            <button
              v-for="emoji in quickEmojis"
              :key="emoji"
              class="act-btn quick-emoji"
              :title="emoji"
              @click="onReact(msg.id, emoji)"
            >{{ emoji }}</button>
            <button class="act-btn" title="Repondre" @click="startReply(msg)">
              <Reply :size="13" />
            </button>
            <button v-if="msg.sender_id === myId" class="act-btn" title="Modifier" @click="startEdit(msg)">
              <Pencil :size="13" />
            </button>
            <button v-if="msg.sender_id === myId" class="act-btn danger" title="Supprimer" @click="onDelete(msg.id)">
              <Trash2 :size="13" />
            </button>
          </div>

          <!-- Bulle -->
          <div class="dm-bubble">
            <span v-if="msg.undecryptable" class="dm-undecryptable">
              <LockKeyholeOpen :size="12" />
              Impossible de dechiffrer
            </span>
            <template v-else>
              <textarea
                v-if="editingId === msg.id"
                v-model="editDraft"
                class="edit-input"
                rows="1"
                @keydown.enter.exact.prevent="confirmEdit"
                @keydown.escape="cancelEdit"
                ref="editEl"
              />
              <span v-else>{{ msg.plaintext }}</span>
            </template>
          </div>

          <!-- Réactions -->
          <div v-if="msg.reactions && msg.reactions.length" class="dm-reactions">
            <button
              v-for="r in msg.reactions"
              :key="r.emoji"
              class="reaction-chip"
              :class="{ mine: r.user_ids.includes(myId) }"
              @click="onReact(msg.id, r.emoji)"
              :title="r.user_ids.map(uid => resolveUserName(uid)).join(', ')"
            >
              {{ r.emoji }} <span class="count">{{ r.count }}</span>
            </button>
          </div>

          <div class="dm-meta">
            {{ formatTime(msg.created_at) }}
            <span v-if="msg.edited"> · modifie</span>
            <span v-if="msg.pending"> · Envoi...</span>
          </div>
        </div>
      </template>
    </div>

    <!-- Reply composer indicator -->
    <div v-if="replyingTo" class="dm-reply-bar">
      <CornerDownRight :size="12" />
      <span class="reply-to">Reponse a : {{ replyPreviewText(replyingTo.id) }}</span>
      <button class="reply-cancel" @click="cancelReply" title="Annuler">
        <X :size="12" />
      </button>
    </div>

    <div class="dm-input-area" v-if="peer?.public_key && editingId === null">
      <textarea
        v-model="draft"
        class="dm-input"
        :placeholder="`Message prive a ${peer.display_name}`"
        rows="1"
        @keydown.enter.exact.prevent="onSend"
        @keydown.escape="cancelReply"
        ref="inputEl"
      />
      <button class="dm-send-btn" :disabled="!draft.trim()" @click="onSend" title="Envoyer">
        <Send :size="16" />
      </button>
    </div>

    <UserCard
      v-if="cardOpen && peer"
      :user="peer"
      :x="cardPos.x"
      :y="cardPos.y"
      @close="cardOpen = false"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { X, Send, ShieldCheck, AlertTriangle, MessageSquare, KeyRound, LockKeyholeOpen, Reply, Pencil, Trash2, CornerDownRight, Loader2 } from "lucide-vue-next";
import {
  activeState, closeDm,
  sendDm, editDm, deleteDm, toggleDmReaction, loadOlderDms,
  resolveAvatarUrl, resolveUser,
} from "../store";
import type { DmMessage } from "../api";
import UserCard from "./UserCard.vue";

const state = computed(() => activeState());
const peerId = computed(() => state.value?.activeDmUserId ?? 0);
const peer = computed(() => state.value?.users.get(peerId.value));
const myId = computed(() => state.value?.user?.id ?? -1);

const messages = computed(() => state.value?.dms.get(peerId.value) ?? []);

const peerFingerprint = computed(() => {
  return state.value?.knownFingerprints.get(peerId.value) ?? peer.value?.key_fingerprint ?? null;
});

const myFingerprint = computed(() => state.value?.dmKeypair?.fingerprint ?? null);

const myKeyStatus = computed<"pending" | "unsynced" | "ok">(() => {
  const local = state.value?.dmKeypair?.fingerprint ?? null;
  const server = state.value?.user?.key_fingerprint ?? null;
  if (!local) return "pending";
  if (server !== local) return "unsynced";
  return "ok";
});

const myKeyTooltip = computed(() => {
  if (myKeyStatus.value === "ok") return `Ma cle est publiee : ${myFingerprint.value}`;
  return `Ma cle n'est PAS publiee au serveur (${myFingerprint.value ?? 'pas encore generee'})`;
});

const avatarSrc = computed(() => resolveAvatarUrl(peerId.value));
const avatarStyle = computed(() => avatarSrc.value ? {} : { background: "var(--accent)" });

const draft = ref("");
const editingId = ref<number | null>(null);
const editDraft = ref("");
const replyingTo = ref<DmMessage | null>(null);
const inputEl = ref<HTMLTextAreaElement>();
const editEl = ref<HTMLTextAreaElement>();
const messagesEl = ref<HTMLDivElement>();
const cardOpen = ref(false);
const cardPos = ref({ x: 0, y: 0 });
const loadingOlder = ref(false);
const reachedTop = ref(false);

const quickEmojis = ["👍", "❤️", "😂", "🔥"];

const isTauri = "__TAURI_INTERNALS__" in window;
const warningKey = "dmWebWarningDismissed";
const showWebWarning = ref(!isTauri && localStorage.getItem(warningKey) !== "1");

function dismissWebWarning() {
  showWebWarning.value = false;
  localStorage.setItem(warningKey, "1");
}

async function onSend() {
  const content = draft.value.trim();
  if (!content) return;
  const replyId = replyingTo.value?.id ?? null;
  draft.value = "";
  replyingTo.value = null;
  await sendDm(peerId.value, content, replyId);
  await nextTick();
  scrollToBottom();
}

function close() {
  closeDm();
}

function openPeerCard(e: MouseEvent) {
  cardPos.value = { x: e.clientX, y: e.clientY };
  cardOpen.value = true;
}

function scrollToBottom() {
  const el = messagesEl.value;
  if (el) el.scrollTop = el.scrollHeight;
}

function scrollToMessage(messageId: number) {
  const el = messagesEl.value?.querySelector(`[data-msg-id="${messageId}"]`) as HTMLElement | null;
  if (el) el.scrollIntoView({ behavior: "smooth", block: "center" });
}

function formatTime(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function replyPreviewText(id: number): string {
  const target = messages.value.find((m) => m.id === id);
  if (!target) return "(message introuvable)";
  if (target.undecryptable) return "(message indechiffrable)";
  const txt = target.plaintext ?? "";
  return txt.length > 60 ? txt.slice(0, 60) + "..." : txt;
}

function resolveUserName(uid: number): string {
  return resolveUser(uid);
}

function startReply(msg: DmMessage) {
  replyingTo.value = msg;
  nextTick(() => inputEl.value?.focus());
}
function cancelReply() {
  replyingTo.value = null;
}

function startEdit(msg: DmMessage) {
  editingId.value = msg.id;
  editDraft.value = msg.plaintext ?? "";
  nextTick(() => editEl.value?.focus());
}
async function confirmEdit() {
  const id = editingId.value;
  if (id == null) return;
  const content = editDraft.value.trim();
  editingId.value = null;
  if (!content) return;
  await editDm(id, content);
}
function cancelEdit() {
  editingId.value = null;
}

function onDelete(id: number) {
  if (!confirm("Supprimer ce message ?")) return;
  deleteDm(id);
}

function onReact(id: number, emoji: string) {
  toggleDmReaction(id, emoji);
}

async function onScroll() {
  const el = messagesEl.value;
  if (!el || loadingOlder.value || reachedTop.value) return;
  if (el.scrollTop <= 40) await onLoadOlder();
}

async function onLoadOlder() {
  if (loadingOlder.value) return;
  loadingOlder.value = true;
  const el = messagesEl.value!;
  const prevHeight = el.scrollHeight;
  try {
    const n = await loadOlderDms(peerId.value);
    if (n === 0) reachedTop.value = true;
    await nextTick();
    // Préserve la position visuelle pendant la prepend.
    el.scrollTop = el.scrollHeight - prevHeight;
  } finally {
    loadingOlder.value = false;
  }
}

// Auto-scroll sur nouveau message (seulement si on est proche du bas).
watch(() => messages.value.length, (newLen, oldLen) => {
  const el = messagesEl.value;
  if (!el) return;
  // Ne scroll que si on était déjà proche du bas — évite de forcer le scroll pendant le load-older.
  const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 120;
  if (newLen > oldLen && nearBottom) {
    nextTick(scrollToBottom);
  }
});

watch(peerId, () => {
  replyingTo.value = null;
  editingId.value = null;
  reachedTop.value = false;
  nextTick(() => {
    inputEl.value?.focus();
    scrollToBottom();
  });
}, { immediate: true });
</script>

<style scoped>
.dm-view {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  background: var(--bg-primary);
}

.dm-header {
  height: 48px;
  padding: 0 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.dm-header-user {
  display: flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
  flex: 1;
  min-width: 0;
}
.dm-avatar {
  width: 28px; height: 28px;
  border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  font-weight: 600; font-size: 0.75rem;
  color: var(--text-bright);
  overflow: hidden; flex-shrink: 0;
}
.dm-avatar img { width: 100%; height: 100%; object-fit: cover; }
.dm-header-meta { display: flex; flex-direction: column; min-width: 0; }
.dm-header-name { font-weight: 600; font-size: 0.9375rem; color: var(--header-primary); }
.dm-header-sub {
  display: flex; align-items: center; gap: 4px;
  font-size: 0.6875rem; color: var(--text-faint); font-family: monospace;
}
.dm-fingerprint { color: var(--green, #3ba55c); }
.dm-no-key {
  color: color-mix(in srgb, var(--yellow, #facc15) 85%, var(--text-faint));
  font-family: inherit; font-style: italic;
}
.fp-ok { color: var(--green, #3ba55c); flex-shrink: 0; }
.fp-warn { color: var(--yellow, #facc15); flex-shrink: 0; }
.dm-header-mine {
  display: flex; align-items: center; gap: 4px;
  font-size: 0.6875rem; font-family: monospace; color: var(--text-muted);
  padding: 0 12px; border-left: 1px solid var(--border);
}
.dm-header-mine.key-unsynced { color: var(--yellow, #facc15); }

.dm-close-btn {
  width: 28px; height: 28px; padding: 0; margin: 0;
  display: flex; align-items: center; justify-content: center;
  background: transparent; color: var(--text-muted);
  border: none; cursor: pointer; border-radius: 6px;
}
.dm-close-btn:hover { background: var(--bg-modifier-hover); color: var(--text-normal); box-shadow: none; }

.dm-web-warning {
  display: flex; align-items: flex-start; gap: 10px;
  margin: 8px 16px; padding: 10px 12px;
  background: color-mix(in srgb, var(--yellow, #facc15) 12%, transparent);
  border: 1px solid color-mix(in srgb, var(--yellow, #facc15) 30%, transparent);
  border-radius: 8px;
}
.dm-warn-icon { color: var(--yellow, #facc15); flex-shrink: 0; margin-top: 2px; }
.dm-warn-text { flex: 1 1 auto; min-width: 0; margin: 0; font-size: 0.75rem; line-height: 1.45; color: var(--text-normal); }
.dm-warn-text strong { color: var(--yellow, #facc15); }
.dm-warn-close {
  flex-shrink: 0; width: 22px; height: 22px;
  padding: 0; margin: 0;
  display: flex; align-items: center; justify-content: center;
  background: transparent; border: none; color: var(--text-muted);
  cursor: pointer; border-radius: 4px;
}
.dm-warn-close:hover { color: var(--text-normal); background: var(--bg-modifier-hover); box-shadow: none; }

.dm-messages {
  flex: 1; overflow-y: auto;
  display: flex; flex-direction: column;
  padding: 16px; gap: 6px;
}

.dm-loading-older {
  display: flex; align-items: center; justify-content: center;
  gap: 6px; padding: 6px;
  font-size: 0.75rem; color: var(--text-faint);
}
.load-older-btn {
  padding: 4px 10px; width: auto; margin: 0;
  background: transparent; border: 1px solid var(--border);
  color: var(--text-muted); font-size: 0.75rem; border-radius: 10px;
  cursor: pointer;
}
.load-older-btn:hover { background: var(--bg-modifier-hover); color: var(--text-normal); box-shadow: none; }
.spinner { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.dm-empty {
  flex: 1; display: flex; flex-direction: column;
  align-items: center; justify-content: center;
  color: var(--text-faint); gap: 6px; text-align: center;
  padding: 40px 16px;
}
.dm-empty-sub { font-size: 0.75rem; }
.dm-empty-warn {
  display: flex; align-items: center; gap: 6px;
  margin-top: 8px; padding: 6px 10px;
  font-size: 0.6875rem; color: var(--yellow, #facc15);
  border: 1px solid color-mix(in srgb, var(--yellow, #facc15) 30%, transparent);
  border-radius: 6px;
}

.dm-message {
  display: flex; flex-direction: column;
  align-items: flex-start;
  max-width: 75%;
  position: relative;
}
.dm-message.mine { align-self: flex-end; align-items: flex-end; }

.dm-reply-preview {
  display: inline-flex; align-items: center; gap: 4px;
  max-width: 100%;
  padding: 2px 8px;
  background: var(--bg-secondary);
  border-left: 2px solid var(--accent);
  border-radius: 4px;
  font-size: 0.6875rem; color: var(--text-muted);
  cursor: pointer;
  margin-bottom: 2px;
}
.dm-reply-preview:hover { background: var(--bg-modifier-hover); }
.dm-reply-preview span { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

.dm-message-actions {
  position: absolute;
  top: -14px;
  right: 4px;
  display: none;
  gap: 2px;
  padding: 2px;
  background: var(--bg-floating);
  border: 1px solid var(--border);
  border-radius: 6px;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
  z-index: 2;
}
.dm-message.mine .dm-message-actions { right: auto; left: 4px; }
.dm-message:hover .dm-message-actions { display: flex; }
.act-btn {
  width: 24px; height: 24px; padding: 0; margin: 0;
  display: flex; align-items: center; justify-content: center;
  background: transparent; border: none; color: var(--text-muted);
  font-size: 0.875rem; cursor: pointer; border-radius: 4px;
}
.act-btn:hover { background: var(--bg-modifier-hover); color: var(--text-normal); box-shadow: none; }
.act-btn.danger:hover { color: var(--danger, #ed4245); background: color-mix(in srgb, var(--danger, #ed4245) 15%, transparent); }
.act-btn.quick-emoji { font-size: 0.9rem; }

.dm-bubble {
  padding: 8px 12px;
  background: var(--bg-tertiary);
  border-radius: 12px 12px 12px 4px;
  color: var(--text-normal);
  font-size: 0.875rem; line-height: 1.4;
  white-space: pre-wrap; word-wrap: break-word;
}
.dm-message.mine .dm-bubble {
  background: var(--accent);
  color: var(--text-bright);
  border-radius: 12px 12px 4px 12px;
}
.dm-message.pending .dm-bubble { opacity: 0.6; }
.dm-message.undecryptable .dm-bubble {
  background: transparent;
  border: 1px dashed var(--border);
  color: var(--text-faint); font-style: italic;
}
.dm-message.editing .dm-bubble { outline: 2px solid var(--accent); }
.edit-input {
  width: 100%; min-width: 200px;
  background: transparent; border: none; outline: none;
  color: inherit; font-family: inherit; font-size: inherit;
  resize: none;
}

.dm-undecryptable {
  display: inline-flex; align-items: center; gap: 6px; font-size: 0.75rem;
}

.dm-reactions {
  display: flex; flex-wrap: wrap; gap: 4px;
  margin-top: 4px;
}
.reaction-chip {
  display: inline-flex; align-items: center; gap: 4px;
  padding: 2px 8px; width: auto; margin: 0;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 10px;
  font-size: 0.75rem; color: var(--text-normal);
  cursor: pointer;
}
.reaction-chip:hover { background: var(--bg-modifier-hover); }
.reaction-chip.mine {
  background: color-mix(in srgb, var(--accent) 20%, transparent);
  border-color: var(--accent);
}
.reaction-chip .count { font-size: 0.6875rem; color: var(--text-muted); }
.reaction-chip.mine .count { color: var(--accent); }

.dm-meta {
  font-size: 0.6875rem; color: var(--text-faint);
  margin-top: 2px; padding: 0 4px;
}

.dm-reply-bar {
  display: flex; align-items: center; gap: 8px;
  padding: 6px 16px;
  background: var(--bg-secondary);
  border-top: 1px solid var(--border);
  font-size: 0.75rem; color: var(--text-muted);
}
.dm-reply-bar .reply-to { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.reply-cancel {
  width: 20px; height: 20px; padding: 0; margin: 0;
  display: flex; align-items: center; justify-content: center;
  background: transparent; border: none; color: var(--text-faint);
  cursor: pointer; border-radius: 4px;
}
.reply-cancel:hover { color: var(--text-normal); background: var(--bg-modifier-hover); box-shadow: none; }

.dm-input-area {
  padding: 8px 16px 16px;
  display: flex; gap: 8px; align-items: flex-end;
  flex-shrink: 0;
}
.dm-input {
  flex: 1;
  padding: 10px 12px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text-normal);
  font-size: 0.875rem; font-family: inherit;
  resize: none; max-height: 160px;
}
.dm-input:focus { outline: none; border-color: var(--accent); }
.dm-send-btn {
  width: 40px; height: 40px;
  border-radius: 8px;
  background: var(--accent); color: var(--text-bright);
  border: none; cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  flex-shrink: 0; padding: 0; margin: 0;
}
.dm-send-btn:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
