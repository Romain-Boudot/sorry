<template>
  <div class="pins-panel">
    <div class="pins-header">
      <span class="pins-title">Messages epingles</span>
      <button class="pins-close" @click="$emit('close')"><X :size="16" /></button>
    </div>

    <div class="pins-body">
      <div v-if="loading" class="pins-status">
        <Loader2 :size="16" class="spinner" /> Chargement...
      </div>
      <div v-else-if="!pins.length" class="pins-status">
        Aucun message epingle
      </div>
      <div v-else class="pins-list">
        <div v-for="msg in pins" :key="msg.id" class="pin-item" @click="$emit('jump-to', msg.id)">
          <div class="pin-header">
            <span class="pin-author">{{ resolveUser(msg.author_id) }}</span>
            <span class="pin-time">{{ formatDate(msg.created_at) }}</span>
          </div>
          <div class="pin-content">{{ truncate(msg.content, 200) }}</div>
          <div v-if="msg.attachments.length" class="pin-attachments">
            <Paperclip :size="12" />
            {{ msg.attachments.length }} fichier{{ msg.attachments.length > 1 ? 's' : '' }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { X, Loader2, Paperclip } from "lucide-vue-next";
import { activeState, activeServer, resolveUser } from "../../store";
import { api, type Message } from "../../api";

defineEmits<{
  close: [];
  "jump-to": [messageId: number];
}>();

const state = computed(() => activeState());
const pins = ref<Message[]>([]);
const loading = ref(false);

async function loadPins() {
  const server = activeServer();
  const channelId = state.value?.activeChannelId;
  if (!server || !channelId) return;

  loading.value = true;
  try {
    pins.value = await api.listPinned(server.url, server.token, channelId);
  } catch {
    pins.value = [];
  } finally {
    loading.value = false;
  }
}

// Reload when channel changes
watch(() => state.value?.activeChannelId, () => loadPins(), { immediate: true });

function truncate(s: string, max: number): string {
  return s.length > max ? s.slice(0, max) + "..." : s;
}

function formatDate(ts: string): string {
  try {
    const date = new Date(ts + "Z");
    return date.toLocaleDateString("fr-FR", { day: "numeric", month: "short" })
      + " " + date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  } catch {
    return ts;
  }
}
</script>

<style scoped>
.pins-panel {
  width: 340px;
  flex-shrink: 0;
  background: var(--bg-secondary);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  height: 100%;
}

.pins-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.pins-title {
  font-weight: 700;
  font-size: 0.875rem;
  color: var(--header-primary);
}

.pins-close {
  width: 28px;
  height: 28px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
}

.pins-close:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
}

.pins-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.pins-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 24px 16px;
  font-size: 0.8125rem;
  color: var(--text-muted);
  justify-content: center;
}

.spinner {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.pins-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.pin-item {
  padding: 10px 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.1s;
}

.pin-item:hover {
  background: var(--bg-modifier-hover);
}

.pin-header {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-bottom: 4px;
}

.pin-author {
  font-weight: 600;
  font-size: 0.8125rem;
  color: var(--header-primary);
}

.pin-time {
  font-size: 0.6875rem;
  color: var(--text-muted);
}

.pin-content {
  font-size: 0.8125rem;
  color: var(--text-normal);
  line-height: 1.4;
  word-break: break-word;
}

.pin-attachments {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 0.75rem;
  color: var(--text-faint);
  margin-top: 4px;
}
</style>
