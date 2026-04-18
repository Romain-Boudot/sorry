/**
 * Message operations: send (optimistic), edit, delete, retry.
 * Extracted from store.ts — operates on the same reactive store object.
 */
import { reactive } from "vue";
import { api, type Message } from "../api";
import type { ServerState, SavedServer } from "../store";
import { wsSend } from "./helpers";

/** Delay after which a pending WS message is considered failed. */
const SEND_TIMEOUT_MS = 12_000;

/** Monotonically-decreasing ID generator for optimistic messages. */
let nextTempId = -1;
function tempId(): number {
  return nextTempId--;
}

function generateNonce(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) return crypto.randomUUID();
  return `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function findMessageByNonce(state: ServerState, nonce: string): Message | undefined {
  for (const [, msgs] of state.messages) {
    const m = msgs.find((msg) => msg.nonce === nonce);
    if (m) return m;
  }
  return undefined;
}

function buildReplyPreview(state: ServerState, replyToId?: number) {
  if (!replyToId) return undefined;
  for (const [, msgs] of state.messages) {
    const m = msgs.find((msg) => msg.id === replyToId);
    if (m) return { id: m.id, author_id: m.author_id, content: m.content };
  }
  return undefined;
}

/** Clean up any blob URLs held by an optimistic message's attachments. */
export function revokeOptimisticBlobs(msg: Message) {
  for (const att of msg.attachments) {
    if (att.url.startsWith("blob:")) URL.revokeObjectURL(att.url);
  }
}

/** Revoke blob URLs held by every message in the given list. Safe on real messages (no-op). */
export function revokeAllOptimisticBlobs(msgs: Iterable<Message>) {
  for (const m of msgs) revokeOptimisticBlobs(m);
}

function markFailedIfStillPending(state: ServerState, nonce: string) {
  const msg = findMessageByNonce(state, nonce);
  if (msg && msg.pending) {
    msg.pending = false;
    msg.failed = true;
  }
}

function scheduleTimeout(state: ServerState, nonce: string) {
  setTimeout(() => markFailedIfStillPending(state, nonce), SEND_TIMEOUT_MS);
}

export async function sendMessage(
  server: SavedServer,
  state: ServerState,
  content: string,
  files?: File[],
  replyToId?: number,
) {
  if (!state.activeChannelId) return;
  const trimmed = content.trim();
  const hasFiles = files && files.length > 0;
  if (!trimmed && !hasFiles) return;

  const nonce = generateNonce();
  const channelId = state.activeChannelId;

  const optimisticAttachments = (files ?? []).map((f, i) => ({
    id: -(i + 1),
    filename: f.name,
    content_type: f.type || "application/octet-stream",
    size: f.size,
    url: URL.createObjectURL(f),
  }));

  const optimistic = reactive<Message>({
    id: tempId(),
    channel_id: channelId,
    author_id: state.user?.id ?? 0,
    content,
    // Match server's SQLite CURRENT_TIMESTAMP format (no Z) — isGrouped/formatTime append Z themselves.
    created_at: new Date().toISOString().slice(0, 19).replace("T", " "),
    attachments: optimisticAttachments,
    reply_to: buildReplyPreview(state, replyToId),
    mentions: [],
    reactions: [],
    pinned: false,
    nonce,
    pending: true,
    pendingFiles: hasFiles ? [...files!] : undefined,
    pendingReplyToId: replyToId,
  });

  const msgs = state.messages.get(channelId);
  if (msgs) msgs.push(optimistic);
  else state.messages.set(channelId, [optimistic]);

  await dispatchSend(server, state, optimistic);
}

async function dispatchSend(server: SavedServer, state: ServerState, optimistic: Message) {
  const { channel_id, content, nonce, pendingFiles, pendingReplyToId } = optimistic;

  if (pendingFiles && pendingFiles.length > 0) {
    try {
      await api.sendMessageWithFiles(server.url, server.token, channel_id, content, pendingFiles, pendingReplyToId, nonce);
      // Success: the MessageCreate event will swap this optimistic entry.
      // Fallback timeout in case the WS event never arrives (e.g. WS disconnected).
      scheduleTimeout(state, nonce!);
    } catch {
      optimistic.pending = false;
      optimistic.failed = true;
    }
  } else {
    const sent = wsSend(state, {
      type: "SendMessage",
      data: { channel_id, content, reply_to_id: pendingReplyToId ?? null, nonce },
    });
    if (!sent) {
      optimistic.pending = false;
      optimistic.failed = true;
      return;
    }
    scheduleTimeout(state, nonce!);
  }
}

export async function retryMessage(server: SavedServer, state: ServerState, nonce: string) {
  const msg = findMessageByNonce(state, nonce);
  if (!msg || !msg.failed) return;
  msg.failed = false;
  msg.pending = true;
  await dispatchSend(server, state, msg);
}

export function discardFailedMessage(state: ServerState, nonce: string) {
  for (const [, msgs] of state.messages) {
    const idx = msgs.findIndex((m) => m.nonce === nonce);
    if (idx >= 0) {
      revokeOptimisticBlobs(msgs[idx]);
      msgs.splice(idx, 1);
      return;
    }
  }
}

/**
 * Consume an optimistic pending message matching `nonce` by swapping in the authoritative
 * server-side message. Returns true if a match was found and replaced.
 */
export function consumeOptimistic(state: ServerState, nonce: string, real: Message): boolean {
  const msgs = state.messages.get(real.channel_id);
  if (!msgs) return false;
  const idx = msgs.findIndex((m) => m.nonce === nonce);
  if (idx < 0) return false;
  revokeOptimisticBlobs(msgs[idx]);
  msgs[idx] = real;
  return true;
}

export function editMessage(state: ServerState, messageId: number, content: string) {
  wsSend(state, {
    type: "EditMessage",
    data: { message_id: messageId, content },
  });
}

export function deleteMessage(state: ServerState, messageId: number) {
  // Optimistic delete
  for (const [, msgs] of state.messages) {
    const idx = msgs.findIndex((m) => m.id === messageId);
    if (idx >= 0) {
      revokeOptimisticBlobs(msgs[idx]);
      msgs.splice(idx, 1);
      break;
    }
  }
  wsSend(state, {
    type: "DeleteMessage",
    data: { message_id: messageId },
  });
}

export function toggleReaction(state: ServerState, messageId: number, emoji: string) {
  wsSend(state, {
    type: "ToggleReaction",
    data: { message_id: messageId, emoji },
  });
}

const TYPING_THROTTLE = 2500; // ms

export function sendTyping(state: ServerState) {
  if (!state.activeChannelId) return;
  const now = Date.now();
  if (now - state.lastTypingSent < TYPING_THROTTLE) return;
  state.lastTypingSent = now;
  wsSend(state, {
    type: "Typing",
    data: { channel_id: state.activeChannelId },
  });
}
