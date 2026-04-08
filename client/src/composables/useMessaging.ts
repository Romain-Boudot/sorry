/**
 * Message operations: send, edit, delete.
 * Extracted from store.ts — operates on the same reactive store object.
 */
import { api } from "../api";
import type { ServerState, SavedServer } from "../store";
import { wsSend } from "./helpers";

export async function sendMessage(
  server: SavedServer,
  state: ServerState,
  content: string,
  files?: File[],
  replyToId?: number,
) {
  if (!state.activeChannelId) return;
  if (!content.trim() && (!files || files.length === 0)) return;

  if (files && files.length > 0) {
    await api.sendMessageWithFiles(server.url, server.token, state.activeChannelId, content, files, replyToId);
  } else {
    wsSend(state, {
      type: "SendMessage",
      data: { channel_id: state.activeChannelId, content, reply_to_id: replyToId ?? null },
    });
  }
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
    if (idx >= 0) { msgs.splice(idx, 1); break; }
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
