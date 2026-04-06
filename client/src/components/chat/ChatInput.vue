<template>
  <div v-if="fileError" class="file-error">{{ fileError }}</div>
  <div class="chat-input">
    <div v-if="pendingFiles.length" class="pending-files">
      <div v-for="(file, i) in pendingFiles" :key="i" class="pending-file">
        <img v-if="file.type.startsWith('image/')" :src="objectUrls.get(file)" class="pending-thumb" />
        <FileText v-else :size="24" class="pending-file-icon" />
        <div class="pending-file-info">
          <span class="pending-file-name">{{ file.name }}</span>
          <span class="pending-file-size">{{ formatSize(file.size) }}</span>
        </div>
        <button class="pending-file-remove" @click="removeFile(i)">
          <X :size="14" />
        </button>
      </div>
    </div>
    <div v-if="replyingTo" class="reply-bar">
      <Reply :size="14" class="reply-bar-icon" />
      <span class="reply-bar-text">
        Reponse a <strong>{{ resolveUser(replyingTo.author_id) }}</strong>
        <span class="reply-bar-content">{{ replyingTo.content.slice(0, 80) }}{{ replyingTo.content.length > 80 ? '...' : '' }}</span>
      </span>
      <button class="reply-bar-close" @click="$emit('cancel-reply')">
        <X :size="14" />
      </button>
    </div>
    <div class="chat-input-wrapper">
      <!-- Mention autocomplete -->
      <div v-if="mentionSuggestions.length" class="mention-popup">
        <div
          v-for="(item, i) in mentionSuggestions"
          :key="item.key"
          class="mention-popup-item"
          :class="{ active: i === mentionIndex }"
          @mousedown.prevent="insertMention(item)"
        >
          <span v-if="item.type === 'role'" class="mention-role-dot" :style="item.color ? `background:${item.color}` : ''"></span>
          <span>{{ item.label }}</span>
          <span class="mention-type-tag">{{ item.type === 'role' ? 'role' : 'user' }}</span>
        </div>
      </div>
      <input type="file" ref="fileInput" multiple hidden @change="onFileSelect" />
      <button class="chat-attach" @click="fileInput?.click()" title="Joindre un fichier">
        <Paperclip :size="18" />
      </button>
      <textarea
        ref="mainInput"
        v-model="input"
        @keydown="onMainKeydown"
        @input="onInputChange"
        @paste="onPaste"
        :placeholder="`Envoyer un message dans #${channelName}`"
        rows="1"
      ></textarea>
      <button class="chat-send" @click="handleSend" :disabled="!input.trim() && !pendingFiles.length">
        <SendHorizonal :size="18" />
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, nextTick } from "vue";
import { SendHorizonal, Paperclip, X, FileText, Reply } from "lucide-vue-next";
import { activeState, activeServer, sendMessage, resolveUser } from "../../store";
import type { Message } from "../../api";

const props = defineProps<{
  channelName: string;
  replyingTo: Message | null;
  /** Callback to check if messages container is scrolled to bottom */
  isScrolledToBottom: () => boolean;
  /** Ref to the messages container for auto-resize scroll correction */
  messagesContainer: HTMLElement | undefined;
}>();

const emit = defineEmits<{
  'cancel-reply': [];
  sent: [];
  'edit-last': [msg: Message];
}>();

const MAX_FILES = 10;
const ALLOWED_EXTENSIONS = new Set([
  "png", "jpg", "jpeg", "gif", "webp", "svg",
  "mp4", "webm", "mov",
  "mp3", "ogg", "wav", "flac",
  "pdf", "txt", "json", "csv",
  "zip", "tar", "gz", "7z", "rar",
]);

const state = computed(() => activeState());
const MAX_FILE_SIZE = computed(() => state.value?.maxFileSize ?? 25 * 1024 * 1024);

const input = ref("");
const mainInput = ref<HTMLTextAreaElement>();
const fileInput = ref<HTMLInputElement>();
const pendingFiles = ref<File[]>([]);
const objectUrls = ref<Map<File, string>>(new Map());
const fileError = ref("");

// ── Mention autocomplete ──
interface MentionItem {
  key: string;
  type: "user" | "role";
  id: number;
  label: string;
  color?: string | null;
}

const mentionQuery = ref("");
const mentionStart = ref(-1);
const mentionIndex = ref(0);

const mentionSuggestions = computed((): MentionItem[] => {
  if (mentionStart.value < 0) return [];
  const q = mentionQuery.value.toLowerCase();
  const st = state.value;
  if (!st) return [];
  const items: MentionItem[] = [];
  for (const [id, user] of st.users) {
    if (user.display_name.toLowerCase().includes(q)) {
      items.push({ key: `u-${id}`, type: "user", id, label: user.display_name });
    }
  }
  for (const role of st.roles) {
    if (role.id === 1) continue;
    if (role.name.toLowerCase().includes(q)) {
      items.push({ key: `r-${role.id}`, type: "role", id: role.id, label: role.name, color: role.color });
    }
  }
  return items.slice(0, 10);
});

function updateMentionState() {
  const el = mainInput.value;
  if (!el) { mentionStart.value = -1; return; }
  const pos = el.selectionStart ?? 0;
  const text = input.value.slice(0, pos);
  const atIdx = text.lastIndexOf("@");
  if (atIdx < 0 || (atIdx > 0 && text[atIdx - 1] !== " " && text[atIdx - 1] !== "\n")) {
    mentionStart.value = -1;
    return;
  }
  const query = text.slice(atIdx + 1);
  if (query.includes(" ") && query.length > 20) {
    mentionStart.value = -1;
    return;
  }
  mentionStart.value = atIdx;
  mentionQuery.value = query;
  mentionIndex.value = 0;
}

function insertMention(item: MentionItem) {
  const el = mainInput.value;
  if (!el || mentionStart.value < 0) return;
  const pos = el.selectionStart ?? 0;
  const before = input.value.slice(0, mentionStart.value);
  const after = input.value.slice(pos);
  const mentionTag = item.type === "role" ? `<@&${item.id}>` : `<@${item.id}>`;
  input.value = before + mentionTag + " " + after;
  mentionStart.value = -1;
  nextTick(() => {
    const newPos = before.length + mentionTag.length + 1;
    el.setSelectionRange(newPos, newPos);
    el.focus();
  });
}

function onInputChange(e: Event) {
  autoResize(e);
  updateMentionState();
}

function autoResize(e: Event) {
  const wasAtBottom = props.isScrolledToBottom();
  const el = e.target as HTMLTextAreaElement;
  el.style.height = "auto";
  el.style.height = el.scrollHeight + "px";
  el.style.overflowY = el.scrollHeight > el.offsetHeight ? "auto" : "hidden";
  if (wasAtBottom) {
    nextTick(() => {
      const container = props.messagesContainer;
      if (container) container.scrollTop = container.scrollHeight;
    });
  }
}

function trimMessage(s: string): string {
  return s.replace(/^\s*\n/, "").replace(/\n\s*$/, "").trim();
}

function getExtension(name: string): string {
  return (name.split(".").pop() || "").toLowerCase();
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} Ko`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} Mo`;
}

function addFiles(fileList: FileList | File[]) {
  fileError.value = "";
  for (const file of fileList) {
    if (pendingFiles.value.length >= MAX_FILES) {
      fileError.value = `Maximum ${MAX_FILES} fichiers par message`;
      break;
    }
    if (file.size === 0) continue;
    if (file.size > MAX_FILE_SIZE.value) {
      fileError.value = `${file.name} est trop volumineux (max ${Math.round(MAX_FILE_SIZE.value / 1024 / 1024)} Mo)`;
      continue;
    }
    if (!ALLOWED_EXTENSIONS.has(getExtension(file.name))) {
      fileError.value = `${file.name} : type de fichier non autorise`;
      continue;
    }
    pendingFiles.value.push(file);
    if (file.type.startsWith("image/")) {
      objectUrls.value.set(file, URL.createObjectURL(file));
    }
  }
  if (fileError.value) setTimeout(() => (fileError.value = ""), 4000);
}

function removeFile(index: number) {
  const file = pendingFiles.value[index];
  const url = objectUrls.value.get(file);
  if (url) {
    URL.revokeObjectURL(url);
    objectUrls.value.delete(file);
  }
  pendingFiles.value.splice(index, 1);
}

function onFileSelect(e: Event) {
  const input = e.target as HTMLInputElement;
  if (input.files) addFiles(input.files);
  input.value = "";
}

function onPaste(e: ClipboardEvent) {
  const items = e.clipboardData?.items;
  if (!items) return;
  const files: File[] = [];
  for (const item of items) {
    if (item.kind === "file") {
      const file = item.getAsFile();
      if (file) files.push(file);
    }
  }
  if (files.length) {
    e.preventDefault();
    addFiles(files);
  }
}

function handleSend() {
  const content = trimMessage(input.value);
  const files = pendingFiles.value.length > 0 ? [...pendingFiles.value] : undefined;
  if (!content && !files) return;
  sendMessage(content || "", files, props.replyingTo?.id);
  input.value = "";
  emit('cancel-reply');
  for (const [, url] of objectUrls.value) URL.revokeObjectURL(url);
  objectUrls.value.clear();
  pendingFiles.value = [];
  nextTick(() => {
    if (mainInput.value) {
      mainInput.value.style.height = "auto";
    }
  });
  emit('sent');
}

function onMainKeydown(e: KeyboardEvent) {
  // Mention autocomplete navigation
  if (mentionSuggestions.value.length > 0) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      mentionIndex.value = (mentionIndex.value + 1) % mentionSuggestions.value.length;
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      mentionIndex.value = (mentionIndex.value - 1 + mentionSuggestions.value.length) % mentionSuggestions.value.length;
      return;
    }
    if (e.key === "Enter" || e.key === "Tab") {
      e.preventDefault();
      insertMention(mentionSuggestions.value[mentionIndex.value]);
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      mentionStart.value = -1;
      return;
    }
  }

  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    handleSend();
    return;
  }
  if (e.key === "ArrowUp" && !input.value) {
    const st = state.value;
    if (!st?.activeChannelId || !st.user) return;
    const msgs = st.messages.get(st.activeChannelId);
    if (!msgs) return;
    for (let i = msgs.length - 1; i >= 0; i--) {
      if (msgs[i].author_id === st.user.id) {
        emit('edit-last', msgs[i]);
        e.preventDefault();
        return;
      }
    }
  }
}

// Expose addFiles so parent can call it for drag/drop
defineExpose({ addFiles, focus: () => mainInput.value?.focus() });
</script>

<style scoped>
.chat-input {
  padding: 0 8px 8px;
}

.chat-input-wrapper {
  position: relative;
  display: flex;
  align-items: flex-end;
  min-height: calc(var(--bar-height) + 2px);
  background: var(--bg-floating);
  border-radius: 8px;
  border: 1px solid var(--border);
  padding-right: 4px;
}

.chat-input textarea {
  width: 100%;
  min-height: var(--bar-height);
  padding: 13px 16px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: var(--text-normal);
  font-size: 0.9375rem;
  font-family: inherit;
  outline: none;
  resize: none;
  overflow: hidden;
  overflow-wrap: break-word;
  line-height: 1.375;
  max-height: 200px;
}

.chat-input textarea::placeholder {
  color: var(--text-faint);
}

.chat-send {
  width: 32px;
  height: var(--bar-height);
  padding: 0;
  margin: 0;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  background: transparent;
  color: var(--text-muted);
  transition: color 0.1s;
}

.chat-send:hover { color: var(--text-normal); box-shadow: none; }
.chat-send:disabled { color: var(--text-faint); opacity: 0.5; }

.chat-attach {
  width: 32px;
  height: var(--bar-height);
  padding: 0;
  margin: 0 0 0 4px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  background: transparent;
  color: var(--text-muted);
  transition: color 0.1s;
}

.chat-attach:hover {
  color: var(--text-normal);
  box-shadow: none;
}

/* ── Pending files ── */
.file-error {
  padding: 6px 16px;
  font-size: 0.75rem;
  color: var(--danger);
}

.pending-files {
  display: flex;
  gap: 8px;
  padding: 8px 8px 0;
  flex-wrap: wrap;
}

.pending-file {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 8px;
  max-width: 200px;
  position: relative;
}

.pending-thumb {
  width: 40px;
  height: 40px;
  object-fit: cover;
  border-radius: 4px;
  flex-shrink: 0;
}

.pending-file-icon {
  color: var(--text-muted);
  flex-shrink: 0;
}

.pending-file-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
  flex: 1;
}

.pending-file-name {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-normal);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pending-file-size {
  font-size: 0.6875rem;
  color: var(--text-muted);
}

.pending-file-remove {
  width: 20px;
  height: 20px;
  padding: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  background: var(--bg-modifier-hover);
  color: var(--text-muted);
  cursor: pointer;
  border: none;
  flex-shrink: 0;
}

.pending-file-remove:hover {
  background: var(--danger);
  color: var(--text-bright);
}

/* ── Reply bar above input ── */
.reply-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: var(--bg-secondary);
  border-radius: 8px 8px 0 0;
  border: 1px solid var(--border);
  border-bottom: none;
  font-size: 0.8125rem;
  color: var(--text-muted);
}

.reply-bar + .chat-input-wrapper {
  border-radius: 0 0 8px 8px;
}

.reply-bar-icon {
  flex-shrink: 0;
  color: var(--accent);
}

.reply-bar-text {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.reply-bar-text strong {
  color: var(--header-primary);
}

.reply-bar-content {
  margin-left: 6px;
  color: var(--text-faint);
}

.reply-bar-close {
  width: 24px;
  height: 24px;
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
  flex-shrink: 0;
}

.reply-bar-close:hover {
  color: var(--text-normal);
  background: var(--bg-modifier-hover);
  box-shadow: none;
}

/* ── Mention autocomplete popup ── */
.mention-popup {
  position: absolute;
  bottom: 100%;
  left: 0;
  right: 0;
  background: var(--bg-tertiary);
  border-radius: 8px;
  padding: 6px;
  margin-bottom: 4px;
  box-shadow: 0 8px 16px rgba(0, 0, 0, 0.24);
  max-height: 200px;
  overflow-y: auto;
  z-index: 50;
}

.mention-popup-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--text-normal);
}

.mention-popup-item:hover,
.mention-popup-item.active {
  background: var(--accent);
  color: var(--text-bright);
}

.mention-role-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--text-faint);
  flex-shrink: 0;
}

.mention-type-tag {
  margin-left: auto;
  font-size: 0.6875rem;
  color: var(--text-faint);
  font-weight: 400;
}

.mention-popup-item:hover .mention-type-tag,
.mention-popup-item.active .mention-type-tag {
  color: rgba(255, 255, 255, 0.6);
}
</style>
