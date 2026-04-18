<template>
  <div v-if="attachments?.length" class="message-attachments">
    <template v-for="att in attachments" :key="att.id">
      <button v-if="isImage(att)" class="attachment-image" @click="lightbox = { src: attachmentUrl(att), filename: att.filename }">
        <img :src="attachmentUrl(att)" :alt="att.filename" loading="lazy" />
      </button>
      <button v-else class="attachment-file" @click="downloadFile(att)">
        <FileText :size="16" />
        <span class="att-name">{{ att.filename }}</span>
        <span class="att-size">{{ formatSize(att.size) }}</span>
        <Download :size="14" />
      </button>
    </template>
  </div>
  <ImageLightbox
    v-if="lightbox"
    :src="lightbox.src"
    :filename="lightbox.filename"
    @close="lightbox = null"
  />
</template>

<script setup lang="ts">
import { ref } from "vue";
import { FileText, Download } from "lucide-vue-next";
import type { Attachment } from "../../api";
import { activeServer } from "../../store";
import ImageLightbox from "./ImageLightbox.vue";
import { showToast } from "../../composables/useToast";

const lightbox = ref<{ src: string; filename: string } | null>(null);

async function downloadFile(att: Attachment) {
  try {
    const res = await fetch(attachmentUrl(att));
    const blob = await res.blob();
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = att.filename;
    a.click();
    URL.revokeObjectURL(url);
    showToast(`${att.filename} telecharge`);
  } catch {
    showToast("Erreur lors du telechargement", "error");
  }
}

defineProps<{
  attachments: Attachment[] | undefined;
}>();

function isImage(att: Attachment): boolean {
  return att.content_type.startsWith("image/");
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} Ko`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} Mo`;
}

function attachmentUrl(att: Attachment): string {
  // Absolute URLs (incl. blob: from optimistic uploads) are used as-is.
  if (/^(blob:|https?:|data:)/.test(att.url)) return att.url;
  const server = activeServer();
  if (!server) return att.url;
  return `${server.url}${att.url}`;
}
</script>

<style scoped>
.message-attachments {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 4px;
}

.attachment-image {
  display: block;
  max-width: 400px;
  border-radius: 8px;
  overflow: hidden;
  cursor: pointer;
  background: transparent;
  border: none;
  padding: 0;
  margin: 0;
}

.attachment-image img {
  display: block;
  max-width: 100%;
  max-height: 300px;
  object-fit: contain;
  border-radius: 8px;
}

.attachment-file {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  width: auto;
  padding: 8px 12px;
  margin: 0;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text-normal);
  font-size: 0.8125rem;
  max-width: 300px;
  cursor: pointer;
  transition: background 0.15s;
}

.attachment-file:hover {
  background: var(--bg-modifier-hover);
}

.att-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
  min-width: 0;
}

.att-size {
  color: var(--text-muted);
  font-size: 0.75rem;
  flex-shrink: 0;
}
</style>
