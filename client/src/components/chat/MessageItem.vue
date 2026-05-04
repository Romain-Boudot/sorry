<template>
  <div
    class="message"
    :class="messageClasses"
    :data-msg-id="id"
    @contextmenu.prevent="$emit('context-menu', $event)"
  >
    <!-- Hover toolbar -->
    <div v-if="canHover && actions.length" class="message-actions">
      <button
        v-for="action in actions"
        :key="action.key"
        class="msg-action-btn"
        :class="{ 'quick-emoji': !!action.emoji, danger: action.danger }"
        :title="action.label"
        @click="action.handler($event)"
      >
        <template v-if="action.emoji">{{ action.emoji }}</template>
        <component v-else-if="action.icon" :is="action.icon" :size="14" />
      </button>
    </div>

    <!-- Avatar (full layout) or gutter (grouped layout) -->
    <div
      v-if="!grouped"
      class="message-avatar"
      :class="{ 'has-reply': !!replyTo, clickable: canOpenCard }"
      @click="canOpenCard && $emit('open-author-card', $event)"
    >
      <img v-if="authorAvatar" :src="authorAvatar" />
      <span v-else>{{ authorInitial }}</span>
    </div>
    <div v-else class="message-gutter">
      <span class="message-time-hover">{{ formattedShortTime }}</span>
    </div>

    <!-- Body -->
    <div class="message-body">
      <ReplyPreview
        v-if="replyTo"
        :avatar-url="replyTo.avatarUrl"
        :author-name="replyTo.authorName"
        :author-color="replyTo.authorColor"
        :content="replyTo.content"
        @click="$emit('scroll-to-reply', replyTo.id)"
      />

      <div v-if="!grouped" class="message-header">
        <span
          class="message-author"
          :class="{ clickable: canOpenCard }"
          :style="authorColor ? `color:${authorColor}` : ''"
          @click="canOpenCard && $emit('open-author-card', $event)"
        >{{ authorName }}</span>
        <span v-if="isWebhook" class="bot-tag" title="Message envoye par un webhook">BOT</span>
        <span v-else-if="isGuest" class="guest-tag">Guest</span>
        <span class="message-time">{{ formattedTime }}</span>
        <Pin v-if="pinned" :size="12" class="pin-icon" title="Message epingle" />
      </div>

      <!-- Edit textarea -->
      <template v-if="editing">
        <textarea
          ref="editEl"
          class="message-edit-input"
          :value="editContent"
          rows="1"
          @input="onEditInput"
          @keydown="onEditKeydown"
        />
        <div class="message-edit-hint">Echap pour annuler · Entree pour enregistrer</div>
      </template>

      <!-- Undecryptable (DMs) -->
      <span v-else-if="undecryptable" class="message-undecryptable">
        <LockKeyholeOpen :size="12" />
        Impossible de dechiffrer
      </span>

      <!-- Markdown content -->
      <div
        v-else-if="content"
        class="message-content"
        v-html="renderedContent"
      />

      <LinkPreview v-for="url in extractedUrls" :key="url" :url="url" />
      <AttachmentList v-if="attachments?.length" :attachments="attachments" />

      <MessageReactions
        v-if="(reactions?.length ?? 0) > 0"
        :reactions="reactions ?? []"
        :my-id="myUserId"
        @toggle="$emit('react', $event)"
        @open-picker="$emit('open-emoji-picker', $event)"
      />

      <div v-if="failed && nonce" class="message-failed-bar">
        <AlertCircle :size="12" />
        <span class="message-failed-label">Echec d'envoi</span>
        <button type="button" class="message-failed-btn" @click="$emit('retry')">Reessayer</button>
        <span class="message-failed-sep">·</span>
        <button type="button" class="message-failed-btn" @click="$emit('discard')">Supprimer</button>
      </div>

      <div v-if="pending && pendingLabel" class="message-pending-meta">{{ pendingLabel }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch, type Component } from "vue";
import { Pin, AlertCircle, LockKeyholeOpen } from "lucide-vue-next";
import { renderMarkdown, extractUrls } from "../../markdown";
import ReplyPreview from "./ReplyPreview.vue";
import AttachmentList from "./AttachmentList.vue";
import MessageReactions from "./MessageReactions.vue";
import LinkPreview from "../LinkPreview.vue";
import type { Reaction, Attachment } from "../../api";

export interface MessageAction {
  key: string;
  label: string;
  /** Emoji glyph for quick-react buttons. Mutually exclusive with `icon`. */
  emoji?: string;
  icon?: Component;
  danger?: boolean;
  handler: (event: MouseEvent) => void;
}

export interface ReplyPreviewData {
  id: number;
  authorName: string;
  authorColor: string | null;
  avatarUrl: string | null;
  content: string;
}

const props = withDefaults(defineProps<{
  id: number;
  authorName: string;
  authorAvatar: string | null;
  authorColor: string | null;
  content: string;
  createdAt: string;
  myUserId: number;

  /** Render plaintext (DMs) instead of markdown when false. */
  markdown?: boolean;
  edited?: boolean;
  pinned?: boolean;
  pending?: boolean;
  failed?: boolean;
  undecryptable?: boolean;
  isWebhook?: boolean;
  isGuest?: boolean;
  /** When true the avatar/author are clickable to open the user card. */
  canOpenCard?: boolean;

  /** Hide header & avatar (grouped consecutive messages). */
  grouped?: boolean;

  reactions?: Reaction[];
  attachments?: Attachment[];
  replyTo?: ReplyPreviewData | null;

  /** Hover toolbar actions. Empty list = no toolbar. */
  actions?: MessageAction[];

  /** Edit mode controlled by parent (v-model:edit-content / submit-edit / cancel-edit). */
  editing?: boolean;
  editContent?: string;

  /** Optional nonce for failed channel sends — drives the retry/discard bar. */
  nonce?: string | null;
  /** Inline pending hint shown below the message (e.g. "Envoi..."). */
  pendingLabel?: string;
}>(), {
  markdown: true,
  canOpenCard: true,
  actions: () => [],
});

const emit = defineEmits<{
  react: [emoji: string];
  "open-emoji-picker": [event: MouseEvent];
  "open-author-card": [event: MouseEvent];
  "scroll-to-reply": [messageId: number];
  "context-menu": [event: MouseEvent];
  "update:edit-content": [value: string];
  "submit-edit": [];
  "cancel-edit": [];
  retry: [];
  discard: [];
}>();

const editEl = ref<HTMLTextAreaElement>();

const canHover = computed(() => !props.editing && !props.pending && !props.failed && !props.undecryptable);

const messageClasses = computed(() => ({
  grouped: props.grouped,
  editing: props.editing,
  pending: props.pending,
  failed: props.failed,
}));

const authorInitial = computed(() => (props.authorName[0] ?? "?").toUpperCase());

const renderedContent = computed(() => {
  if (!props.content) return "";
  if (!props.markdown) return escapeHtml(props.content).replace(/\n/g, "<br>");
  return renderMarkdown(props.content);
});

const extractedUrls = computed(() => {
  if (!props.content || props.editing || props.undecryptable) return [];
  return extractUrls(props.content);
});

const formattedTime = computed(() => formatRelativeTime(props.createdAt, !!props.edited));
const formattedShortTime = computed(() => formatShortTime(props.createdAt));

watch(() => props.editing, (active) => {
  if (active) nextTick(() => {
    const el = editEl.value;
    if (!el) return;
    el.focus();
    autoResizeEdit(el);
  });
});

function onEditInput(e: Event) {
  const t = e.target as HTMLTextAreaElement;
  emit("update:edit-content", t.value);
  autoResizeEdit(t);
}

function onEditKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    emit("submit-edit");
  } else if (e.key === "Escape") {
    e.preventDefault();
    emit("cancel-edit");
  }
}

function autoResizeEdit(el: HTMLTextAreaElement) {
  el.style.height = "auto";
  el.style.height = el.scrollHeight + "px";
  el.style.overflowY = el.scrollHeight > el.offsetHeight ? "auto" : "hidden";
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function formatRelativeTime(ts: string, edited: boolean): string {
  try {
    const date = new Date(ts.endsWith("Z") ? ts : ts + "Z");
    const now = new Date();
    const yesterday = new Date(now);
    yesterday.setDate(yesterday.getDate() - 1);
    const time = date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    let label: string;
    if (date.toDateString() === now.toDateString()) label = time;
    else if (date.toDateString() === yesterday.toDateString()) label = `Hier ${time}`;
    else label = `${date.toLocaleDateString("fr-FR", { day: "2-digit", month: "2-digit", year: "numeric" })} ${time}`;
    return edited ? `${label} · modifie` : label;
  } catch {
    return ts;
  }
}

function formatShortTime(ts: string): string {
  try {
    const date = new Date(ts.endsWith("Z") ? ts : ts + "Z");
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  } catch {
    return ts;
  }
}
</script>

<style scoped>
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

.message.editing {
  background: var(--bg-modifier-active);
}

.message.pending { opacity: 0.55; }
.message.failed { opacity: 0.9; }

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
  color: var(--text-bright);
  flex-shrink: 0;
  margin-top: 2px;
  overflow: hidden;
}

.message-avatar.clickable { cursor: pointer; }
.message-avatar.has-reply { margin-top: 26px; }

.message-avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
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
}

.message-author.clickable { cursor: pointer; }
.message-author.clickable:hover { text-decoration: underline; }

.message-time {
  font-size: 0.6875rem;
  color: var(--text-muted);
  font-weight: 400;
}

.pin-icon {
  color: var(--text-faint);
  flex-shrink: 0;
}

.message-content {
  color: var(--text-normal);
  line-height: 1.375rem;
  word-break: break-word;
  font-size: 0.9375rem;
}

.message-content :deep(p) { margin: 0; }
.message-content :deep(p + p) { margin-top: 4px; }
.message-content :deep(a) { color: var(--accent); text-decoration: none; }
.message-content :deep(a:hover) { text-decoration: underline; }
.message-content :deep(strong) { font-weight: 700; color: var(--header-primary); }
.message-content :deep(em) { font-style: italic; }
.message-content :deep(del) { text-decoration: line-through; color: var(--text-muted); }

.message-content :deep(code) {
  background: var(--bg-tertiary);
  padding: 1px 5px;
  border-radius: 4px;
  font-size: 0.85em;
  font-family: monospace;
}

.message-content :deep(pre) {
  background: var(--bg-tertiary);
  padding: 10px 12px;
  border-radius: 6px;
  overflow-x: auto;
  max-height: 300px;
  overflow-y: auto;
  margin: 4px 0;
}

.message-content :deep(pre code) {
  background: none;
  padding: 0;
  border-radius: 0;
  font-size: 0.85em;
}

.message-content :deep(h3),
.message-content :deep(h4),
.message-content :deep(h5),
.message-content :deep(h6) {
  font-weight: 700;
  color: var(--header-primary);
  margin: 8px 0 4px;
}

.message-content :deep(h3) { font-size: 1.1em; }
.message-content :deep(h4) { font-size: 1em; }

.message-content :deep(hr) {
  border: none;
  border-top: 1px solid var(--border);
  margin: 8px 0;
}

.message-content :deep(table) {
  border-collapse: collapse;
  max-width: 100%;
  overflow-x: auto;
  display: block;
  margin: 4px 0;
  font-size: 0.875em;
}

.message-content :deep(th),
.message-content :deep(td) {
  border: 1px solid var(--border);
  padding: 4px 8px;
}

.message-content :deep(th) {
  background: var(--bg-tertiary);
  font-weight: 600;
}

.message-content :deep(blockquote) {
  border-left: 3px solid var(--accent);
  margin: 4px 0;
  padding: 2px 12px;
  color: var(--text-muted);
}

.message-content :deep(ul),
.message-content :deep(ol) {
  margin: 4px 0;
  padding-left: 24px;
}

.message-undecryptable {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 0.8125rem;
  font-style: italic;
  color: var(--text-faint);
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

.message-pending-meta {
  font-size: 0.6875rem;
  color: var(--text-faint);
  margin-top: 2px;
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

.msg-action-btn.quick-emoji {
  font-size: 0.875rem;
  line-height: 1;
}

.msg-action-btn.danger:hover {
  background: var(--danger);
  color: var(--text-bright);
}

.message-failed-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 4px;
  font-size: 0.75rem;
  color: var(--danger);
}

.message-failed-label { font-weight: 500; }
.message-failed-sep { color: var(--text-faint); }

.message-failed-btn {
  width: auto;
  background: none;
  border: none;
  padding: 0;
  margin: 0;
  color: var(--danger);
  font-size: 0.75rem;
  font-weight: 500;
  cursor: pointer;
  text-decoration: underline;
  text-underline-offset: 2px;
}

.message-failed-btn:hover {
  color: var(--text-bright);
  background: none;
}
</style>
