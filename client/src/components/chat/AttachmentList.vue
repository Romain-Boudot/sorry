<template>
  <div v-if="attachments?.length" class="message-attachments">
    <template v-for="att in attachments" :key="att.id">
      <a v-if="isImage(att)" :href="attachmentUrl(att)" target="_blank" class="attachment-image">
        <img :src="attachmentUrl(att)" :alt="att.filename" loading="lazy" />
      </a>
      <a v-else :href="attachmentUrl(att)" :download="att.filename" target="_blank" class="attachment-file">
        <FileText :size="16" />
        <span class="att-name">{{ att.filename }}</span>
        <span class="att-size">{{ formatSize(att.size) }}</span>
        <Download :size="14" />
      </a>
    </template>
  </div>
</template>

<script setup lang="ts">
import { FileText, Download } from "lucide-vue-next";
import type { Attachment } from "../../api";
import { activeServer } from "../../store";

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
}

.attachment-image img {
  display: block;
  max-width: 100%;
  max-height: 300px;
  object-fit: contain;
  border-radius: 8px;
}

.attachment-file {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text-normal);
  text-decoration: none;
  font-size: 0.8125rem;
  max-width: 300px;
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
