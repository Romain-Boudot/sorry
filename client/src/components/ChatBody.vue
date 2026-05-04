<template>
  <div class="chat-body" @dragover="onDragOver" @dragleave="onDragLeave" @drop="onDrop">
    <!-- Skeleton while connecting -->
    <div v-if="!state?.connected" class="chat-skeleton">
      <div v-for="i in 6" :key="i" class="skeleton-message" :class="{ grouped: i % 3 !== 1 }">
        <template v-if="i % 3 === 1">
          <div class="skeleton skeleton-avatar"></div>
          <div class="skeleton-msg-body">
            <div class="skeleton-msg-header">
              <div class="skeleton skeleton-text" :style="{ width: (60 + Math.random() * 60) + 'px', height: '14px' }"></div>
              <div class="skeleton skeleton-text" style="width: 40px; height: 10px;"></div>
            </div>
            <div class="skeleton skeleton-text" :style="{ width: (150 + Math.random() * 200) + 'px', height: '14px' }"></div>
          </div>
        </template>
        <template v-else>
          <div class="skeleton-gutter"></div>
          <div class="skeleton-msg-body">
            <div class="skeleton skeleton-text" :style="{ width: (100 + Math.random() * 250) + 'px', height: '14px' }"></div>
          </div>
        </template>
      </div>
    </div>

    <template v-else>
    <div v-if="dragging" class="drop-overlay">
      <div class="drop-overlay-inner">
        <Paperclip :size="40" :stroke-width="1.2" />
        <p>Depose tes fichiers ici</p>
      </div>
    </div>
    <div class="chat-messages" ref="messagesContainer" @scroll="onMessagesScroll">
      <div ref="messagesInner">
      <div v-if="loadingOlder" class="loading-older">
        <Loader2 :size="18" class="spinner" />
        Chargement...
      </div>
      <div v-if="!messages.length" class="chat-empty">
        <MessageSquare :size="40" :stroke-width="1.2" />
        <p>Aucun message dans #{{ activeChannel?.name }}</p>
        <p class="chat-empty-sub">Sois le premier !</p>
      </div>
      <template v-for="(msg, i) in messages" :key="msg.id">
        <div v-if="showDateSeparator(i)" class="date-separator">
          <span>{{ formatDate(msg.created_at) }}</span>
        </div>
        <MessageItem
          :id="msg.id"
          :author-name="displayName(msg)"
          :author-avatar="displayAvatar(msg)"
          :author-color="displayColor(msg)"
          :content="msg.content"
          :created-at="msg.created_at"
          :pinned="msg.pinned"
          :pending="msg.pending"
          :failed="msg.failed"
          :nonce="msg.nonce ?? null"
          :is-webhook="isWebhookMessage(msg)"
          :is-guest="!isWebhookMessage(msg) && isGuest(msg.author_id)"
          :can-open-card="!isWebhookMessage(msg)"
          :grouped="isGrouped(i)"
          :reactions="msg.reactions"
          :attachments="msg.attachments"
          :reply-to="buildReplyPreview(msg)"
          :my-user-id="state?.user?.id ?? 0"
          :actions="buildActions(msg)"
          :editing="editingMessageId === msg.id"
          :edit-content="editContent"
          @react="(emoji) => onToggleReaction(msg.id, emoji)"
          @open-emoji-picker="(e) => openEmojiPicker(msg.id, e)"
          @open-author-card="(e) => openCard(msg.author_id, e)"
          @scroll-to-reply="scrollToMessage"
          @context-menu="(e) => onMessageContextMenu(msg, e)"
          @update:edit-content="editContent = $event"
          @submit-edit="submitEdit"
          @cancel-edit="cancelEdit"
          @retry="onRetry(msg.nonce!)"
          @discard="onDiscard(msg.nonce!)"
        />
      </template>
      </div>
    </div>

    <div v-if="typingText" class="typing-indicator">
      <span class="typing-dots"><span></span><span></span><span></span></span>
      {{ typingText }}
    </div>

    <ChatInput
      ref="chatInputRef"
      :channel-name="activeChannel?.name ?? '...'"
      :replying-to="replyingTo"
      :is-scrolled-to-bottom="isScrolledToBottom"
      :messages-container="messagesContainer"
      @cancel-reply="replyingTo = null"
      @sent="onMessageSent"
      @edit-last="startEdit"
    />

    <ContextMenu
      v-if="ctxMenu"
      :x="ctxMenu.x"
      :y="ctxMenu.y"
      :items="ctxMenu.items"
      @close="ctxMenu = null"
    />

    <EmojiPicker
      v-if="emojiPicker"
      :x="emojiPicker.x"
      :y="emojiPicker.y"
      @select="onEmojiSelect"
      @close="emojiPicker = null"
    />

    <UserCard
      v-if="cardUser"
      :user="cardUser"
      :x="cardX"
      :y="cardY"
      @close="cardUser = null"
    />

    <div v-if="confirmDeleteId" class="modal-overlay" @click.self="confirmDeleteId = null">
      <div class="modal-small">
        <h3>Supprimer le message</h3>
        <p class="confirm-text">Es-tu sur de vouloir supprimer ce message ?</p>
        <div class="modal-actions">
          <button type="button" class="btn-cancel" @click="confirmDeleteId = null">Annuler</button>
          <button class="btn-danger" @click="confirmDelete">Supprimer</button>
        </div>
      </div>
    </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, nextTick, onMounted, onUnmounted } from "vue";
import { MessageSquare, Pencil, Trash2, Paperclip, Loader2, Reply, SmilePlus, Pin, PinOff } from "lucide-vue-next";
import { activeState, activeServer, editMessage, deleteMessage, toggleReaction, resolveUser, resolveUserColor, resolveAvatarUrl, isGuest, retryMessage, discardFailedMessage, resolveWebhookName, resolveWebhookAvatar, isWebhookMessage } from "../store";
import { topEmojis, recordEmoji } from "../composables/useEmojiFrequency";
import * as perms from "../permissions";
import { api, type Message, type User } from "../api";
import ContextMenu, { type MenuItem } from "./ui/ContextMenu.vue";
import UserCard from "./UserCard.vue";
import EmojiPicker from "./chat/EmojiPicker.vue";
import ChatInput from "./chat/ChatInput.vue";
import MessageItem, { type MessageAction, type ReplyPreviewData } from "./chat/MessageItem.vue";

const typingText = computed(() => {
  const st = state.value;
  if (!st?.activeChannelId) return "";
  const channelTyping = st.typingUsers.get(st.activeChannelId);
  if (!channelTyping || channelTyping.size === 0) return "";
  const names = Array.from(channelTyping.keys())
    .map((uid) => st.users.get(uid)?.display_name ?? `User #${uid}`)
    .slice(0, 3);
  if (names.length === 1) return `${names[0]} est en train d'ecrire...`;
  if (names.length === 2) return `${names[0]} et ${names[1]} sont en train d'ecrire...`;
  return `${names[0]}, ${names[1]} et d'autres sont en train d'ecrire...`;
});

const messagesContainer = ref<HTMLElement>();
const chatInputRef = ref<InstanceType<typeof ChatInput>>();
const editingMessageId = ref<number | null>(null);
const editContent = ref("");
const replyingTo = ref<Message | null>(null);
const dragging = ref(false);

const state = computed(() => activeState());

const activeChannel = computed(() =>
  state.value?.channels.find((c) => c.id === state.value?.activeChannelId)
);

const messages = computed(() =>
  state.value?.messages.get(state.value?.activeChannelId ?? 0) ?? []
);

const loadingOlder = ref(false);
const noMoreMessages = ref(false);

function scrollToBottom() {
  wasAtBottom = true;
  nextTick(() => {
    const el = messagesContainer.value;
    if (el) el.scrollTop = el.scrollHeight;
  });
}

watch(() => messages.value.length, (newLen, oldLen) => {
  if (!oldLen) { scrollToBottom(); return; }
  if (newLen <= oldLen) return;
  if (isScrolledToBottom()) scrollToBottom();
});

// Auto-scroll when content height grows (images loaded, reactions added, etc.)
// but only if user was already at the bottom.
let wasAtBottom = true;
const messagesInner = ref<HTMLElement>();
let resizeObserver: ResizeObserver | null = null;

function setupResizeObserver() {
  resizeObserver?.disconnect();
  const inner = messagesInner.value;
  const container = messagesContainer.value;
  if (!inner || !container) return;
  resizeObserver = new ResizeObserver(() => {
    if (wasAtBottom) {
      container.scrollTop = container.scrollHeight;
    }
  });
  resizeObserver.observe(inner);
}

watch(() => state.value?.connected, (connected) => {
  if (connected) {
    scrollToBottom();
    nextTick(setupResizeObserver);
  }
});

watch(() => state.value?.activeChannelId, () => {
  noMoreMessages.value = false;
  scrollToBottom();
  nextTick(setupResizeObserver);
});

onMounted(() => {
  scrollToBottom();
  nextTick(setupResizeObserver);
});

onUnmounted(() => {
  resizeObserver?.disconnect();
});

async function loadOlderMessages() {
  const server = activeServer();
  const st = activeState();
  if (!server || !st?.activeChannelId) return;
  if (loadingOlder.value || noMoreMessages.value) return;

  const msgs = st.messages.get(st.activeChannelId);
  if (!msgs || msgs.length === 0) return;

  const oldestId = msgs[0].id;
  loadingOlder.value = true;

  try {
    const older = await api.listMessages(server.url, server.token, st.activeChannelId, 50, oldestId);
    if (older.length === 0) {
      noMoreMessages.value = true;
      return;
    }
    const el = messagesContainer.value;
    const prevScrollHeight = el?.scrollHeight ?? 0;

    msgs.unshift(...older.reverse());

    nextTick(() => {
      if (el) {
        el.scrollTop = el.scrollHeight - prevScrollHeight;
      }
    });

    if (older.length < 50) {
      noMoreMessages.value = true;
    }
  } finally {
    loadingOlder.value = false;
  }
}

function onMessagesScroll() {
  const el = messagesContainer.value;
  if (!el) return;
  wasAtBottom = isScrolledToBottom();
  if (el.scrollTop < 100) {
    loadOlderMessages();
  }
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
    const t = new Date(msg.created_at + "Z").getTime();
    let grouped = false;
    if (i > 0 && !msg.reply_to) {
      const prev = messages.value[i - 1];
      const sameAuthor = isWebhookMessage(msg) || isWebhookMessage(prev)
        ? isWebhookMessage(msg) === isWebhookMessage(prev) && displayName(msg) === displayName(prev)
        : msg.author_id === prev.author_id;
      if (sameAuthor && t - headTime < GROUP_MAX_SPAN_MS) grouped = true;
    }
    if (grouped) set.add(i);
    else headTime = t;
  }
  return set;
});

function isGrouped(index: number): boolean {
  return groupedIndices.value.has(index);
}

function displayName(msg: Message): string {
  return isWebhookMessage(msg) ? resolveWebhookName(msg) : resolveUser(msg.author_id);
}

function displayAvatar(msg: Message): string | null {
  return isWebhookMessage(msg) ? resolveWebhookAvatar(msg) : resolveAvatarUrl(msg.author_id);
}

function displayColor(msg: Message): string | null {
  return isWebhookMessage(msg) ? null : resolveUserColor(msg.author_id);
}

function buildReplyPreview(msg: Message): ReplyPreviewData | null {
  if (!msg.reply_to) return null;
  return {
    id: msg.reply_to.id,
    authorName: resolveUser(msg.reply_to.author_id),
    authorColor: resolveUserColor(msg.reply_to.author_id),
    avatarUrl: resolveAvatarUrl(msg.reply_to.author_id),
    content: msg.reply_to.content,
  };
}

function showDateSeparator(index: number): boolean {
  if (index === 0) return true;
  const msg = messages.value[index];
  const prev = messages.value[index - 1];
  return new Date(msg.created_at + "Z").toDateString() !== new Date(prev.created_at + "Z").toDateString();
}

function isScrolledToBottom(): boolean {
  const el = messagesContainer.value;
  if (!el) return true;
  return el.scrollHeight - el.scrollTop - el.clientHeight < 30;
}

function trimMessage(s: string): string {
  return s.replace(/^\s*\n/, "").replace(/\n\s*$/, "").trim();
}

function startReply(msg: Message) {
  replyingTo.value = msg;
  nextTick(() => chatInputRef.value?.focus());
}

function onMessageSent() {
  // scrollToBottom is handled by the watcher on messages.value.length
}

function scrollToMessage(messageId: number) {
  const el = messagesContainer.value?.querySelector(`[data-msg-id="${messageId}"]`);
  if (el) {
    el.scrollIntoView({ behavior: "smooth", block: "center" });
    el.classList.add("message-highlight");
    setTimeout(() => el.classList.remove("message-highlight"), 2000);
  }
}

defineExpose({ scrollToMessage });

function startEdit(msg: Message) {
  editingMessageId.value = msg.id;
  editContent.value = msg.content;
  nextTick(() => {
    const container = messagesContainer.value;
    if (!container) return;
    const msgEl = container.querySelector(`.message.editing`) as HTMLElement;
    if (msgEl) {
      const rect = msgEl.getBoundingClientRect();
      const containerRect = container.getBoundingClientRect();
      const overflow = rect.bottom + 60 - containerRect.bottom;
      if (overflow > 0) {
        container.scrollTop += overflow;
      }
    }
  });
}

function submitEdit() {
  const newContent = trimMessage(editContent.value);
  if (!editingMessageId.value || !newContent) return;
  const id = editingMessageId.value;
  const msgs = state.value?.messages.get(state.value?.activeChannelId ?? 0);
  if (msgs) {
    const msg = msgs.find((m) => m.id === id);
    if (msg) msg.content = newContent;
  }
  editMessage(id, newContent);
  editingMessageId.value = null;
  editContent.value = "";
}

function cancelEdit() {
  editingMessageId.value = null;
  editContent.value = "";
}

// ── Drag & drop (delegated to ChatInput) ──
function onDragOver(e: DragEvent) {
  e.preventDefault();
  dragging.value = true;
}

function onDragLeave() {
  dragging.value = false;
}

function onDrop(e: DragEvent) {
  e.preventDefault();
  dragging.value = false;
  if (e.dataTransfer?.files.length) {
    chatInputRef.value?.addFiles(e.dataTransfer.files);
  }
}

// ── Reactions ──
const quickEmojis = ref(topEmojis());
const emojiPicker = ref<{ x: number; y: number; messageId: number } | null>(null);

function openEmojiPicker(messageId: number, e: MouseEvent | Event) {
  const me = e as MouseEvent;
  const x = Math.min(me.clientX, window.innerWidth - 300);
  const y = Math.max(me.clientY - 280, 8);
  emojiPicker.value = { x, y, messageId };
}

function onEmojiSelect(emoji: string) {
  if (emojiPicker.value) {
    recordEmoji(emoji);
    toggleReaction(emojiPicker.value.messageId, emoji);
    emojiPicker.value = null;
    quickEmojis.value = topEmojis();
  }
}

function onToggleReaction(messageId: number, emoji: string) {
  recordEmoji(emoji);
  toggleReaction(messageId, emoji);
  quickEmojis.value = topEmojis();
}

function onRetry(nonce: string) {
  retryMessage(nonce);
}

function onDiscard(nonce: string) {
  discardFailedMessage(nonce);
}

// ── Message actions ──
const ctxMenu = ref<{ x: number; y: number; items: MenuItem[] } | null>(null);
const confirmDeleteId = ref<number | null>(null);

const canManage = computed(() =>
  perms.has(state.value?.permissions ?? 0, perms.MANAGE_MESSAGES)
);

function isOwnMessage(msg: Message): boolean {
  if (isWebhookMessage(msg)) return false;
  return msg.author_id === state.value?.user?.id;
}

function canActOnMessage(msg: Message): boolean {
  return isOwnMessage(msg) || canManage.value;
}

function handleDelete(msg: Message, e: MouseEvent) {
  if (e.shiftKey) {
    deleteMessage(msg.id);
  } else {
    confirmDeleteId.value = msg.id;
  }
}

function confirmDelete() {
  if (confirmDeleteId.value) {
    deleteMessage(confirmDeleteId.value);
    confirmDeleteId.value = null;
  }
}

async function togglePin(msg: Message) {
  const server = activeServer();
  if (!server) return;
  try {
    await api.togglePin(server.url, server.token, msg.channel_id, msg.id);
  } catch {}
}

function buildActions(msg: Message): MessageAction[] {
  if (msg.pending || msg.failed) return [];
  const items: MessageAction[] = [];
  for (const emoji of quickEmojis.value) {
    items.push({
      key: `quick-${emoji}`,
      label: emoji,
      emoji,
      handler: () => onToggleReaction(msg.id, emoji),
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
  if (canManage.value) {
    items.push({
      key: "pin",
      label: msg.pinned ? "Desepingler" : "Epingler",
      icon: msg.pinned ? PinOff : Pin,
      handler: () => togglePin(msg),
    });
  }
  if (isOwnMessage(msg)) {
    items.push({
      key: "edit",
      label: "Modifier",
      icon: Pencil,
      handler: () => startEdit(msg),
    });
  }
  if (canActOnMessage(msg)) {
    items.push({
      key: "delete",
      label: "Supprimer",
      icon: Trash2,
      danger: true,
      handler: (e) => handleDelete(msg, e),
    });
  }
  return items;
}

function onMessageContextMenu(msg: Message, e: MouseEvent) {
  if (msg.pending || msg.failed) return;
  const items: MenuItem[] = [];

  items.push({ label: "Repondre", icon: Reply, action: () => startReply(msg) });

  if (isOwnMessage(msg)) {
    items.push({ label: "Modifier", icon: Pencil, action: () => startEdit(msg) });
  }

  if (canManage.value) {
    items.push({
      label: msg.pinned ? "Desepingler" : "Epingler",
      icon: msg.pinned ? PinOff : Pin,
      action: () => togglePin(msg),
    });
  }

  if (canActOnMessage(msg)) {
    items.push({
      label: "Supprimer",
      icon: Trash2,
      danger: true,
      action: () => {
        if (e.shiftKey) {
          deleteMessage(msg.id);
        } else {
          confirmDeleteId.value = msg.id;
        }
      },
    });
  }

  if (items.length) {
    ctxMenu.value = { x: e.clientX, y: e.clientY, items };
  }
}

function formatDate(ts: string): string {
  try {
    const date = new Date(ts + "Z");
    const now = new Date();
    const yesterday = new Date(now);
    yesterday.setDate(yesterday.getDate() - 1);

    if (date.toDateString() === now.toDateString()) return "Aujourd'hui";
    if (date.toDateString() === yesterday.toDateString()) return "Hier";
    return date.toLocaleDateString("fr-FR", { day: "numeric", month: "long", year: "numeric" });
  } catch {
    return ts;
  }
}

const cardUser = ref<User | null>(null);
const cardX = ref(0);
const cardY = ref(0);

function openCard(userId: number, e: MouseEvent) {
  const user = state.value?.users.get(userId);
  if (!user) return;

  const el = e.currentTarget as HTMLElement;
  const rect = el.getBoundingClientRect();
  cardX.value = rect.right + 8;
  cardY.value = rect.top;
  cardUser.value = user;
}
</script>

<style scoped>
.chat-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background: var(--bg-primary);
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 16px 16px;
}

.loading-older {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 12px;
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

.chat-empty {
  color: var(--text-muted);
  text-align: center;
  margin-top: 3rem;
  font-size: 0.9375rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.chat-empty-sub {
  font-size: 0.8125rem;
  color: var(--text-faint);
}

.date-separator {
  display: flex;
  align-items: center;
  margin: 8px 16px;
  font-size: 0.6875rem;
  font-weight: 700;
  color: var(--text-muted);
}

.date-separator::before,
.date-separator::after {
  content: '';
  flex: 1;
  height: 1px;
  background: var(--border);
}

.date-separator span {
  padding: 0 8px;
}

.modal-overlay {
  position: fixed;
  inset: 0;
  background: var(--overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
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

/* ── Drop overlay ── */
.drop-overlay {
  position: absolute;
  inset: 0;
  z-index: 50;
  background: var(--overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
}

.drop-overlay-inner {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  color: var(--header-primary);
  font-size: 1rem;
  font-weight: 600;
}

/* ── Message highlight on scroll ── */
.message-highlight {
  background: var(--accent);
  background: rgba(88, 101, 242, 0.1);
  transition: background 0.3s;
}

/* ── Skeleton ── */
.chat-skeleton {
  flex: 1;
  padding: 16px;
  overflow: hidden;
}

.skeleton-message {
  display: flex;
  gap: 16px;
  padding: 2px 0;
}

.skeleton-message:not(.grouped) {
  margin-top: 16px;
}

.skeleton-avatar {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  flex-shrink: 0;
}

.skeleton-gutter {
  width: 40px;
  flex-shrink: 0;
}

.skeleton-msg-body {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.skeleton-msg-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.skeleton {
  background: var(--bg-modifier-hover);
  border-radius: 4px;
  animation: skeleton-pulse 1.5s ease-in-out infinite;
}

.skeleton-text {
  height: 14px;
}

@keyframes skeleton-pulse {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 0.8; }
}

/* ── Typing indicator ── */
.typing-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 16px;
  font-size: 0.75rem;
  color: var(--text-muted);
  height: 20px;
}

.typing-dots {
  display: inline-flex;
  gap: 2px;
  align-items: center;
}

.typing-dots span {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: var(--text-muted);
  animation: typing-bounce 1.4s infinite ease-in-out;
}

.typing-dots span:nth-child(2) { animation-delay: 0.2s; }
.typing-dots span:nth-child(3) { animation-delay: 0.4s; }

@keyframes typing-bounce {
  0%, 60%, 100% { transform: translateY(0); opacity: 0.4; }
  30% { transform: translateY(-3px); opacity: 1; }
}
</style>
