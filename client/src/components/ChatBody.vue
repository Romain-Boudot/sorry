<template>
  <div class="chat-body" @dragover="onDragOver" @dragleave="onDragLeave" @drop="onDrop">
    <div v-if="dragging" class="drop-overlay">
      <div class="drop-overlay-inner">
        <Paperclip :size="40" :stroke-width="1.2" />
        <p>Depose tes fichiers ici</p>
      </div>
    </div>
    <div class="chat-messages" ref="messagesContainer" @scroll="onMessagesScroll">
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
        <!-- Date separator -->
        <div v-if="showDateSeparator(i)" class="date-separator">
          <span>{{ formatDate(msg.created_at) }}</span>
        </div>
        <!-- Message -->
        <div
          class="message"
          :class="{ grouped: isGrouped(i), editing: editingMessageId === msg.id }"
          :data-msg-id="msg.id"
          @contextmenu.prevent="onMessageContextMenu(msg, $event)"
        >
          <!-- Hover actions -->
          <div v-if="editingMessageId !== msg.id" class="message-actions">
            <button class="msg-action-btn" title="Repondre" @click="startReply(msg)">
              <Reply :size="14" />
            </button>
            <button v-if="isOwnMessage(msg)" class="msg-action-btn" title="Modifier" @click="startEdit(msg)">
              <Pencil :size="14" />
            </button>
            <button v-if="canActOnMessage(msg)" class="msg-action-btn danger" title="Supprimer" @click="handleDelete(msg, $event)">
              <Trash2 :size="14" />
            </button>
          </div>

          <template v-if="!isGrouped(i)">
            <div class="message-avatar" :class="{ 'has-reply': msg.reply_to }" @click="openCard(msg.author_id, $event)">
              <img v-if="resolveAvatarUrl(msg.author_id)" :src="resolveAvatarUrl(msg.author_id)!" />
              <span v-else>{{ resolveUser(msg.author_id)[0]?.toUpperCase() }}</span>
            </div>
            <div class="message-body">
              <ReplyPreview
                v-if="msg.reply_to"
                :avatar-url="resolveAvatarUrl(msg.reply_to.author_id)"
                :author-name="resolveUser(msg.reply_to.author_id)"
                :author-color="resolveUserColor(msg.reply_to.author_id)"
                :content="msg.reply_to.content"
                @click="scrollToMessage(msg.reply_to.id)"
              />
              <div class="message-header">
                <span class="message-author" :style="resolveUserColor(msg.author_id) ? `color:${resolveUserColor(msg.author_id)}` : ''" @click="openCard(msg.author_id, $event)">{{ resolveUser(msg.author_id) }}</span>
                <span v-if="isGuest(msg.author_id)" class="guest-tag">Guest</span>
                <span class="message-time">{{ formatTime(msg.created_at) }}</span>
              </div>
              <template v-if="editingMessageId === msg.id">
                <textarea
                  class="message-edit-input"
                  v-model="editContent"
                  @keydown="onEditKeydown"
                  @input="autoResize"
                  v-focus
                  rows="1"
                ></textarea>
                <div class="message-edit-hint">Echap pour annuler · Entree pour enregistrer</div>
              </template>
              <div v-else-if="msg.content" class="message-content" v-html="renderMarkdown(msg.content)"></div>
              <LinkPreview v-for="url in extractUrls(msg.content)" :key="url" :url="url" />
              <AttachmentList :attachments="msg.attachments" />
            </div>
          </template>
          <template v-else>
            <div class="message-gutter">
              <span class="message-time-hover">{{ formatTimeShort(msg.created_at) }}</span>
            </div>
            <div class="message-body">
              <ReplyPreview
                v-if="msg.reply_to"
                :avatar-url="resolveAvatarUrl(msg.reply_to.author_id)"
                :author-name="resolveUser(msg.reply_to.author_id)"
                :author-color="resolveUserColor(msg.reply_to.author_id)"
                :content="msg.reply_to.content"
                @click="scrollToMessage(msg.reply_to.id)"
              />
              <template v-if="editingMessageId === msg.id">
                <textarea
                  class="message-edit-input"
                  v-model="editContent"
                  @keydown="onEditKeydown"
                  @input="autoResize"
                  v-focus
                  rows="1"
                ></textarea>
                <div class="message-edit-hint">Echap pour annuler · Entree pour enregistrer</div>
              </template>
              <div v-else-if="msg.content" class="message-content" v-html="renderMarkdown(msg.content)"></div>
              <LinkPreview v-for="url in extractUrls(msg.content)" :key="url" :url="url" />
              <AttachmentList :attachments="msg.attachments" />
            </div>
          </template>
        </div>
      </template>
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

    <UserCard
      v-if="cardUser"
      :user="cardUser"
      :x="cardX"
      :y="cardY"
      @close="cardUser = null"
    />

    <!-- Delete confirmation -->
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
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, nextTick, onMounted } from "vue";
import { MessageSquare, Pencil, Trash2, Paperclip, Loader2, Reply } from "lucide-vue-next";
import { activeState, activeServer, editMessage, deleteMessage, resolveUser, resolveUserColor, resolveAvatarUrl, isGuest } from "../store";
import * as perms from "../permissions";
import { api, type Message, type User } from "../api";
import { renderMarkdown, extractUrls } from "../markdown";
import LinkPreview from "./LinkPreview.vue";
import ContextMenu, { type MenuItem } from "./ContextMenu.vue";
import UserCard from "./UserCard.vue";
import ReplyPreview from "./chat/ReplyPreview.vue";
import AttachmentList from "./chat/AttachmentList.vue";
import ChatInput from "./chat/ChatInput.vue";

const messagesContainer = ref<HTMLElement>();
const chatInputRef = ref<InstanceType<typeof ChatInput>>();
const editingMessageId = ref<number | null>(null);
const editContent = ref("");
const replyingTo = ref<Message | null>(null);
const dragging = ref(false);

const vFocus = {
  mounted: (el: HTMLElement) => {
    el.focus();
    autoResize({ target: el } as unknown as Event);
  },
};

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
  nextTick(() => {
    const el = messagesContainer.value;
    if (el) el.scrollTop = el.scrollHeight;
  });
}

watch(() => messages.value.length, (newLen, oldLen) => {
  if (!oldLen || newLen <= oldLen) return;
  if (isScrolledToBottom()) scrollToBottom();
});

watch(() => state.value?.activeChannelId, () => {
  noMoreMessages.value = false;
  scrollToBottom();
});

onMounted(scrollToBottom);

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
  if (el.scrollTop < 100) {
    loadOlderMessages();
  }
}

function isGrouped(index: number): boolean {
  if (index === 0) return false;
  const msg = messages.value[index];
  if (msg.reply_to) return false;
  const prev = messages.value[index - 1];
  if (msg.author_id !== prev.author_id) return false;
  const diff = new Date(msg.created_at + "Z").getTime() - new Date(prev.created_at + "Z").getTime();
  return diff < 5 * 60 * 1000;
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

function autoResize(e: Event) {
  const wasAtBottom = isScrolledToBottom();
  const el = e.target as HTMLTextAreaElement;
  el.style.height = "auto";
  el.style.height = el.scrollHeight + "px";
  el.style.overflowY = el.scrollHeight > el.offsetHeight ? "auto" : "hidden";
  if (wasAtBottom) {
    nextTick(() => {
      const container = messagesContainer.value;
      if (container) container.scrollTop = container.scrollHeight;
    });
  }
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

function onEditKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    submitEdit();
  } else if (e.key === "Escape") {
    cancelEdit();
  }
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

// ── Message actions ──
const ctxMenu = ref<{ x: number; y: number; items: MenuItem[] } | null>(null);
const confirmDeleteId = ref<number | null>(null);

const canManage = computed(() =>
  perms.has(state.value?.permissions ?? 0, perms.MANAGE_MESSAGES)
);

function isOwnMessage(msg: Message): boolean {
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

function onMessageContextMenu(msg: Message, e: MouseEvent) {
  const items: MenuItem[] = [];

  items.push({ label: "Repondre", icon: Reply, action: () => startReply(msg) });

  if (isOwnMessage(msg)) {
    items.push({ label: "Modifier", icon: Pencil, action: () => startEdit(msg) });
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

function formatTime(ts: string): string {
  try {
    const date = new Date(ts + "Z");
    const now = new Date();
    const yesterday = new Date(now);
    yesterday.setDate(yesterday.getDate() - 1);

    const time = date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });

    if (date.toDateString() === now.toDateString()) return time;
    if (date.toDateString() === yesterday.toDateString()) return `Hier ${time}`;
    return `${date.toLocaleDateString("fr-FR", { day: "2-digit", month: "2-digit", year: "numeric" })} ${time}`;
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

function formatTimeShort(ts: string): string {
  try {
    return new Date(ts + "Z").toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  } catch {
    return ts;
  }
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

.message {
  display: flex;
  padding: 2px 16px;
  margin: 0 -16px;
  gap: 16px;
  position: relative;
}

.message:not(.grouped) {
  margin-top: 16px;
}

.message:hover {
  background: var(--bg-modifier-hover);
}

.message-avatar {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 0.875rem;
  color: var(--text-bright);
  flex-shrink: 0;
  margin-top: 2px;
  overflow: hidden;
  cursor: pointer;
}

.message-avatar.has-reply {
  margin-top: 26px;
}

.message-avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.message-gutter {
  width: 40px;
  flex-shrink: 0;
  display: flex;
  align-items: flex-start;
  justify-content: flex-end;
  padding-top: 4px;
}

.message-time-hover {
  font-size: 0.625rem;
  color: var(--text-muted);
  opacity: 0;
}

.message:hover .message-time-hover {
  opacity: 1;
}

.message-body {
  min-width: 0;
  flex: 1;
}

.message-header {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.message-author {
  font-weight: 600;
  font-size: 0.9375rem;
  color: var(--header-primary);
  cursor: pointer;
}

.message-author:hover {
  text-decoration: underline;
}

.message-time {
  font-size: 0.6875rem;
  color: var(--text-muted);
  font-weight: 400;
}

.message-content {
  color: var(--text-normal);
  line-height: 1.375rem;
  word-break: break-word;
  font-size: 0.9375rem;
}

.message-content :deep(p) { margin: 0; }
.message-content :deep(p + p) { margin-top: 4px; }
.message-content :deep(a) { color: var(--accent); text-decoration: none; }
.message-content :deep(a:hover) { text-decoration: underline; }
.message-content :deep(strong) { font-weight: 700; color: var(--header-primary); }
.message-content :deep(em) { font-style: italic; }
.message-content :deep(del) { text-decoration: line-through; color: var(--text-muted); }

.message-content :deep(code) {
  background: var(--bg-tertiary);
  padding: 1px 5px;
  border-radius: 4px;
  font-size: 0.85em;
  font-family: monospace;
}

.message-content :deep(pre) {
  background: var(--bg-tertiary);
  padding: 10px 12px;
  border-radius: 6px;
  overflow-x: auto;
  max-height: 300px;
  overflow-y: auto;
  margin: 4px 0;
}

.message-content :deep(pre code) {
  background: none;
  padding: 0;
  border-radius: 0;
  font-size: 0.85em;
}

.message-content :deep(h3),
.message-content :deep(h4),
.message-content :deep(h5),
.message-content :deep(h6) {
  font-weight: 700;
  color: var(--header-primary);
  margin: 8px 0 4px;
}

.message-content :deep(h3) { font-size: 1.1em; }
.message-content :deep(h4) { font-size: 1em; }

.message-content :deep(hr) {
  border: none;
  border-top: 1px solid var(--border);
  margin: 8px 0;
}

.message-content :deep(table) {
  border-collapse: collapse;
  max-width: 100%;
  overflow-x: auto;
  display: block;
  margin: 4px 0;
  font-size: 0.875em;
}

.message-content :deep(th),
.message-content :deep(td) {
  border: 1px solid var(--border);
  padding: 4px 8px;
}

.message-content :deep(th) {
  background: var(--bg-tertiary);
  font-weight: 600;
}

.message-content :deep(blockquote) {
  border-left: 3px solid var(--accent);
  margin: 4px 0;
  padding: 2px 12px;
  color: var(--text-muted);
}

.message-content :deep(ul),
.message-content :deep(ol) {
  margin: 4px 0;
  padding-left: 24px;
}

.message.editing {
  background: var(--bg-modifier-active);
}

.message-edit-input {
  width: 100%;
  padding: 8px 10px;
  border-radius: 6px;
  border: 1px solid var(--accent);
  background: var(--bg-tertiary);
  color: var(--text-normal);
  font-size: 0.9375rem;
  font-family: inherit;
  outline: none;
  margin-top: 2px;
  resize: none;
  overflow: hidden;
  line-height: 1.375;
  max-height: 200px;
}

.message-edit-hint {
  font-size: 0.6875rem;
  color: var(--text-muted);
  margin-top: 4px;
}

.message-actions {
  position: absolute;
  top: -26px;
  right: 16px;
  display: none;
  gap: 2px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 2px;
  z-index: 1;
}

.message:hover .message-actions {
  display: flex;
}

.msg-action-btn {
  width: 28px;
  height: 28px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  background: transparent;
  color: var(--text-muted);
  border: none;
  cursor: pointer;
  transition: background 0.1s, color 0.1s;
}

.msg-action-btn:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
  box-shadow: none;
}

.msg-action-btn.danger:hover {
  background: var(--danger);
  color: var(--text-bright);
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
</style>
