<template>
  <div class="chat-body">
    <div class="chat-messages" ref="messagesContainer">
      <div v-if="!messages.length" class="chat-empty">
        <MessageSquare :size="40" :stroke-width="1.2" />
        <p>Aucun message dans #{{ activeChannel?.name }}</p>
        <p class="chat-empty-sub">Sois le premier !</p>
      </div>
      <div v-for="msg in messages" :key="msg.id" class="message">
        <div class="message-header">
          <span class="message-author">{{ resolveUser(msg.author_id) }}</span>
          <span class="message-time">{{ formatTime(msg.created_at) }}</span>
        </div>
        <div class="message-content">{{ msg.content }}</div>
      </div>
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

function handleSend() {
  if (!input.value.trim()) return;
  sendMessage(input.value);
  input.value = "";
}

function formatTime(ts: string): string {
  try {
    return new Date(ts + "Z").toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return ts;
  }
}
</script>
