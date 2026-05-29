import { activeState, activeServer } from "../core";
import * as _dms from "../../composables/useDms";

export async function sendDm(peerId: number, content: string, replyToId?: number | null) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;
  await _dms.sendDm(server, state, peerId, content, replyToId);
}

export async function editDm(messageId: number, newContent: string) {
  const state = activeState();
  if (!state) return;
  await _dms.editDm(state, messageId, newContent);
}

export function deleteDm(messageId: number) {
  const state = activeState();
  if (!state) return;
  _dms.deleteDm(state, messageId);
}

export function toggleDmReaction(messageId: number, emoji: string) {
  const state = activeState();
  if (!state) return;
  _dms.toggleDmReaction(state, messageId, emoji);
}

export async function loadOlderDms(peerId: number): Promise<number> {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return 0;
  return _dms.loadOlderDms(server, state, peerId);
}

export async function openDmWith(peerId: number) {
  const server = activeServer();
  const state = activeState();
  if (!server || !state) return;
  state.activeDmUserId = peerId;
  state.activeTab = "dms";
  state.dmUnread.delete(peerId);
  if (!state.dms.has(peerId)) {
    await _dms.loadConversation(server, state, peerId);
  }
}
