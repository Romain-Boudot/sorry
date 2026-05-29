import { activeState, activeServer } from "../core";
import * as _messaging from "../../composables/useMessaging";

// Thin wrappers that resolve activeState/activeServer automatically
export async function sendMessage(content: string, files?: File[], replyToId?: number) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;
  await _messaging.sendMessage(server, state, content, files, replyToId);
}

export async function retryMessage(nonce: string) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;
  await _messaging.retryMessage(server, state, nonce);
}

export function discardFailedMessage(nonce: string) {
  const state = activeState();
  if (!state) return;
  _messaging.discardFailedMessage(state, nonce);
}

export function editMessage(messageId: number, content: string) {
  const state = activeState();
  if (!state) return;
  _messaging.editMessage(state, messageId, content);
}

export function deleteMessage(messageId: number) {
  const state = activeState();
  if (!state) return;
  _messaging.deleteMessage(state, messageId);
}

export function toggleReaction(messageId: number, emoji: string) {
  const state = activeState();
  if (!state) return;
  _messaging.toggleReaction(state, messageId, emoji);
}

export function sendTyping() {
  const state = activeState();
  if (!state) return;
  _messaging.sendTyping(state);
}
