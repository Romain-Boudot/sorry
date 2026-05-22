<template>
  <div class="chat-header">
    <template v-if="activeChannel">
      <template v-if="!searching">
        <Volume2 v-if="isVoice" class="channel-icon" :size="18" />
        <Hash v-else class="channel-icon" :size="18" />
        <span>{{ activeChannel.name }}</span>
        <template v-if="activeChannel.description">
          <div class="topic-separator"></div>
          <span class="topic">{{ activeChannel.description }}</span>
        </template>
      </template>
      <div v-else class="search-bar">
        <Search :size="14" class="search-icon" />
        <input
          ref="searchInput"
          v-model="query"
          type="text"
          :placeholder="`Rechercher par debut de mot dans #${activeChannel.name}`"
          class="search-input"
          @keydown.escape="closeSearch"
        />
        <button class="search-close" @click="closeSearch"><X :size="14" /></button>
      </div>
      <div class="header-spacer"></div>
      <button v-if="!searching && !isVoice" class="header-btn" :class="{ active: pinsOpen }" title="Messages epingles" @click="$emit('toggle-pins')">
        <Pin :size="16" />
      </button>
      <button v-if="!searching && !isVoice" class="header-btn" :class="{ active: galleryOpen }" title="Fichiers" @click="$emit('toggle-gallery')">
        <ImageIcon :size="16" />
      </button>
      <button v-if="!searching" class="header-btn" title="Rechercher" @click="openSearch">
        <Search :size="16" />
      </button>
    </template>
    <template v-else>
      <div class="skeleton skeleton-icon"></div>
      <div class="skeleton skeleton-text"></div>
    </template>

    <!-- Search results dropdown -->
    <div v-if="searching && (results.length || searchLoading || query.length >= 2)" class="search-results" ref="resultsEl">
      <div v-if="searchLoading" class="search-status">
        <Loader2 :size="14" class="spinner" /> Recherche...
      </div>
      <div v-else-if="query.length >= 2 && !results.length" class="search-status">
        Aucun resultat
      </div>
      <button
        v-for="msg in results"
        :key="msg.id"
        class="search-result"
        @click="$emit('jump-to', msg.id); closeSearch()"
      >
        <div class="search-result-header">
          <span class="search-result-author">{{ resolveUser(msg.author_id) }}</span>
          <span class="search-result-time">{{ formatResultDate(msg.created_at) }}</span>
        </div>
        <div class="search-result-content" v-html="highlightQuery(msg.content)"></div>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch, nextTick } from "vue";
import { Hash, Volume2, Search, X, Loader2, ImageIcon, Pin } from "lucide-vue-next";
import { activeState, activeServer, isActiveChannelVoice, resolveUser } from "../store";
import { api, type Message } from "../api";

defineProps<{
  galleryOpen?: boolean;
  pinsOpen?: boolean;
}>();

defineEmits<{
  "jump-to": [messageId: number];
  "toggle-gallery": [];
  "toggle-pins": [];
}>();

const state = computed(() => activeState());
const isVoice = computed(() => isActiveChannelVoice());
const activeChannel = computed(() =>
  state.value?.channels.find((c) => c.id === state.value?.activeChannelId)
);

const searching = ref(false);
const query = ref("");
const results = ref<Message[]>([]);
const searchLoading = ref(false);
const searchInput = ref<HTMLInputElement>();

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

function openSearch() {
  searching.value = true;
  nextTick(() => searchInput.value?.focus());
}

function closeSearch() {
  searching.value = false;
  query.value = "";
  results.value = [];
}

watch(query, (q) => {
  if (debounceTimer) clearTimeout(debounceTimer);
  const trimmed = q.trim();
  if (trimmed.length < 2) {
    results.value = [];
    searchLoading.value = false;
    return;
  }
  searchLoading.value = true;
  debounceTimer = setTimeout(() => doSearch(trimmed), 300);
});

// Reset search when switching channels
watch(() => state.value?.activeChannelId, () => {
  if (searching.value) closeSearch();
});

async function doSearch(q: string) {
  const server = activeServer();
  const channelId = state.value?.activeChannelId;
  if (!server || !channelId) return;
  try {
    results.value = await api.searchMessages(server.url, server.token, channelId, q);
  } catch {
    results.value = [];
  } finally {
    searchLoading.value = false;
  }
}

function highlightQuery(content: string): string {
  const q = query.value.trim();
  if (!q) return escapeHtml(content);
  // Truncate long content
  const maxLen = 200;
  let text = content.length > maxLen ? content.slice(0, maxLen) + "..." : content;
  text = escapeHtml(text);
  // Highlight each search word
  for (const word of q.split(/\s+/)) {
    if (!word) continue;
    const escaped = word.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    text = text.replace(new RegExp(escaped, "gi"), "<mark>$&</mark>");
  }
  return text;
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function formatResultDate(ts: string): string {
  try {
    const date = new Date(ts + "Z");
    return date.toLocaleDateString("fr-FR", { day: "numeric", month: "short", year: "numeric" })
      + " " + date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  } catch {
    return ts;
  }
}
</script>

<style scoped>
.chat-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 18px;
  height: 52px;
  font-weight: 600;
  font-size: 0.9375rem;
  color: var(--header-primary);
  background: var(--bg-primary);
  border-bottom: 1px solid var(--border-soft);
  flex-shrink: 0;
  position: relative;
}

.header-spacer {
  flex: 1;
}

.header-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  margin: 0;
  padding: 0;
  border-radius: 6px;
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  transition: background 0.1s, color 0.1s;
}

.header-btn:hover {
  background: var(--bg-modifier-hover);
  color: var(--text-normal);
}

.header-btn.active {
  background: var(--accent-soft);
  color: var(--accent);
}

.channel-icon {
  color: var(--text-muted);
}

.topic-separator {
  width: 1px;
  height: 16px;
  background: var(--border);
  flex-shrink: 0;
  margin-left: 6px;
}

.topic {
  font-weight: 400;
  font-size: 0.8125rem;
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}

/* ── Search bar ── */
.search-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: 1;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 0 10px;
  height: 32px;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.search-bar:focus-within {
  border-color: var(--accent-line);
  box-shadow: 0 0 0 3px var(--accent-soft);
}

.search-icon {
  color: var(--text-faint);
  flex-shrink: 0;
}

.search-input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: var(--text-normal);
  font-size: 0.8125rem;
  font-family: inherit;
}

.search-input::placeholder {
  color: var(--text-faint);
}

.search-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  margin: 0;
  padding: 0;
  border-radius: 4px;
  background: transparent;
  border: none;
  color: var(--text-faint);
  cursor: pointer;
}

.search-close:hover {
  color: var(--text-normal);
}

/* ── Search results dropdown ── */
.search-results {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  max-height: 400px;
  overflow-y: auto;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-top: none;
  border-radius: 0 0 10px 10px;
  box-shadow: var(--shadow-pop);
  z-index: 40;
}

.search-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 16px;
  font-size: 0.8125rem;
  color: var(--text-muted);
}

.spinner {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.search-result {
  display: block;
  width: 100%;
  margin: 0;
  padding: 10px 16px;
  text-align: left;
  background: transparent;
  border: none;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  transition: background 0.1s;
}

.search-result:last-child {
  border-bottom: none;
}

.search-result:hover {
  background: var(--bg-modifier-hover);
}

.search-result-header {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-bottom: 2px;
}

.search-result-author {
  font-weight: 600;
  font-size: 0.8125rem;
  color: var(--header-primary);
}

.search-result-time {
  font-size: 0.6875rem;
  color: var(--text-faint);
  font-family: var(--font-mono);
}

.search-result-content {
  font-size: 0.8125rem;
  color: var(--text-normal);
  line-height: 1.4;
  word-break: break-word;
}

.search-result-content :deep(mark) {
  background: var(--accent-soft);
  color: var(--accent);
  border-radius: 2px;
  padding: 0 2px;
  font-weight: 600;
}

.skeleton {
  background: var(--bg-modifier-hover);
  border-radius: 4px;
  animation: skeleton-pulse 1.5s ease-in-out infinite;
}

.skeleton-icon {
  width: 18px;
  height: 18px;
  border-radius: 4px;
}

.skeleton-text {
  width: 120px;
  height: 16px;
}

@keyframes skeleton-pulse {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 0.8; }
}
</style>
