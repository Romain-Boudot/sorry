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
        <div class="message" :class="{ grouped: isGrouped(i) }">
          <template v-if="!isGrouped(i)">
            <div class="message-avatar">
              {{ resolveUser(msg.author_id)[0]?.toUpperCase() }}
            </div>
            <div class="message-body">
              <div class="message-header">
                <span class="message-author">{{ resolveUser(msg.author_id) }}</span>
                <span class="message-time">{{ formatTime(msg.created_at) }}</span>
              </div>
              <div class="message-content">{{ msg.content }}</div>
            </div>
          </template>
          <template v-else>
            <div class="message-gutter">
              <span class="message-time-hover">{{ formatTimeShort(msg.created_at) }}</span>
            </div>
            <div class="message-body">
              <div class="message-content">{{ msg.content }}</div>
            </div>
          </template>
        </div>
      </template>
    </div>

    <div class="chat-input">
      <div class="chat-input-wrapper">
        <input
          v-model="input"
          @keydown.enter="handleSend"
          :placeholder="`Envoyer un message dans #${activeChannel?.name ?? '...'}`"
          type="text"
        />
        <button class="chat-send" @click="handleSend" :disabled="!input.trim()">
          <SendHorizonal :size="18" />
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, nextTick } from "vue";
import { MessageSquare, SendHorizonal } from "lucide-vue-next";
import { activeState, sendMessage, resolveUser } from "../store";
import type { Message } from "../api";

const input = ref("");
const messagesContainer = ref<HTMLElement>();

const state = computed(() => activeState());

const activeChannel = computed(() =>
  state.value?.channels.find((c) => c.id === state.value?.activeChannelId)
);

const messages = computed(() =>
  state.value?.messages.get(state.value?.activeChannelId ?? 0) ?? []
);

watch(
  () => messages.value.length,
  async () => {
    await nextTick();
    const el = messagesContainer.value;
    if (el) el.scrollTop = el.scrollHeight;
  }
);

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

function handleSend() {
  if (!input.value.trim()) return;
  sendMessage(input.value);
  input.value = "";
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

    if (date.toDateString() === now.toDateString()) return `Aujourd'hui ${time}`;
    if (date.toDateString() === yesterday.toDateString()) return `Hier ${time}`;
    return `${date.toLocaleDateString("fr-FR", { day: "2-digit", month: "2-digit", year: "numeric" })} ${time}`;
  } catch {
    return ts;
  }
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
  align-items: center;
  justify-content: flex-end;
}

.message-time-hover {
  font-size: 0.625rem;
  color: var(--text-muted);
  opacity: 0;
  transition: opacity 0.1s;
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

.chat-input {
  padding: 0 8px 8px;
}

.chat-input-wrapper {
  display: flex;
  align-items: center;
  height: var(--bar-height);
  background: var(--bg-floating);
  border-radius: 8px;
  border: 1px solid var(--border);
  padding-right: 4px;
}

.chat-input input {
  width: 100%;
  padding: 11px 16px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: var(--text-normal);
  font-size: 0.9375rem;
  font-family: inherit;
  outline: none;
}

.chat-input input::placeholder {
  color: var(--text-faint);
}

.chat-send {
  width: 32px;
  height: 32px;
  padding: 0;
  margin: 0;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  background: transparent;
  color: var(--text-muted);
  transition: color 0.1s;
}

.chat-send:hover { color: var(--text-normal); box-shadow: none; }
.chat-send:disabled { color: var(--text-faint); opacity: 0.5; }
</style>
