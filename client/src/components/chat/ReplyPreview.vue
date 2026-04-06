<template>
  <div class="reply-preview" @click="$emit('click')">
    <div class="reply-spine"></div>
    <div class="reply-mini-avatar">
      <img v-if="avatarUrl" :src="avatarUrl" />
      <span v-else>{{ authorName[0]?.toUpperCase() }}</span>
    </div>
    <span class="reply-author" :style="authorColor ? `color:${authorColor}` : ''">{{ authorName }}</span>
    <span class="reply-content">{{ content }}</span>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  avatarUrl: string | null;
  authorName: string;
  authorColor: string | null;
  content: string;
}>();

defineEmits<{
  click: [];
}>();
</script>

<style scoped>
.reply-preview {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 0.75rem;
  color: var(--text-muted);
  margin-bottom: 4px;
  cursor: pointer;
  position: relative;
  padding-left: 0;
  min-height: 20px;
}

.reply-preview:hover .reply-content {
  color: var(--text-normal);
}

.reply-spine {
  position: absolute;
  left: -37px;
  top: 43%;
  width: 33px;
  height: calc(57% + 3px);
  border-left: 2px solid var(--text-faint);
  border-top: 2px solid var(--text-faint);
  border-top-left-radius: 8px;
}

.reply-mini-avatar {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.5rem;
  font-weight: 700;
  color: var(--text-bright);
  flex-shrink: 0;
  overflow: hidden;
}

.reply-mini-avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.reply-author {
  font-weight: 600;
  color: var(--header-primary);
  flex-shrink: 0;
  font-size: 0.75rem;
}

.reply-content {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  transition: color 0.1s;
}
</style>
