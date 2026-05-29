<template>
  <div class="composer">
    <div v-if="error" class="composer-error">{{ error }}</div>
    <slot name="above" />
    <div v-if="replyingTo" class="composer-reply-bar">
      <Reply :size="14" class="composer-reply-icon" />
      <span class="composer-reply-text">
        Reponse a <strong>{{ replyingTo.authorName }}</strong>
        <span class="composer-reply-snippet">{{ replySnippet }}</span>
      </span>
      <button class="composer-reply-close" @click="$emit('cancel-reply')">
        <X :size="14" />
      </button>
    </div>
    <div class="composer-wrapper" :class="{ 'has-reply': !!replyingTo }">
      <slot name="popups" />
      <slot name="left" />
      <textarea
        ref="textareaEl"
        :value="modelValue"
        @input="onInput"
        @keydown="onKeydown"
        @paste="$emit('paste', $event)"
        :placeholder="placeholder"
        rows="1"
      />
      <button class="composer-send" @click="$emit('submit')" :disabled="disabled">
        <SendHorizonal :size="18" />
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { SendHorizonal, Reply, X } from "lucide-vue-next";

const props = defineProps<{
  modelValue: string;
  placeholder: string;
  disabled?: boolean;
  error?: string;
  replyingTo?: { authorName: string; content: string } | null;
  /** Container scrolled by content; used to keep position pinned to bottom on textarea growth. */
  scrollContainer?: HTMLElement | null;
  /** Callback to know if the scroll container is at bottom before textarea auto-grow. */
  isScrolledToBottom?: () => boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: string];
  submit: [];
  "cancel-reply": [];
  paste: [event: ClipboardEvent];
  keydown: [event: KeyboardEvent];
  input: [];
}>();

const textareaEl = ref<HTMLTextAreaElement>();

const replySnippet = computed(() => {
  const c = props.replyingTo?.content ?? "";
  return c.length > 80 ? c.slice(0, 80) + "..." : c;
});

function onInput(e: Event) {
  const t = e.target as HTMLTextAreaElement;
  emit("update:modelValue", t.value);
  emit("input");
  autoResize(t);
}

function onKeydown(e: KeyboardEvent) {
  emit("keydown", e);
  if (e.defaultPrevented) return;
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    emit("submit");
  }
}

function autoResize(el: HTMLTextAreaElement) {
  const wasAtBottom = props.isScrolledToBottom?.() ?? false;
  el.style.height = "auto";
  el.style.height = el.scrollHeight + "px";
  el.style.overflowY = el.scrollHeight > el.offsetHeight ? "auto" : "hidden";
  if (wasAtBottom && props.scrollContainer) {
    nextTick(() => {
      const c = props.scrollContainer!;
      c.scrollTop = c.scrollHeight;
    });
  }
}

function reset() {
  if (textareaEl.value) textareaEl.value.style.height = "auto";
}

defineExpose({
  focus: () => textareaEl.value?.focus(),
  textarea: () => textareaEl.value ?? null,
  reset,
});
</script>

<style scoped>
.composer {
  padding: 0 8px 8px;
}

.composer-error {
  padding: 6px 16px;
  font-size: 0.75rem;
  color: var(--danger);
}

.composer-wrapper {
  position: relative;
  display: flex;
  align-items: flex-end;
  min-height: calc(var(--bar-height) + 2px);
  background: var(--bg-floating);
  border-radius: 10px;
  border: 1px solid var(--border);
  padding-right: 4px;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.composer-wrapper:focus-within {
  border-color: var(--accent-line);
  box-shadow: 0 0 0 3px var(--accent-soft);
}

.composer-wrapper.has-reply {
  border-radius: 0 0 10px 10px;
}

.composer textarea {
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

.composer textarea::placeholder {
  color: var(--text-faint);
}

.composer-send {
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

.composer-send:hover { color: var(--text-normal); box-shadow: none; }
.composer-send:disabled { color: var(--text-faint); opacity: 0.5; }

.composer-reply-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: var(--bg-secondary);
  border-radius: 10px 10px 0 0;
  border: 1px solid var(--border);
  border-bottom: none;
  font-size: 0.8125rem;
  color: var(--text-muted);
}

.composer-reply-icon {
  flex-shrink: 0;
  color: var(--accent);
}

.composer-reply-text {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.composer-reply-text strong {
  color: var(--header-primary);
}

.composer-reply-snippet {
  margin-left: 6px;
  color: var(--text-faint);
}

.composer-reply-close {
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

.composer-reply-close:hover {
  color: var(--text-normal);
  background: var(--bg-modifier-hover);
  box-shadow: none;
}
</style>
