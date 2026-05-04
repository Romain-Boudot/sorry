<template>
  <div v-if="!peerId" class="dm-view dm-view-empty">
    <MessageSquare :size="48" :stroke-width="1.2" />
    <h3>Aucun message privé</h3>
    <p>Sélectionne une conversation à gauche ou démarre-en une nouvelle.</p>
  </div>
  <div v-else class="dm-view">
    <!-- Header (DM-specific: peer info + fingerprints + close) -->
    <div class="dm-header">
      <div class="dm-header-user" @click="openPeerCard">
        <div class="dm-avatar" :style="avatarStyle">
          <img v-if="peerAvatar" :src="peerAvatar" />
          <span v-else>{{ peerInitial }}</span>
        </div>
        <div class="dm-header-meta">
          <div class="dm-header-name">{{ peerName }}</div>
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

    <!-- Web key storage warning -->
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

    <!-- Messages -->
    <div class="dm-messages" ref="messagesContainer" @scroll="onScroll">
      <div v-if="!peer?.public_key" class="dm-empty">
        <KeyRound :size="36" :stroke-width="1.2" />
        <p>En attente de la cle publique de {{ peerName }}.</p>
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
        <div v-if="loadingOlder" class="loading-older">
          <Loader2 :size="16" class="spinner" /> Chargement...
        </div>
        <div v-else-if="!reachedTop && messages.length >= 50" class="load-older-row">
          <button class="load-older-btn" @click="onLoadOlder">Charger les anciens messages</button>
        </div>

        <div v-if="!messages.length" class="dm-empty">
          <MessageSquare :size="36" :stroke-width="1.2" />
          <p>Debute la conversation avec {{ peerName }}.</p>
          <p class="dm-empty-sub">Les messages sont chiffres de bout en bout.</p>
        </div>

        <MessageItem
          v-for="(msg, i) in messages"
          :key="msg.id"
          :id="msg.id"
          :author-name="resolveAuthorName(msg.sender_id)"
          :author-avatar="resolveAvatarUrl(msg.sender_id)"
          :author-color="resolveUserColor(msg.sender_id)"
          :content="msg.plaintext ?? ''"
          :created-at="msg.created_at"
          :edited="msg.edited"
          :pending="msg.pending"
          :undecryptable="msg.undecryptable"
          :grouped="isGrouped(i)"
          :reactions="msg.reactions"
          :reply-to="buildReplyPreview(msg)"
          :my-user-id="myId"
          :can-open-card="msg.sender_id !== myId"
          :actions="buildActions(msg)"
          :editing="editingId === msg.id"
          :edit-content="editDraft"
          :pending-label="msg.pending ? 'Envoi...' : ''"
          @react="(emoji) => onReact(msg.id, emoji)"
          @open-emoji-picker="(e) => openEmojiPicker(msg.id, e)"
          @open-author-card="openPeerCard"
          @scroll-to-reply="scrollToMessage"
          @update:edit-content="editDraft = $event"
          @submit-edit="confirmEdit"
          @cancel-edit="cancelEdit"
        />
      </template>
    </div>

    <!-- Composer (only when peer key is known and we're not editing) -->
    <Composer
      v-if="peer?.public_key && editingId === null"
      ref="composerRef"
      v-model="draft"
      :placeholder="`Message prive a ${peerName}`"
      :disabled="!draft.trim()"
      :replying-to="replyContext"
      :scroll-container="messagesContainer ?? null"
      :is-scrolled-to-bottom="isScrolledToBottom"
      @submit="onSend"
      @cancel-reply="cancelReply"
    />

    <UserCard
      v-if="cardOpen && peer"
      :user="peer"
      :x="cardPos.x"
      :y="cardPos.y"
      @close="cardOpen = false"
    />

    <EmojiPicker
      v-if="emojiPicker"
      :x="emojiPicker.x"
      :y="emojiPicker.y"
      @select="onEmojiSelect"
      @close="emojiPicker = null"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import {
  X, ShieldCheck, AlertTriangle, MessageSquare, KeyRound, Reply, Pencil, Trash2, SmilePlus, Loader2,
} from "lucide-vue-next";
import {
  activeState, closeDm,
  sendDm, editDm, deleteDm, toggleDmReaction, loadOlderDms,
  resolveAvatarUrl, resolveUser, resolveUserColor,
} from "../store";
import type { DmMessage } from "../api";
import { topEmojis, recordEmoji } from "../composables/useEmojiFrequency";
import UserCard from "./UserCard.vue";
import EmojiPicker from "./chat/EmojiPicker.vue";
import Composer from "./chat/Composer.vue";
import MessageItem, { type MessageAction, type ReplyPreviewData } from "./chat/MessageItem.vue";

const state = computed(() => activeState());
const peerId = computed(() => state.value?.activeDmUserId ?? 0);
const peer = computed(() => state.value?.users.get(peerId.value));
const myId = computed(() => state.value?.user?.id ?? -1);

const messages = computed(() => state.value?.dms.get(peerId.value) ?? []);

const peerName = computed(() => peer.value?.display_name ?? `User #${peerId.value}`);
const peerInitial = computed(() => (peerName.value[0] ?? "?").toUpperCase());
const peerAvatar = computed(() => resolveAvatarUrl(peerId.value));
const avatarStyle = computed(() => peerAvatar.value ? {} : { background: "var(--accent)" });

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

// ── Composer state ──
const draft = ref("");
const composerRef = ref<InstanceType<typeof Composer>>();
const replyingTo = ref<DmMessage | null>(null);

const replyContext = computed(() => {
  if (!replyingTo.value) return null;
  return {
    authorName: resolveAuthorName(replyingTo.value.sender_id),
    content: replyingTo.value.undecryptable ? "(message indechiffrable)" : (replyingTo.value.plaintext ?? ""),
  };
});

// ── Edit state ──
const editingId = ref<number | null>(null);
const editDraft = ref("");

// ── Scroll ──
const messagesContainer = ref<HTMLDivElement>();
const loadingOlder = ref(false);
const reachedTop = ref(false);

// ── User card popup (peer) ──
const cardOpen = ref(false);
const cardPos = ref({ x: 0, y: 0 });

// ── Emoji picker ──
const emojiPicker = ref<{ x: number; y: number; messageId: number } | null>(null);
const quickEmojis = ref(topEmojis());

// ── Browser key warning ──
const isTauri = "__TAURI_INTERNALS__" in window;
const warningKey = "dmWebWarningDismissed";
const showWebWarning = ref(!isTauri && localStorage.getItem(warningKey) !== "1");

function dismissWebWarning() {
  showWebWarning.value = false;
  localStorage.setItem(warningKey, "1");
}

// ── Display helpers ──
function resolveAuthorName(userId: number): string {
  if (userId === myId.value) return state.value?.user?.display_name ?? "Moi";
  return resolveUser(userId);
}

/** Max time between a group's first message and any of its messages (ms). */
const GROUP_MAX_SPAN_MS = 5 * 60 * 1000;

// Single-pass build of the set of grouped indices. A message is grouped under
// the previous one when the same author posts within GROUP_MAX_SPAN_MS of the
// CURRENT GROUP HEAD (not the previous message), so long bursts naturally split
// once they exceed the span.
const groupedIndices = computed(() => {
  const set = new Set<number>();
  let headTime = 0;
  for (let i = 0; i < messages.value.length; i++) {
    const msg = messages.value[i];
    const t = parseTs(msg.created_at);
    let grouped = false;
    if (i > 0 && !msg.reply_to_id) {
      const prev = messages.value[i - 1];
      if (msg.sender_id === prev.sender_id && t - headTime < GROUP_MAX_SPAN_MS) grouped = true;
    }
    if (grouped) set.add(i);
    else headTime = t;
  }
  return set;
});

function isGrouped(index: number): boolean {
  return groupedIndices.value.has(index);
}

// Optimistic DMs use new Date().toISOString() (with trailing Z); server payloads come without Z.
function parseTs(ts: string): number {
  return new Date(ts.endsWith("Z") ? ts : ts + "Z").getTime();
}

function buildReplyPreview(msg: DmMessage): ReplyPreviewData | null {
  if (!msg.reply_to_id) return null;
  const target = messages.value.find((m) => m.id === msg.reply_to_id);
  const authorId = target?.sender_id ?? 0;
  const content = target
    ? (target.undecryptable ? "(message indechiffrable)" : (target.plaintext ?? ""))
    : "(message introuvable)";
  return {
    id: msg.reply_to_id,
    authorName: target ? resolveAuthorName(authorId) : "?",
    authorColor: target ? resolveUserColor(authorId) : null,
    avatarUrl: target ? resolveAvatarUrl(authorId) : null,
    content,
  };
}

function buildActions(msg: DmMessage): MessageAction[] {
  if (msg.pending || msg.undecryptable) return [];
  const items: MessageAction[] = [];
  for (const emoji of quickEmojis.value) {
    items.push({
      key: `quick-${emoji}`,
      label: emoji,
      emoji,
      handler: () => onReact(msg.id, emoji),
    });
  }
  items.push({
    key: "picker",
    label: "Reaction",
    icon: SmilePlus,
    handler: (e) => openEmojiPicker(msg.id, e),
  });
  items.push({
    key: "reply",
    label: "Repondre",
    icon: Reply,
    handler: () => startReply(msg),
  });
  if (msg.sender_id === myId.value) {
    items.push({
      key: "edit",
      label: "Modifier",
      icon: Pencil,
      handler: () => startEdit(msg),
    });
    items.push({
      key: "delete",
      label: "Supprimer",
      icon: Trash2,
      danger: true,
      handler: () => onDelete(msg.id),
    });
  }
  return items;
}

// ── Send / edit / delete / react ──
async function onSend() {
  const content = draft.value.trim();
  if (!content) return;
  const replyId = replyingTo.value?.id ?? null;
  draft.value = "";
  replyingTo.value = null;
  await sendDm(peerId.value, content, replyId);
  await nextTick();
  composerRef.value?.reset();
  scrollToBottom();
}

function startReply(msg: DmMessage) {
  replyingTo.value = msg;
  nextTick(() => composerRef.value?.focus());
}

function cancelReply() {
  replyingTo.value = null;
}

function startEdit(msg: DmMessage) {
  editingId.value = msg.id;
  editDraft.value = msg.plaintext ?? "";
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
  recordEmoji(emoji);
  toggleDmReaction(id, emoji);
  quickEmojis.value = topEmojis();
}

function openEmojiPicker(messageId: number, e: MouseEvent | Event) {
  const me = e as MouseEvent;
  const x = Math.min(me.clientX, window.innerWidth - 300);
  const y = Math.max(me.clientY - 280, 8);
  emojiPicker.value = { x, y, messageId };
}

function onEmojiSelect(emoji: string) {
  if (!emojiPicker.value) return;
  onReact(emojiPicker.value.messageId, emoji);
  emojiPicker.value = null;
}

// ── Navigation / scrolling ──
function close() {
  closeDm();
}

function openPeerCard(e: MouseEvent) {
  cardPos.value = { x: e.clientX, y: e.clientY };
  cardOpen.value = true;
}

function scrollToBottom() {
  const el = messagesContainer.value;
  if (el) el.scrollTop = el.scrollHeight;
}

function scrollToMessage(messageId: number) {
  const el = messagesContainer.value?.querySelector(`[data-msg-id="${messageId}"]`) as HTMLElement | null;
  if (el) {
    el.scrollIntoView({ behavior: "smooth", block: "center" });
    el.classList.add("message-highlight");
    setTimeout(() => el.classList.remove("message-highlight"), 2000);
  }
}

function isScrolledToBottom(): boolean {
  const el = messagesContainer.value;
  if (!el) return true;
  return el.scrollHeight - el.scrollTop - el.clientHeight < 30;
}

async function onScroll() {
  const el = messagesContainer.value;
  if (!el || loadingOlder.value || reachedTop.value) return;
  if (el.scrollTop <= 40) await onLoadOlder();
}

async function onLoadOlder() {
  if (loadingOlder.value) return;
  loadingOlder.value = true;
  const el = messagesContainer.value!;
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
  const el = messagesContainer.value;
  if (!el) return;
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
    composerRef.value?.focus();
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

.dm-view-empty {
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: var(--text-faint);
  text-align: center;
  padding: 24px;
}

.dm-view-empty h3 {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-muted);
}

.dm-view-empty p {
  margin: 0;
  font-size: 0.875rem;
  max-width: 320px;
}

/* ── Header (DM-specific) ── */
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

/* ── Web key storage warning ── */
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

/* ── Messages container ── */
.dm-messages {
  flex: 1; overflow-y: auto;
  padding: 16px;
}

.loading-older,
.load-older-row {
  display: flex; align-items: center; justify-content: center;
  gap: 6px; padding: 8px;
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

/* ── Highlight on scroll-to-reply ── */
:deep(.message-highlight) {
  background: rgba(88, 101, 242, 0.1);
  transition: background 0.3s;
}
</style>
