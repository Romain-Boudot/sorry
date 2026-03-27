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
