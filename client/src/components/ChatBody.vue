<template>
  <div class="chat-body">
    <div class="chat-messages" ref="messagesContainer">
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
          @contextmenu.prevent="onMessageContextMenu(msg, $event)"
        >
          <!-- Hover actions -->
          <div v-if="canActOnMessage(msg) && editingMessageId !== msg.id" class="message-actions">
            <button v-if="isOwnMessage(msg)" class="msg-action-btn" title="Modifier" @click="startEdit(msg)">
              <Pencil :size="14" />
            </button>
            <button class="msg-action-btn danger" title="Supprimer" @click="handleDelete(msg, $event)">
              <Trash2 :size="14" />
            </button>
          </div>

          <template v-if="!isGrouped(i)">
            <div class="message-avatar" @click="openCard(msg.author_id, $event)">
              {{ resolveUser(msg.author_id)[0]?.toUpperCase() }}
            </div>
            <div class="message-body">
              <div class="message-header">
                <span class="message-author" @click="openCard(msg.author_id, $event)">{{ resolveUser(msg.author_id) }}</span>
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
                <div class="message-edit-hint">Echap pour annuler · Entrée pour enregistrer</div>
              </template>
              <div v-else class="message-content">{{ msg.content }}</div>
            </div>
          </template>
          <template v-else>
            <div class="message-gutter">
              <span class="message-time-hover">{{ formatTimeShort(msg.created_at) }}</span>
            </div>
            <div class="message-body">
              <template v-if="editingMessageId === msg.id">
                <textarea
                  class="message-edit-input"
                  v-model="editContent"
                  @keydown="onEditKeydown"
                  @input="autoResize"
                  v-focus
                  rows="1"
                ></textarea>
                <div class="message-edit-hint">Echap pour annuler · Entrée pour enregistrer</div>
              </template>
              <div v-else class="message-content">{{ msg.content }}</div>
            </div>
          </template>
        </div>
      </template>
    </div>

    <div class="chat-input">
      <div class="chat-input-wrapper">
        <textarea
          ref="mainInput"
          v-model="input"
          @keydown="onMainKeydown"
          @input="autoResize"
          :placeholder="`Envoyer un message dans #${activeChannel?.name ?? '...'}`"
          rows="1"
        ></textarea>
        <button class="chat-send" @click="handleSend" :disabled="!input.trim()">
          <SendHorizonal :size="18" />
        </button>
      </div>
    </div>

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
import { MessageSquare, SendHorizonal, Pencil, Trash2 } from "lucide-vue-next";
import { activeState, sendMessage, editMessage, deleteMessage, resolveUser } from "../store";
import * as perms from "../permissions";
import type { Message, User } from "../api";
import ContextMenu, { type MenuItem } from "./ContextMenu.vue";
import UserCard from "./UserCard.vue";

const input = ref("");
const messagesContainer = ref<HTMLElement>();
const mainInput = ref<HTMLTextAreaElement>();
const editingMessageId = ref<number | null>(null);
const editContent = ref("");
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

function scrollToBottom() {
  nextTick(() => {
    const el = messagesContainer.value;
    if (el) el.scrollTop = el.scrollHeight;
  });
}

watch(() => messages.value.length, scrollToBottom);
watch(() => state.value?.activeChannelId, scrollToBottom);
onMounted(scrollToBottom);

// Group messages from the same author within 5 minutes
function isGrouped(index: number): boolean {
  if (index === 0) return false;
  const msg = messages.value[index];
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

function handleSend() {
  const content = trimMessage(input.value);
  if (!content) return;
  sendMessage(content);
  input.value = "";
  nextTick(() => {
    if (mainInput.value) {
      mainInput.value.style.height = "auto";
    }
  });
}

function onMainKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    handleSend();
    return;
  }
  if (e.key === "ArrowUp" && !input.value) {
    const st = state.value;
    if (!st?.activeChannelId || !st.user) return;
    const msgs = st.messages.get(st.activeChannelId);
    if (!msgs) return;
    for (let i = msgs.length - 1; i >= 0; i--) {
      if (msgs[i].author_id === st.user.id) {
        startEdit(msgs[i]);
        e.preventDefault();
        return;
      }
    }
  }
}

function onEditKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    submitEdit();
  } else if (e.key === "Escape") {
    cancelEdit();
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
      // Scroll so the whole message + some margin is visible
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
  if (!canActOnMessage(msg)) return;

  const items: MenuItem[] = [];

  if (isOwnMessage(msg)) {
    items.push({ label: "Modifier", action: () => startEdit(msg) });
  }

  items.push({
    label: "Supprimer",
    danger: true,
    action: () => {
      if (e.shiftKey) {
        deleteMessage(msg.id);
      } else {
        confirmDeleteId.value = msg.id;
      }
    },
  });

  ctxMenu.value = { x: e.clientX, y: e.clientY, items };
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
  color: #fff;
  flex-shrink: 0;
  margin-top: 2px;
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
  white-space: pre-wrap;
}

.chat-input {
  padding: 0 8px 8px;
}

.chat-input-wrapper {
  display: flex;
  align-items: flex-end;
  /* Match the user card height: --bar-height is set on .user-row (child),
     but the border (1px × 2) is on .bottom-card (parent) → total = 48 + 2 = 50px.
     Here the border is on this element itself, so border-box gives us 50px total. */
  min-height: calc(var(--bar-height) + 2px);
  background: var(--bg-floating);
  border-radius: 8px;
  border: 1px solid var(--border);
  padding-right: 4px;
}

.chat-input textarea {
  width: 100%;
  min-height: var(--bar-height);
  padding: 13px 16px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: var(--text-normal);
  font-size: 0.9375rem;
  font-family: inherit;
  outline: none;
  resize: none;
  overflow: hidden;
  overflow-wrap: break-word;
  line-height: 1.375;
  max-height: 200px;
}

.chat-input textarea::placeholder {
  color: var(--text-faint);
}

.chat-send {
  width: 32px;
  height: var(--bar-height);
  padding: 0;
  margin: 0;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  background: transparent;
  color: var(--text-muted);
  transition: color 0.1s;
}

.chat-send:hover { color: var(--text-normal); box-shadow: none; }
.chat-send:disabled { color: var(--text-faint); opacity: 0.5; }

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
  color: #fff;
}

.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
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
  color: #fff;
}

.btn-danger:hover {
  opacity: 0.9;
}
</style>
