<template>
  <div class="gallery-panel">
    <div class="gallery-header">
      <span class="gallery-title">Fichiers</span>
      <div class="gallery-tabs">
        <button
          class="gallery-tab"
          :class="{ active: tab === 'media' }"
          @click="tab = 'media'"
        >Media</button>
        <button
          class="gallery-tab"
          :class="{ active: tab === 'files' }"
          @click="tab = 'files'"
        >Fichiers</button>
      </div>
      <button class="gallery-close" @click="$emit('close')"><X :size="16" /></button>
    </div>

    <div class="gallery-body" ref="scrollContainer" @scroll="onScroll">
      <div v-if="loading && !items.length" class="gallery-status">
        <Loader2 :size="16" class="spinner" /> Chargement...
      </div>
      <div v-else-if="!filtered.length" class="gallery-status">
        {{ tab === 'media' ? 'Aucun media' : 'Aucun fichier' }}
      </div>

      <!-- Media grid -->
      <div v-if="tab === 'media' && filtered.length" class="gallery-grid">
        <a
          v-for="att in filtered"
          :key="att.id"
          :href="baseUrl + att.url"
          target="_blank"
          class="gallery-thumb"
        >
          <img v-if="isImage(att)" :src="baseUrl + att.url" :alt="att.filename" loading="lazy" />
          <div v-else class="gallery-video-thumb">
            <Play :size="24" />
            <span>{{ att.filename }}</span>
          </div>
        </a>
      </div>

      <!-- File list -->
      <div v-if="tab === 'files' && filtered.length" class="gallery-list">
        <a
          v-for="att in filtered"
          :key="att.id"
          :href="baseUrl + att.url"
          target="_blank"
          class="gallery-file"
        >
          <FileIcon :size="16" />
          <div class="gallery-file-info">
            <span class="gallery-file-name">{{ att.filename }}</span>
            <span class="gallery-file-meta">
              {{ resolveUser(att.author_id) }} · {{ formatDate(att.created_at) }} · {{ formatSize(att.size) }}
            </span>
          </div>
        </a>
      </div>

      <div v-if="loading && items.length" class="gallery-status">
        <Loader2 :size="14" class="spinner" /> Chargement...
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { X, Loader2, Play, FileIcon } from "lucide-vue-next";
import { activeState, activeServer, resolveUser } from "../../store";
import { api, type ChannelAttachment } from "../../api";

defineEmits<{ close: [] }>();

const state = computed(() => activeState());
const server = computed(() => activeServer());
const baseUrl = computed(() => server.value?.url ?? "");

const tab = ref<"media" | "files">("media");
const items = ref<ChannelAttachment[]>([]);
const loading = ref(false);
const hasMore = ref(false);
const scrollContainer = ref<HTMLElement>();

const filtered = computed(() => {
  if (tab.value === "media") {
    return items.value.filter(a => isMedia(a));
  }
  return items.value.filter(a => !isMedia(a));
});

function isMedia(att: ChannelAttachment): boolean {
  return att.content_type.startsWith("image/") || att.content_type.startsWith("video/");
}

function isImage(att: ChannelAttachment): boolean {
  return att.content_type.startsWith("image/");
}

async function load(before?: number) {
  const s = server.value;
  const channelId = state.value?.activeChannelId;
  if (!s || !channelId) return;

  loading.value = true;
  try {
    const data = await api.channelAttachments(s.url, s.token, channelId, 50, before);
    if (before) {
      items.value.push(...data);
    } else {
      items.value = data;
    }
    hasMore.value = data.length >= 50;
  } catch {
    if (!before) items.value = [];
    hasMore.value = false;
  } finally {
    loading.value = false;
  }
}

function loadMore() {
  if (!items.value.length || loading.value || !hasMore.value) return;
  const lastId = items.value[items.value.length - 1].id;
  load(lastId);
}

function onScroll() {
  const el = scrollContainer.value;
  if (!el) return;
  if (el.scrollHeight - el.scrollTop - el.clientHeight < 100) {
    loadMore();
  }
}

// Reload when channel changes
watch(() => state.value?.activeChannelId, () => load());

onMounted(() => load());

function formatDate(ts: string): string {
  try {
    const d = new Date(ts + "Z");
    return d.toLocaleDateString("fr-FR", { day: "numeric", month: "short" });
  } catch {
    return ts;
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} Ko`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} Mo`;
}
</script>

<style scoped>
.gallery-panel {
  width: 300px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-secondary);
  border-left: 1px solid var(--border);
}

.gallery-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.gallery-title {
  font-weight: 700;
  font-size: 0.8125rem;
  color: var(--header-primary);
}

.gallery-tabs {
  display: flex;
  gap: 0;
  background: var(--bg-tertiary);
  border-radius: 6px;
  padding: 2px;
  margin-left: auto;
}

.gallery-tab {
  padding: 3px 8px;
  margin: 0;
  border-radius: 4px;
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--text-muted);
  background: transparent;
  cursor: pointer;
  border: none;
  transition: background 0.15s, color 0.15s;
}

.gallery-tab:hover { color: var(--text-normal); }
.gallery-tab.active { background: var(--bg-primary); color: var(--text-normal); }

.gallery-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  margin: 0;
  padding: 0;
  border-radius: 4px;
  background: transparent;
  border: none;
  color: var(--text-faint);
  cursor: pointer;
}

.gallery-close:hover {
  color: var(--text-normal);
  background: var(--bg-modifier-hover);
}

.gallery-body {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.gallery-status {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 24px 8px;
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

/* ── Media grid ── */
.gallery-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 4px;
}

.gallery-thumb {
  aspect-ratio: 1;
  border-radius: 4px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-tertiary);
}

.gallery-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.gallery-video-thumb {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 8px;
  color: var(--text-muted);
  text-align: center;
}

.gallery-video-thumb span {
  font-size: 0.5625rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 100%;
}

/* ── File list ── */
.gallery-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.gallery-file {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px;
  border-radius: 6px;
  text-decoration: none;
  color: var(--text-normal);
  transition: background 0.1s;
}

.gallery-file:hover {
  background: var(--bg-modifier-hover);
}

.gallery-file-info {
  min-width: 0;
  flex: 1;
  display: flex;
  flex-direction: column;
}

.gallery-file-name {
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--accent);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.gallery-file-meta {
  font-size: 0.6875rem;
  color: var(--text-faint);
}
</style>
